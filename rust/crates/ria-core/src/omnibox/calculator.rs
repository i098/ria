// Ported from ardia packages/core/src/calculator.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The omnibox calculator: arithmetic, functions, and unit and temperature conversion.
//!
//! The manual matchers preserve JavaScript backtracking order and the JavaScript `\s` set.
//! They also preserve ASCII-only `\w`, `\d`, and case folding.
//!
//! shortcut: the source gives `None` when the JS stack overflows, at a depth that depends on the
//! engine. Here the parser counts the frames the source would use and gives `None` past
//! `MAX_FRAMES`; V8 took 1500 nested parentheses (9000 frames) and failed at 3000 (18000), so the
//! real limit lies between. Upgrade only if a caller needs the same limit as V8.
//!
//! Changes from Ardia: `tokenize`, `Parser::term`, `js_exp`, and `call` are split into helpers
//! (`call` into a table) for the complexity gate, with unchanged behavior.

use std::f64::consts::{E, PI};
use std::sync::LazyLock;

use crate::fuzzy::is_js_space;

const MAX_FRAMES: usize = 12_000;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Op(char),
    Id(String),
}

/// `\d` of a non-unicode JS regex.
fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// `π` under the `i` flag: the characters whose JS upper case is `Π`.
fn is_pi(c: char) -> bool {
    matches!(c, 'π' | 'Π' | 'ϖ')
}

/// `τ` under the `i` flag.
fn is_tau(c: char) -> bool {
    matches!(c, 'τ' | 'Τ')
}

/// `\w` of a non-unicode JS regex.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `.` of a JS regex without the `s` flag.
fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn skip_space(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && is_js_space(chars[i]) {
        i += 1;
    }
    i
}

fn skip_digits(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && is_digit(chars[i]) {
        i += 1;
    }
    i
}

/// The end of `/^(\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?/i` at `i`.
fn number_end(chars: &[char], i: usize) -> Option<usize> {
    let mut end = skip_digits(chars, i);
    if end > i {
        if chars.get(end) == Some(&'.') {
            end = skip_digits(chars, end + 1);
        }
    } else if chars.get(i) == Some(&'.') {
        end = skip_digits(chars, i + 1);
        if end == i + 1 {
            return None;
        }
    } else {
        return None;
    }
    if matches!(chars.get(end), Some('e' | 'E')) {
        let mut at = end + 1;
        if matches!(chars.get(at), Some('+' | '-')) {
            at += 1;
        }
        let digits_end = skip_digits(chars, at);
        if digits_end > at {
            end = digits_end;
        }
    }
    Some(end)
}

/// The `**` or single-character operator token at `i`, with its end.
fn operator_token(chars: &[char], i: usize) -> Option<(Token, usize)> {
    let c = chars[i];
    if c == '*' && chars.get(i + 1) == Some(&'*') {
        return Some((Token::Op('^'), i + 2));
    }
    if !"-+*/^%!(),×·÷−–°".contains(c) {
        return None;
    }
    let op = match c {
        '×' | '·' => '*',
        '÷' => '/',
        '−' | '–' => '-',
        other => other,
    };
    Some((Token::Op(op), i + 1))
}

/// The identifier token (ASCII word, `π` or `τ`) at `i`, with its end.
fn identifier_token(chars: &[char], i: usize) -> Option<(Token, usize)> {
    let c = chars[i];
    if c.is_ascii_alphabetic() {
        let mut end = i + 1;
        while end < chars.len() && chars[end].is_ascii_alphanumeric() {
            end += 1;
        }
        let id = chars[i..end].iter().collect::<String>().to_lowercase();
        return Some((Token::Id(id), end));
    }
    if is_pi(c) || is_tau(c) {
        return Some((Token::Id(c.to_lowercase().collect()), i + 1));
    }
    None
}

fn tokenize(source: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if is_js_space(chars[i]) {
            i = skip_space(&chars, i);
        } else if let Some(end) = number_end(&chars, i) {
            let text: String = chars[i..end].iter().collect();
            tokens.push(Token::Num(text.parse().ok()?));
            i = end;
        } else {
            let (token, end) = operator_token(&chars, i).or_else(|| identifier_token(&chars, i))?;
            tokens.push(token);
            i = end;
        }
    }
    Some(tokens)
}

/// `key in CONSTANTS`, with its value. `constructor` is a key too, through `Object.prototype`:
/// its value is a function, which every JS arithmetic turns to NaN, as here.
fn constant(name: &str) -> Option<f64> {
    match name {
        "pi" | "π" => Some(PI),
        "e" => Some(E),
        "tau" | "τ" => Some(2.0 * PI),
        "constructor" => Some(f64::NAN),
        _ => None,
    }
}

const FUNCTION_NAMES: [&str; 23] = [
    "sqrt", "cbrt", "abs", "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh",
    "ln", "log", "log2", "log10", "exp", "floor", "ceil", "round", "min", "max", "pow",
];

fn is_function(name: &str) -> bool {
    FUNCTION_NAMES.contains(&name)
}

/// `Math.pow`: the cases where it differs from `f64::powf` are NaN exponents and `±1` to an infinity.
fn js_pow(x: f64, y: f64) -> f64 {
    if y.is_nan() || (x.abs() == 1.0 && y.is_infinite()) {
        f64::NAN
    } else {
        x.powf(y)
    }
}

/// `Math.round`: halves go toward `+Infinity`, and a negative `x` that rounds to zero gives `-0`.
fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let floor = x.floor();
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 && x < 0.0 {
        -0.0
    } else {
        rounded
    }
}

/// The `Math.exp` result for a large `|x|` (`hx >= 0x40862e42`), where fdlibm returns early.
#[expect(clippy::excessive_precision, reason = "fdlibm constants as published")]
fn exp_overflow(x: f64, hx: u32, negative: bool) -> Option<f64> {
    if hx < 0x4086_2e42 {
        return None;
    }
    if x.is_nan() {
        return Some(x);
    }
    if x.is_infinite() {
        return Some(if negative { 0.0 } else { x });
    }
    if x > 7.097_827_128_933_839_730_96e+02 {
        return Some(f64::INFINITY);
    }
    if x < -7.451_332_191_019_411_084_20e+02 {
        return Some(0.0);
    }
    None
}

/// `y * 2^k` as fdlibm scales the `Math.exp` result.
#[expect(clippy::excessive_precision, reason = "fdlibm constants as published")]
fn exp_scale(y: f64, k: i32) -> f64 {
    let two_pow =
        |k: i32| f64::from_bits(u64::from(0x3ff0_0000_i32.wrapping_add(k << 20) as u32) << 32);
    if k >= -1021 {
        if k == 1024 {
            y * 2.0 * 8.988_465_674_311_579_539e307
        } else {
            y * two_pow(k)
        }
    } else {
        y * two_pow(k + 1000) * 9.332_636_185_032_188_789_90e-302
    }
}

/// `Math.exp` as V8 computes it (fdlibm `e_exp.c`): glibc's `exp` differs from it in the last bit.
#[expect(
    clippy::excessive_precision,
    clippy::approx_constant,
    reason = "fdlibm constants as published"
)]
fn js_exp(x: f64) -> f64 {
    const LN2_HI: f64 = 6.931_471_803_691_238_164_90e-01;
    const LN2_LO: f64 = 1.908_214_929_270_587_700_02e-10;
    const INV_LN2: f64 = 1.442_695_040_888_963_387_00e+00;
    const P: [f64; 5] = [
        1.666_666_666_666_660_190_37e-01,
        -2.777_777_777_701_559_338_42e-03,
        6.613_756_321_437_934_361_17e-05,
        -1.653_390_220_546_525_153_90e-06,
        4.138_136_797_057_238_460_39e-08,
    ];
    let bits = x.to_bits();
    let negative = bits >> 63 == 1;
    let hx = ((bits >> 32) & 0x7fff_ffff) as u32;
    if let Some(out) = exp_overflow(x, hx, negative) {
        return out;
    }
    let (k, hi, lo, x) = if hx > 0x3fd6_2e42 {
        let (k, hi, lo) = if hx < 0x3ff0_a2b2 {
            if x == 1.0 {
                return std::f64::consts::E;
            }
            if negative {
                (-1, x + LN2_HI, -LN2_LO)
            } else {
                (1, x - LN2_HI, LN2_LO)
            }
        } else {
            let k = (INV_LN2 * x + if negative { -0.5 } else { 0.5 }) as i32;
            let t = f64::from(k);
            (k, x - t * LN2_HI, t * LN2_LO)
        };
        (k, hi, lo, hi - lo)
    } else if hx < 0x3e30_0000 {
        return 1.0 + x;
    } else {
        (0, 0.0, 0.0, x)
    };
    let t = x * x;
    let c = x - t * (P[0] + t * (P[1] + t * (P[2] + t * (P[3] + t * P[4]))));
    if k == 0 {
        return 1.0 - ((x * c) / (c - 2.0) - x);
    }
    exp_scale(1.0 - ((lo - (x * c) / (2.0 - c)) - hi), k)
}

fn js_extreme(args: &[f64], want_min: bool) -> f64 {
    let mut out = if want_min {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for &a in args {
        if a.is_nan() {
            return f64::NAN;
        }
        let signed_zero = a == 0.0 && out == 0.0 && a.is_sign_negative() == want_min;
        if (want_min && a < out) || (!want_min && a > out) || signed_zero {
            out = a;
        }
    }
    out
}

/// The functions of one argument, by name.
type UnaryFunction = (&'static str, fn(f64) -> f64);
const UNARY_FUNCTIONS: [UnaryFunction; 19] = [
    ("sqrt", f64::sqrt),
    ("cbrt", f64::cbrt),
    ("abs", f64::abs),
    ("sin", f64::sin),
    ("cos", f64::cos),
    ("tan", f64::tan),
    ("asin", f64::asin),
    ("acos", f64::acos),
    ("atan", f64::atan),
    ("sinh", f64::sinh),
    ("cosh", f64::cosh),
    ("tanh", f64::tanh),
    ("ln", f64::ln),
    ("log2", f64::log2),
    ("log10", f64::log10),
    ("exp", js_exp),
    ("floor", f64::floor),
    ("ceil", f64::ceil),
    ("round", js_round),
];

fn call(name: &str, args: &[f64]) -> f64 {
    let x = args.first().copied().unwrap_or(f64::NAN);
    match name {
        "log" => match args.get(1) {
            None => x.log10(),
            Some(base) => x.ln() / base.ln(),
        },
        "min" => js_extreme(args, true),
        "max" => js_extreme(args, false),
        "pow" => js_pow(x, args.get(1).copied().unwrap_or(f64::NAN)),
        _ => UNARY_FUNCTIONS
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(f64::NAN, |(_, f)| f(x)),
    }
}

fn factorial(n: f64) -> f64 {
    if n.fract() != 0.0 || !(0.0..=170.0).contains(&n) {
        return f64::NAN;
    }
    let mut out = 1.0;
    let mut k = 2.0;
    while k <= n {
        out *= k;
        k += 1.0;
    }
    out
}

struct Parser {
    tokens: Vec<Token>,
    i: usize,
    depth: usize,
}

/// A syntax error, or nesting past `MAX_DEPTH`.
struct Failed;

type Parsed = Result<f64, Failed>;

impl Parser {
    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.i + offset)
    }

    fn is_op(&self, value: char, offset: usize) -> bool {
        self.peek(offset) == Some(&Token::Op(value))
    }

    fn is_id(&self, value: &str, offset: usize) -> bool {
        matches!(self.peek(offset), Some(Token::Id(id)) if id == value)
    }

    fn starts_primary(&self, offset: usize) -> bool {
        match self.peek(offset) {
            Some(Token::Num(_)) => true,
            Some(Token::Id(id)) => constant(id).is_some() || is_function(id),
            Some(Token::Op(op)) => *op == '(',
            None => false,
        }
    }

    /// Counts `frames` more JS stack frames.
    fn enter(&mut self, frames: usize) -> Result<(), Failed> {
        self.depth += frames;
        if self.depth > MAX_FRAMES {
            Err(Failed)
        } else {
            Ok(())
        }
    }

    fn expr(&mut self) -> Parsed {
        self.enter(2)?; // expr and term
        let mut value = self.term()?;
        while self.is_op('+', 0) || self.is_op('-', 0) {
            let plus = self.is_op('+', 0);
            self.i += 1;
            let rhs = self.term()?;
            value = if plus { value + rhs } else { value - rhs };
        }
        self.depth -= 2;
        Ok(value)
    }

    /// `*`, `/`, `mod`, `of`, or `x` at the cursor.
    fn at_multiplicative(&self) -> bool {
        self.is_op('*', 0)
            || self.is_op('/', 0)
            || self.is_id("mod", 0)
            || self.is_id("of", 0)
            || self.is_id("x", 0)
    }

    fn term(&mut self) -> Parsed {
        let mut value = self.unary()?;
        loop {
            if self.at_multiplicative() {
                let divide = self.is_op('/', 0);
                let modulo = self.is_id("mod", 0);
                self.i += 1;
                let rhs = self.unary()?;
                if divide {
                    value /= rhs;
                } else if modulo {
                    value %= rhs;
                } else {
                    value *= rhs;
                }
            } else if self.is_op('%', 0) && self.starts_primary(1) {
                self.i += 1;
                value %= self.unary()?;
            } else if !matches!(self.peek(0), Some(Token::Num(_))) && self.starts_primary(0) {
                value *= self.unary()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn unary(&mut self) -> Parsed {
        self.enter(1)?;
        let value = if self.is_op('-', 0) {
            self.i += 1;
            -self.unary()?
        } else if self.is_op('+', 0) {
            self.i += 1;
            self.unary()?
        } else {
            self.power()?
        };
        self.depth -= 1;
        Ok(value)
    }

    fn power(&mut self) -> Parsed {
        self.enter(1)?;
        self.enter(2)?; // postfix and primary
        let base = self.postfix()?;
        self.depth -= 2;
        let value = if self.is_op('^', 0) {
            self.i += 1;
            js_pow(base, self.unary()?)
        } else {
            base
        };
        self.depth -= 1;
        Ok(value)
    }

    fn postfix(&mut self) -> Parsed {
        let mut value = self.primary()?;
        loop {
            if self.is_op('!', 0) {
                self.i += 1;
                value = factorial(value);
            } else if self.is_op('%', 0) && !self.starts_primary(1) {
                self.i += 1;
                value /= 100.0;
            } else if self.is_op('°', 0) || self.is_id("deg", 0) || self.is_id("degrees", 0) {
                self.i += 1;
                value *= PI / 180.0;
            } else {
                return Ok(value);
            }
        }
    }

    fn primary(&mut self) -> Parsed {
        let token = self.tokens.get(self.i).cloned().ok_or(Failed)?;
        self.i += 1;
        match token {
            Token::Num(value) => Ok(value),
            Token::Op('(') => {
                let value = self.expr()?;
                self.expect(')')?;
                Ok(value)
            }
            Token::Id(id) => {
                if let Some(value) = constant(&id) {
                    return Ok(value);
                }
                if !is_function(&id) {
                    return Err(Failed);
                }
                self.expect('(')?;
                let mut args = vec![self.expr()?];
                while self.is_op(',', 0) {
                    self.i += 1;
                    args.push(self.expr()?);
                }
                self.expect(')')?;
                Ok(call(&id, &args))
            }
            Token::Op(_) => Err(Failed),
        }
    }

    fn expect(&mut self, op: char) -> Result<(), Failed> {
        if !self.is_op(op, 0) {
            return Err(Failed);
        }
        self.i += 1;
        Ok(())
    }
}

/// `/[a-z]\s*\(/i.test(text)`.
fn has_call_paren(chars: &[char]) -> bool {
    chars
        .iter()
        .enumerate()
        .any(|(i, c)| c.is_ascii_alphabetic() && chars.get(skip_space(chars, i + 1)) == Some(&'('))
}

/// `text.replace(/(\d),(?=\d{3}(?!\d))/g, "$1")`.
fn strip_thousands(chars: &[char]) -> String {
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let group = i + 5 <= chars.len()
            && is_digit(chars[i])
            && chars[i + 1] == ','
            && chars[i + 2..i + 5].iter().all(|&c| is_digit(c))
            && !chars.get(i + 5).is_some_and(|&c| is_digit(c));
        out.push(chars[i]);
        i += if group { 2 } else { 1 };
    }
    out
}

/// Port of `evaluate` (`packages/core/src/calculator.ts:203`).
pub fn evaluate(expression: &str) -> Option<f64> {
    let chars: Vec<char> = expression.chars().collect();
    let stripped;
    let source = if has_call_paren(&chars) {
        expression
    } else {
        stripped = strip_thousands(&chars);
        &stripped
    };
    let tokens = tokenize(source)?;
    if tokens.is_empty() {
        return None;
    }
    let mut parser = Parser {
        tokens,
        i: 0,
        depth: 0,
    };
    let value = parser.expr().ok()?;
    (parser.i == parser.tokens.len() && value.is_finite()).then_some(value)
}

/// The digits (no dot) of `value` to `digits` significant digits, as `toPrecision` rounds them
/// (an exact tie goes up), then back to a number: `Number(value.toPrecision(digits))`.
fn round_significant(value: f64, digits: usize) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    // 30 digits are exact enough: a double has at most 17 significant digits that matter, so a
    // run of 13+ nines or zeros past `digits` cannot be an artefact of this print.
    let text = format!("{:.29e}", value.abs());
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let all: Vec<u8> = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .collect();
    let mut n = all[..digits]
        .iter()
        .fold(0u64, |acc, &d| acc * 10 + u64::from(d));
    if all[digits] >= 5 {
        n += 1;
    }
    let exp10: i32 = exponent.parse().unwrap_or(0) - (digits as i32 - 1);
    let rounded: f64 = format!("{n}e{exp10}").parse().unwrap_or(f64::NAN);
    if value < 0.0 { -rounded } else { rounded }
}

/// `String(value)` and `value.toExponential()` of a finite non-zero number: the shortest digits
/// that read back as `value`, and the decimal exponent of the first digit.
fn shortest_digits(value: f64) -> (String, i32) {
    let text = format!("{:e}", value.abs());
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    (mantissa.replace('.', ""), exponent.parse().unwrap_or(0))
}

fn js_exponential(value: f64) -> String {
    let (digits, exp) = shortest_digits(value);
    let sign = if value < 0.0 { "-" } else { "" };
    let (first, rest) = digits.split_at(1);
    let dot = if rest.is_empty() { "" } else { "." };
    format!(
        "{sign}{first}{dot}{rest}e{}{}",
        if exp < 0 { '-' } else { '+' },
        exp.abs()
    )
}

/// `String(value)` for a finite number.
fn js_to_string(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    let (digits, exp) = shortest_digits(value);
    let sign = if value < 0.0 { "-" } else { "" };
    let k = digits.len() as i32;
    let n = exp + 1; // the decimal point sits after n digits
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        return js_exponential(value);
    };
    format!("{sign}{body}")
}

/// Port of `formatNumber` (`packages/core/src/calculator.ts:216`).
fn format_number(value: f64, digits: usize) -> String {
    if value == 0.0 {
        return "0".into();
    }
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let rounded = round_significant(value, digits);
    let abs = rounded.abs();
    // The source also runs `.replace(/\.?0+e/, "e")` here; the shortest digits never end in 0.
    if !(1e-9..1e21).contains(&abs) {
        js_exponential(rounded)
    } else {
        js_to_string(rounded)
    }
}

/// A unit of a conversion: its dimension, size in the base unit, and the label printed.
struct Unit {
    dim: &'static str,
    factor: f64,
    label: &'static str,
}

/// `units(dim, factor, label, ...names)`: the table in the source's order. A later row with the
/// same lowercase name replaces the earlier one in place, as a JS object does.
fn unit_table() -> &'static [(String, Unit)] {
    static TABLE: LazyLock<Vec<(String, Unit)>> = LazyLock::new(|| {
        const ROWS: &[(&str, f64, &str, &[&str])] = &[
            ("length", 1.0, "m", &["meter", "meters", "metre", "metres"]),
            (
                "length",
                1000.0,
                "km",
                &["kilometer", "kilometers", "kilometre", "kilometres"],
            ),
            ("length", 0.01, "cm", &["centimeter", "centimeters"]),
            ("length", 0.001, "mm", &["millimeter", "millimeters"]),
            ("length", 1609.344, "mi", &["mile", "miles"]),
            ("length", 0.9144, "yd", &["yard", "yards"]),
            ("length", 0.3048, "ft", &["foot", "feet"]),
            ("length", 0.0254, "in", &["inch", "inches"]),
            ("length", 1852.0, "nmi", &["nautical mile"]),
            (
                "mass",
                1.0,
                "kg",
                &["kilogram", "kilograms", "kilo", "kilos"],
            ),
            ("mass", 0.001, "g", &["gram", "grams"]),
            ("mass", 1e-6, "mg", &["milligram", "milligrams"]),
            ("mass", 0.45359237, "lb", &["lbs", "pound", "pounds"]),
            ("mass", 0.028349523125, "oz", &["ounce", "ounces"]),
            ("mass", 6.35029318, "st", &["stone", "stones"]),
            ("mass", 1000.0, "t", &["tonne", "tonnes"]),
            ("volume", 1.0, "l", &["liter", "liters", "litre", "litres"]),
            (
                "volume",
                0.001,
                "ml",
                &["milliliter", "milliliters", "millilitre", "millilitres"],
            ),
            ("volume", 3.785411784, "gal", &["gallon", "gallons"]),
            ("volume", 0.946352946, "qt", &["quart", "quarts"]),
            ("volume", 0.473176473, "pt", &["pint", "pints"]),
            ("volume", 0.2365882365, "cup", &["cups"]),
            ("volume", 0.0295735295625, "fl oz", &["floz"]),
            (
                "volume",
                0.01478676478125,
                "tbsp",
                &["tablespoon", "tablespoons"],
            ),
            (
                "volume",
                0.00492892159375,
                "tsp",
                &["teaspoon", "teaspoons"],
            ),
            ("time", 0.001, "ms", &["millisecond", "milliseconds"]),
            ("time", 1.0, "s", &["sec", "secs", "second", "seconds"]),
            ("time", 60.0, "min", &["mins", "minute", "minutes"]),
            ("time", 3600.0, "h", &["hr", "hrs", "hour", "hours"]),
            ("time", 86400.0, "days", &["day", "d"]),
            ("time", 604800.0, "weeks", &["week", "wk"]),
            ("time", 31556952.0, "years", &["year", "yr"]),
            ("data", 0.125, "bits", &["bit"]),
            ("data", 1.0, "bytes", &["byte", "B"]),
            ("data", 1e3, "KB", &["kilobyte", "kilobytes"]),
            ("data", 1e6, "MB", &["megabyte", "megabytes"]),
            ("data", 1e9, "GB", &["gigabyte", "gigabytes"]),
            ("data", 1e12, "TB", &["terabyte", "terabytes"]),
            ("data", 1024.0, "KiB", &[]),
            ("data", 1048576.0, "MiB", &[]),
            ("data", 1073741824.0, "GiB", &[]),
            ("data", 1099511627776.0, "TiB", &[]),
            ("speed", 1.0, "m/s", &[]),
            ("speed", 1000.0 / 3600.0, "km/h", &["kph", "kmh"]),
            ("speed", 1609.344 / 3600.0, "mph", &[]),
            ("speed", 1852.0 / 3600.0, "knots", &["knot", "kn"]),
        ];
        let mut table: Vec<(String, Unit)> = Vec::new();
        for &(dim, factor, label, names) in ROWS {
            for name in std::iter::once(label).chain(names.iter().copied()) {
                let key = name.to_lowercase();
                let unit = Unit { dim, factor, label };
                match table.iter_mut().find(|(k, _)| *k == key) {
                    Some(slot) => slot.1 = unit,
                    None => table.push((key, unit)),
                }
            }
        }
        table
    });
    &TABLE
}

const TEMPERATURES: [(&str, char); 8] = [
    ("c", 'C'),
    ("°c", 'C'),
    ("celsius", 'C'),
    ("f", 'F'),
    ("°f", 'F'),
    ("fahrenheit", 'F'),
    ("k", 'K'),
    ("kelvin", 'K'),
];

fn temperature(key: &str) -> Option<char> {
    TEMPERATURES
        .iter()
        .find(|(k, _)| *k == key)
        .map(|&(_, scale)| scale)
}

fn to_celsius(scale: char, v: f64) -> f64 {
    match scale {
        'F' => (v - 32.0) * 5.0 / 9.0,
        'K' => v - 273.15,
        _ => v,
    }
}

fn from_celsius(scale: char, v: f64) -> f64 {
    match scale {
        'F' => v * 9.0 / 5.0 + 32.0,
        'K' => v + 273.15,
        _ => v,
    }
}

/// The alternatives of `unitPattern`: each key, longest first (stable), as its words; a space in
/// a key is `\s*` in the pattern.
fn unit_alternatives() -> &'static [Vec<Vec<char>>] {
    static ALTS: LazyLock<Vec<Vec<Vec<char>>>> = LazyLock::new(|| {
        let mut keys: Vec<&str> = unit_table().iter().map(|(k, _)| k.as_str()).collect();
        keys.extend(TEMPERATURES.iter().map(|&(k, _)| k));
        keys.sort_by_key(|k| std::cmp::Reverse(k.chars().count()));
        keys.iter()
            .map(|k| k.split(' ').map(|w| w.chars().collect()).collect())
            .collect()
    });
    &ALTS
}

/// `text` at `at`, ASCII case-insensitive (the `i` flag).
fn matches_at(chars: &[char], at: usize, text: &[char]) -> bool {
    at + text.len() <= chars.len()
        && chars[at..at + text.len()]
            .iter()
            .zip(text)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// The end of each alternative of `(unitPattern)` that matches at `at`, in pattern order.
fn unit_ends(chars: &[char], at: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    'alt: for words in unit_alternatives() {
        let mut pos = at;
        for (n, word) in words.iter().enumerate() {
            if n > 0 {
                pos = skip_space(chars, pos);
            }
            if !matches_at(chars, pos, word) {
                continue 'alt;
            }
            pos += word.len();
        }
        ends.push(pos);
    }
    ends
}

/// `from` and `to` of `/^(.+?)\s*(unit)\s+(?:to|in|into|as|=|->|→)\s+(unit)$/i`, and the end of
/// the amount, for the first way the backtracking finds.
fn match_conversion(chars: &[char]) -> Option<(usize, String, String)> {
    const SEPARATORS: [&str; 7] = ["to", "in", "into", "as", "=", "->", "→"];
    let normalize = |from: usize, to: usize| -> String {
        let mut out = String::new();
        let mut in_space = false;
        for &c in &chars[from..to] {
            if is_js_space(c) {
                if !in_space {
                    out.push(' ');
                }
                in_space = true;
            } else {
                out.extend(c.to_lowercase());
                in_space = false;
            }
        }
        out
    };
    for amount_end in 1..=chars.len() {
        if is_line_terminator(chars[amount_end - 1]) {
            return None;
        }
        let from_start = skip_space(chars, amount_end);
        for from_end in unit_ends(chars, from_start) {
            let sep_start = skip_space(chars, from_end);
            if sep_start == from_end {
                continue;
            }
            for sep in SEPARATORS {
                let sep: Vec<char> = sep.chars().collect();
                if !matches_at(chars, sep_start, &sep) {
                    continue;
                }
                let to_start = skip_space(chars, sep_start + sep.len());
                if to_start == sep_start + sep.len() {
                    continue;
                }
                if let Some(to_end) = unit_ends(chars, to_start)
                    .into_iter()
                    .find(|&e| e == chars.len())
                {
                    return Some((
                        amount_end,
                        normalize(from_start, from_end),
                        normalize(to_start, to_end),
                    ));
                }
            }
        }
    }
    None
}

/// Port of `convert` (`packages/core/src/calculator.ts:259`): the value and the label of the target unit.
fn convert(input: &str) -> Option<(f64, String)> {
    let chars: Vec<char> = input.chars().collect();
    let (amount_end, from, to) = match_conversion(&chars)?;
    let amount = evaluate(&chars[..amount_end].iter().collect::<String>())?;
    if let (Some(a), Some(b)) = (temperature(&from), temperature(&to)) {
        let unit = if b == 'K' {
            "K".to_string()
        } else {
            format!("°{b}")
        };
        return Some((from_celsius(b, to_celsius(a, amount)), unit));
    }
    let table = unit_table();
    let find = |key: &str| table.iter().find(|(k, _)| k == key).map(|(_, unit)| unit);
    let (a, b) = (find(&from)?, find(&to)?);
    (a.dim == b.dim).then(|| ((amount * a.factor) / b.factor, b.label.to_string()))
}

/// An expression, its value and the text shown for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Calculation {
    pub expression: String,
    pub value: f64,
    pub display: String,
}

/// `/[\dπτ]|\b(pi|e|tau)\b/i.test(text)`.
fn has_number_or_constant(chars: &[char]) -> bool {
    if chars.iter().any(|&c| is_digit(c) || is_pi(c) || is_tau(c)) {
        return true;
    }
    (0..chars.len()).any(|i| {
        (i == 0 || !is_word(chars[i - 1]))
            && ["pi", "e", "tau"].iter().any(|name| {
                let name: Vec<char> = name.chars().collect();
                matches_at(chars, i, &name)
                    && !chars.get(i + name.len()).is_some_and(|&c| is_word(c))
            })
    })
}

fn is_date_like(chars: &[char]) -> bool {
    let digits = |from: usize, min: usize, max: usize| {
        let end = skip_digits(chars, from);
        (min..=max).contains(&(end - from)).then_some(end)
    };
    // `^\d{4}-\d{1,2}-\d{1,2}$`
    let date = digits(0, 4, 4)
        .filter(|&i| chars.get(i) == Some(&'-'))
        .and_then(|i| digits(i + 1, 1, 2))
        .filter(|&i| chars.get(i) == Some(&'-'))
        .and_then(|i| digits(i + 1, 1, 2))
        .is_some_and(|i| i == chars.len());
    // `^\d+(\.\d+){2,}$`
    let mut i = skip_digits(chars, 0);
    let mut groups = 0;
    while i > 0 && chars.get(i) == Some(&'.') && skip_digits(chars, i + 1) > i + 1 {
        i = skip_digits(chars, i + 1);
        groups += 1;
    }
    date || (i > 0 && groups >= 2 && i == chars.len())
}

/// `/[\w).πτ]\s*(\*\*|[-+*/^%×·÷−–]|mod|of|x)\s*[\d(.a-zπτ]/i.test(text)`.
fn has_operator_between(chars: &[char]) -> bool {
    let before = |c: char| is_word(c) || matches!(c, ')' | '.') || is_pi(c) || is_tau(c);
    let after = |c: char| {
        is_digit(c) || c == '(' || c == '.' || c.is_ascii_alphabetic() || is_pi(c) || is_tau(c)
    };
    let words: [Vec<char>; 5] = ["**", "mod", "of", "x", ""].map(|w| w.chars().collect());
    (0..chars.len()).filter(|&i| before(chars[i])).any(|i| {
        let at = skip_space(chars, i + 1);
        let mut ends: Vec<usize> = words[..4]
            .iter()
            .filter(|w| matches_at(chars, at, w))
            .map(|w| at + w.len())
            .collect();
        if chars.get(at).is_some_and(|&c| "-+*/^%×·÷−–".contains(c)) {
            ends.push(at + 1);
        }
        ends.into_iter()
            .any(|end| chars.get(skip_space(chars, end)).is_some_and(|&c| after(c)))
    })
}

/// `/\b(sqrt|cbrt|...)\s*\(/i.test(text)`.
fn has_function_call(chars: &[char]) -> bool {
    (0..chars.len())
        .filter(|&i| i == 0 || !is_word(chars[i - 1]))
        .any(|i| {
            FUNCTION_NAMES.iter().any(|name| {
                let name: Vec<char> = name.chars().collect();
                matches_at(chars, i, &name)
                    && chars.get(skip_space(chars, i + name.len())) == Some(&'(')
            })
        })
}

/// Port of `calculate` (`packages/core/src/calculator.ts:306`).
pub fn calculate(raw: &str) -> Option<Calculation> {
    let trimmed = raw.trim_matches(is_js_space);
    // `.replace(/^=\s*/, "")`
    let head = trimmed
        .strip_prefix('=')
        .map_or(trimmed, |rest| rest.trim_start_matches(is_js_space));
    // `.replace(/\s*=\s*\??$/, "")`
    let body = head.strip_suffix('?').unwrap_or(head);
    let expression = body
        .trim_end_matches(is_js_space)
        .strip_suffix('=')
        .map_or(head, |rest| rest.trim_end_matches(is_js_space));
    let chars: Vec<char> = expression.chars().collect();
    if chars.is_empty()
        || expression.encode_utf16().count() > 200
        || !has_number_or_constant(&chars)
        || is_date_like(&chars)
    {
        return None;
    }
    if let Some((value, unit)) = convert(expression) {
        return Some(Calculation {
            expression: expression.into(),
            value,
            display: format!("{} {unit}", format_number(value, 9)),
        });
    }
    let ends_with_postfix = chars
        .iter()
        .rev()
        .find(|&&c| !is_js_space(c))
        .is_some_and(|c| matches!(c, '!' | '%' | '°'));
    if !has_operator_between(&chars) && !ends_with_postfix && !has_function_call(&chars) {
        return None;
    }
    let value = evaluate(expression)?;
    Some(Calculation {
        expression: expression.into(),
        value,
        display: format_number(value, 12),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(raw: &str) -> Option<String> {
        calculate(raw).map(|c| c.display)
    }

    #[test]
    fn formats_like_js_numbers() {
        assert_eq!(shown("0.1 + 0.2").as_deref(), Some("0.3"));
        assert_eq!(shown("1e21 + 1").as_deref(), Some("1e+21"));
        assert_eq!(shown("1e-10 * 1").as_deref(), Some("1e-10"));
        assert_eq!(
            shown("1234567890.125 * 1").as_deref(),
            Some("1234567890.13")
        );
    }

    #[test]
    fn converts_units_and_temperatures() {
        assert_eq!(shown("10 km to mi").as_deref(), Some("6.21371192 mi"));
        assert_eq!(shown("100 c to f").as_deref(), Some("212 °F"));
        assert_eq!(shown("0 c to k").as_deref(), Some("273.15 K"));
        assert_eq!(shown("1 cup to fl  oz").as_deref(), Some("8 fl oz"));
        assert_eq!(shown("10 km to kg"), None);
    }

    #[test]
    fn rounds_and_signs_like_js() {
        assert_eq!(js_round(-2.5), -2.0);
        assert!(js_round(-0.4).is_sign_negative());
        assert_eq!(js_exp(1.0), std::f64::consts::E);
        assert_eq!(evaluate("1,234 + 1"), Some(1235.0));
        assert_eq!(evaluate("constructor ^ 0"), Some(1.0));
    }
}
