// Ported from ardia apps/browser/src/lib/pluginWhen.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `when` clauses of plugin contributions: a small expression language over the current tab
//! (`urlHost == "example.com" && !isPinned`), its parser, glob match and evaluator.
//!
//! Changes from Ardia:
//! - `WhenValue` and the values of `WhenContext` are `Json`, so a context from untyped data keeps
//!   the JS rules (`String(value)`, `NaN` is truthy). `WhenContext` is `Fields`.
//! - `WhenParse` is `Result<WhenExpr, String>`; the parser returns errors where the TS throws.
//! - The tokenizer and `globMatches` read UTF-16 code units, so positions in problems and the
//!   length limits count as in JS (an astral character is two units, and is reported as its
//!   first, a lone surrogate, escaped as `JSON.stringify` does).
//! - `whenHolds` has no parse cache (`parsed`): the cache changes speed only, not results.

use ria_core::identity::telemetry_wire::Json;

use super::decode::Fields;
use super::errors::js_string;
use super::js::{js_number, js_trim};

/// `WHEN_KEYS` (`pluginWhen.ts:1`).
pub const WHEN_KEYS: [&str; 7] = [
    "urlHost",
    "urlScheme",
    "isPinned",
    "isInGroup",
    "isSplit",
    "isActive",
    "tabCount",
];
/// `WhenKey` (`pluginWhen.ts:2`): one of [`WHEN_KEYS`].
pub type WhenKey = &'static str;
/// `WhenValue` (`pluginWhen.ts:3`): a string, number or boolean.
pub type WhenValue = Json;
/// `WhenContext` (`pluginWhen.ts:4`): values by [`WhenKey`].
pub type WhenContext = Fields;

/// `MAX_WHEN_LENGTH` (`pluginWhen.ts:6`).
pub const MAX_WHEN_LENGTH: usize = 200;
const MAX_DEPTH: usize = 16;
const MAX_GLOB: usize = 128;

/// `Operand` (`pluginWhen.ts:12`): `Literal | Key`.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Literal(WhenValue),
    Key(WhenKey),
}

/// `WhenExpr` (`pluginWhen.ts:13`).
#[derive(Debug, Clone, PartialEq)]
pub enum WhenExpr {
    Operand(Operand),
    Not(Box<WhenExpr>),
    And(Box<WhenExpr>, Box<WhenExpr>),
    Or(Box<WhenExpr>, Box<WhenExpr>),
    /// `op` is `==`, `!=` or `~=`.
    Compare {
        op: &'static str,
        left: Operand,
        right: Operand,
    },
}

/// `Token` (`pluginWhen.ts:19`).
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Op(&'static str),
    String(String),
    Number(f64),
    Word(String),
}

/// `WhenParse` (`pluginWhen.ts:21`): the expression, or the problem.
pub type WhenParse = Result<WhenExpr, String>;

const OPERATORS: [&str; 8] = ["&&", "||", "==", "!=", "~=", "!", "(", ")"];

fn is_digit(unit: Option<&u16>) -> bool {
    unit.is_some_and(|u| (u16::from(b'0')..=u16::from(b'9')).contains(u))
}

fn is_ascii(unit: u16, f: fn(&u8) -> bool) -> bool {
    u8::try_from(unit).is_ok_and(|b| f(&b))
}

/// `JSON.stringify` of one UTF-16 code unit.
fn quote_unit(unit: u16) -> String {
    match char::from_u32(u32::from(unit)) {
        Some(ch) => Json::String(ch.to_string()).stringify(),
        None => format!("\"\\u{unit:04x}\""),
    }
}

/// `tokenize` (`pluginWhen.ts:28`): the tokens, or the problem.
fn tokenize(source: &[u16]) -> Result<Vec<Token>, String> {
    let text = |from: usize, to: usize| String::from_utf16_lossy(&source[from..to]);
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < source.len() {
        let ch = source[i];
        if ch == u16::from(b' ') || ch == u16::from(b'\t') {
            i += 1;
            continue;
        }
        let op = OPERATORS.iter().find(|o| {
            let o = o.as_bytes();
            source[i..].len() >= o.len()
                && o.iter().zip(&source[i..]).all(|(a, b)| u16::from(*a) == *b)
        });
        if let Some(op) = op {
            tokens.push(Token::Op(op));
            i += op.len();
            continue;
        }
        if ch == u16::from(b'"') || ch == u16::from(b'\'') {
            let Some(end) = source[i + 1..]
                .iter()
                .position(|u| *u == ch)
                .map(|p| p + i + 1)
            else {
                return Err("has a string without its closing quote".to_owned());
            };
            tokens.push(Token::String(text(i + 1, end)));
            i = end + 1;
            continue;
        }
        if is_digit(Some(&ch)) || (ch == u16::from(b'-') && is_digit(source.get(i + 1))) {
            let mut end = i + 1;
            while end < source.len()
                && (is_digit(Some(&source[end])) || source[end] == u16::from(b'.'))
            {
                end += 1;
            }
            let value = js_number(&text(i, end));
            if !value.is_finite() {
                return Err(format!("has a bad number at {i}"));
            }
            tokens.push(Token::Number(value));
            i = end;
            continue;
        }
        if is_ascii(ch, u8::is_ascii_alphabetic) || ch == u16::from(b'_') {
            let mut end = i + 1;
            while end < source.len()
                && (is_ascii(source[end], u8::is_ascii_alphanumeric)
                    || source[end] == u16::from(b'_'))
            {
                end += 1;
            }
            tokens.push(Token::Word(text(i, end)));
            i = end;
            continue;
        }
        return Err(format!("has an unexpected {} at {i}", quote_unit(ch)));
    }
    Ok(tokens)
}

/// `Parser` (`pluginWhen.ts:71`).
struct Parser {
    at: usize,
    tokens: Vec<Token>,
}

impl Parser {
    fn parse(&mut self) -> WhenParse {
        let expr = self.or(0)?;
        if self.at < self.tokens.len() {
            return Err(format!(
                "has something unexpected after position {}",
                self.at
            ));
        }
        Ok(expr)
    }

    fn peek_op(&self, value: &str) -> bool {
        matches!(self.tokens.get(self.at), Some(Token::Op(op)) if *op == value)
    }

    fn or(&mut self, depth: usize) -> WhenParse {
        let mut left = self.and(depth)?;
        while self.peek_op("||") {
            self.at += 1;
            left = WhenExpr::Or(Box::new(left), Box::new(self.and(depth)?));
        }
        Ok(left)
    }

    fn and(&mut self, depth: usize) -> WhenParse {
        let mut left = self.unary(depth)?;
        while self.peek_op("&&") {
            self.at += 1;
            left = WhenExpr::And(Box::new(left), Box::new(self.unary(depth)?));
        }
        Ok(left)
    }

    fn unary(&mut self, depth: usize) -> WhenParse {
        if depth > MAX_DEPTH {
            return Err(format!("nests deeper than {MAX_DEPTH}"));
        }
        if self.peek_op("!") {
            self.at += 1;
            return Ok(WhenExpr::Not(Box::new(self.unary(depth + 1)?)));
        }
        if self.peek_op("(") {
            self.at += 1;
            let inner = self.or(depth + 1)?;
            if !self.peek_op(")") {
                return Err("has a ( without its )".to_owned());
            }
            self.at += 1;
            return Ok(inner);
        }
        let left = self.operand()?;
        if let Some(&Token::Op(op @ ("==" | "!=" | "~="))) = self.tokens.get(self.at) {
            self.at += 1;
            return Ok(WhenExpr::Compare {
                op,
                left,
                right: self.operand()?,
            });
        }
        Ok(WhenExpr::Operand(left))
    }

    fn operand(&mut self) -> Result<Operand, String> {
        let t = self.tokens.get(self.at).cloned();
        self.at += 1;
        match t {
            None => Err("ends too soon".to_owned()),
            Some(Token::String(s)) => Ok(Operand::Literal(Json::String(s))),
            Some(Token::Number(n)) => Ok(Operand::Literal(Json::Number(n))),
            Some(Token::Word(w)) => match w.as_str() {
                "true" | "false" => Ok(Operand::Literal(Json::Bool(w == "true"))),
                _ => match WHEN_KEYS.iter().find(|k| **k == w) {
                    Some(key) => Ok(Operand::Key(key)),
                    None => Err(format!(
                        "names {w}, which isn't one of {}",
                        WHEN_KEYS.join(", ")
                    )),
                },
            },
            Some(Token::Op(op)) => Err(format!("has {op} where a key or value goes")),
        }
    }
}

/// `parseWhen` (`pluginWhen.ts:142`).
pub fn parse_when(source: &str) -> WhenParse {
    if js_trim(source).is_empty() {
        return Err("is empty".to_owned());
    }
    let units: Vec<u16> = source.encode_utf16().collect();
    if units.len() > MAX_WHEN_LENGTH {
        return Err(format!("is over {MAX_WHEN_LENGTH} characters"));
    }
    let tokens = tokenize(&units)?;
    Parser { at: 0, tokens }.parse()
}

/// `globMatches` (`pluginWhen.ts:154`): `*` matches any run of UTF-16 code units.
pub fn glob_matches(pattern: &str, value: &str) -> bool {
    let pattern: Vec<u16> = pattern.encode_utf16().collect();
    let value: Vec<u16> = value.encode_utf16().collect();
    if pattern.len() > MAX_GLOB || value.len() > 2048 {
        return false;
    }
    let star_unit = u16::from(b'*');
    let (mut p, mut v, mut star, mut mark) = (0, 0, None, 0);
    while v < value.len() {
        if p < pattern.len() && pattern[p] != star_unit && pattern[p] == value[v] {
            p += 1;
            v += 1;
        } else if p < pattern.len() && pattern[p] == star_unit {
            star = Some(p);
            p += 1;
            mark = v;
        } else if let Some(s) = star {
            p = s + 1;
            mark += 1;
            v = mark;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == star_unit {
        p += 1;
    }
    p == pattern.len()
}

/// `valueOf` (`pluginWhen.ts:176`): `None` is `undefined`.
fn value_of<'a>(o: &'a Operand, context: &'a WhenContext) -> Option<&'a WhenValue> {
    match o {
        Operand::Literal(value) => Some(value),
        Operand::Key(key) => context.iter().find(|(k, _)| k == key).map(|(_, v)| v),
    }
}

/// `truthy` (`pluginWhen.ts:177`).
fn truthy(v: Option<&WhenValue>) -> bool {
    match v {
        Some(Json::String(s)) => !s.is_empty(),
        Some(Json::Number(n)) => *n != 0.0,
        Some(v) => *v == Json::Bool(true),
        None => false,
    }
}

/// `String(v ?? "")`.
fn text_of(v: Option<&WhenValue>) -> String {
    match v {
        None | Some(Json::Null) => String::new(),
        Some(v) => js_string(v),
    }
}

/// `same` (`pluginWhen.ts:179`).
fn same(a: Option<&WhenValue>, b: Option<&WhenValue>) -> bool {
    if let (Some(Json::Number(a)), Some(Json::Number(b))) = (a, b) {
        return a == b;
    }
    text_of(a) == text_of(b)
}

/// `evaluateWhen` (`pluginWhen.ts:184`).
pub fn evaluate_when(expr: &WhenExpr, context: &WhenContext) -> bool {
    match expr {
        WhenExpr::Operand(o) => truthy(value_of(o, context)),
        WhenExpr::Not(of) => !evaluate_when(of, context),
        WhenExpr::And(left, right) => evaluate_when(left, context) && evaluate_when(right, context),
        WhenExpr::Or(left, right) => evaluate_when(left, context) || evaluate_when(right, context),
        WhenExpr::Compare { op, left, right } => {
            let left = value_of(left, context);
            let right = value_of(right, context);
            match *op {
                "==" => same(left, right),
                "!=" => !same(left, right),
                _ => glob_matches(
                    &text_of(right).to_lowercase(),
                    &text_of(left).to_lowercase(),
                ),
            }
        }
    }
}

/// `whenHolds` (`pluginWhen.ts:207`): an empty or missing clause holds.
pub fn when_holds(source: Option<&str>, context: &WhenContext) -> bool {
    match source {
        None | Some("") => true,
        Some(source) => parse_when(source).is_ok_and(|expr| evaluate_when(&expr, context)),
    }
}
