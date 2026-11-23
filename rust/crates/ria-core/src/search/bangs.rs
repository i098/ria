// Ported from ardia packages/core/src/bangs.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Bangs (`!gh react`): the slim bang list made from a Helium/Kagi bang file, its checks, the
//! table that looks triggers up, and the URL a bang opens.
//!
//! Changes from the original: a `BangTable` is one concrete type (slim lists, joined or not)
//! instead of any object with `size` and `get`; `parse_loose_json` returns [`Json`], whose
//! strings hold U+FFFD where `JSON.parse` keeps a lone surrogate escape; `wellFormed` is not
//! needed because a Rust string is always well-formed.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::OnceLock;

use super::js_text::{encode_uri_component, is_js_space, js_trim};

/// A bang: the trigger that was looked up, the site name, the search URL template and the site's
/// home page (`bangs.ts:1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bang {
    pub trigger: String,
    pub name: String,
    pub template: String,
    pub home: String,
}

/// A bang found in the omnibox text, the rest of the text and the URL to open (`bangs.ts:5`).
#[derive(Debug, PartialEq, Eq)]
pub struct BangMatch<'a> {
    pub bang: &'a Bang,
    pub query: String,
    pub url: String,
}

/// `bangs.ts:7`.
pub const BANG_PLACEHOLDER: &str = "{searchTerms}";
const STORED_PLACEHOLDER: &str = "{s}";
const MIN_BANGS: usize = 1000;
/// The largest slim list, in UTF-16 code units (`bangs.ts:10`).
pub const MAX_BANG_LIST_CHARS: usize = 4_000_000;
const REQUIRED_TRIGGERS: [&str; 3] = ["gh", "w", "yt"];
const BANG_DOCUMENT: &str = "ardia-bangs 1";

/// The JS `.length` of `text`: its UTF-16 code units.
fn js_length(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `text.indexOf(pattern, from)` in bytes; `from` is on a character boundary.
fn find_from(text: &str, pattern: &str, from: usize) -> Option<usize> {
    text[from..].find(pattern).map(|i| from + i)
}

// ---- parseLooseJson ---------------------------------------------------------------------------

/// A value of `JSON.parse`. An object keeps each key at its first position with its last value,
/// as a JS object does; a JS object also lists array-index keys first, which callers that care
/// about order apply themselves.
#[derive(Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The value of member `key` when this is an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Moves the children of `value` onto `stack`.
fn take_children(value: &mut Json, stack: &mut Vec<Json>) {
    match value {
        Json::Array(items) => stack.append(items),
        Json::Object(members) => stack.extend(members.drain(..).map(|(_, v)| v)),
        _ => {}
    }
}

impl Drop for Json {
    // `JSON.parse` takes any depth, so the drop must not recurse.
    fn drop(&mut self) {
        let mut stack = Vec::new();
        take_children(self, &mut stack);
        while let Some(mut value) = stack.pop() {
            take_children(&mut value, &mut stack);
        }
    }
}

/// A `JSON.parse` failure, with the message V8 gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError(pub String);

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JsonError {}

/// `endOfString` (`bangs.ts:14`): the index after the quote that closes the string at `from`.
fn end_of_string(text: &[u8], from: usize) -> usize {
    let mut at = from;
    while let Some(i) = text[at..].iter().position(|&c| c == b'"') {
        let quote = at + i;
        let slashes = text[..quote]
            .iter()
            .rev()
            .take_while(|&&c| c == b'\\')
            .count();
        if slashes % 2 == 0 {
            return quote + 1;
        }
        at = quote + 1;
    }
    text.len()
}

/// `skipBlank` (`bangs.ts:26`): the index of the next character that is not JS space or a comment.
fn skip_blank(text: &str, from: usize) -> usize {
    let mut at = from;
    loop {
        at += text[at..]
            .find(|c| !is_js_space(c))
            .unwrap_or(text.len() - at);
        if text[at..].starts_with("//") {
            at = find_from(text, "\n", at).unwrap_or(text.len());
        } else if text[at..].starts_with("/*") {
            at = find_from(text, "*/", at + 2).map_or(text.len(), |end| end + 2);
        } else {
            return at;
        }
    }
}

/// The text `parseLooseJson` gives `JSON.parse`: comments and trailing commas out, strings kept
/// (`bangs.ts:41-64`).
fn strip_loose(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let (mut from, mut at) = (0, 0);
    while let Some(i) = bytes[at..]
        .iter()
        .position(|c| matches!(c, b'"' | b'/' | b','))
    {
        let i = at + i;
        match bytes[i] {
            b'"' => at = end_of_string(bytes, i + 1),
            b'/' => {
                let end = match bytes.get(i + 1) {
                    Some(b'/') => find_from(text, "\n", i).unwrap_or(text.len()),
                    Some(b'*') => find_from(text, "*/", i + 2).map_or(text.len(), |end| end + 2),
                    _ => {
                        at = i + 1;
                        continue;
                    }
                };
                out.push_str(&text[from..i]);
                from = end;
                at = end;
            }
            _ => {
                at = i + 1;
                if matches!(bytes.get(skip_blank(text, i + 1)), Some(b']' | b'}')) {
                    out.push_str(&text[from..i]);
                    from = i + 1;
                }
            }
        }
    }
    out.push_str(&text[from..]);
    out
}

/// Port of `parseLooseJson` (`bangs.ts:40`): `JSON.parse` of JSON with `//` and `/* */`
/// comments and trailing commas.
pub fn parse_loose_json(text: &str) -> Result<Json, JsonError> {
    JsonParser::new(&strip_loose(text)).parse()
}

/// A container `JsonParser` is filling; an object holds the key of the value being parsed.
enum Frame {
    Array(Vec<Json>),
    Object(Vec<(String, Json)>, String),
}

/// `JSON.parse` as V8's iterative `JsonParser` runs it: the same values, and the same error
/// messages at the same positions (UTF-16, with line and column).
struct JsonParser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
}

impl<'a> JsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            bytes: text.as_bytes(),
            at: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// V8 `Check`: skips whitespace and takes `token` when it is next.
    fn check(&mut self, token: u8) -> bool {
        self.skip_whitespace();
        let next = self.peek() == Some(token);
        if next {
            self.at += 1;
        }
        next
    }

    /// V8 `Expect` after whitespace was skipped.
    fn expect(&mut self, token: u8, message: &str) -> Result<(), JsonError> {
        if self.peek() == Some(token) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    /// The key after `{` or `,` and the `:` after it.
    fn key(&mut self, message: &str) -> Result<String, JsonError> {
        self.skip_whitespace();
        self.expect(b'"', message)?;
        let key = self.string()?;
        self.skip_whitespace();
        self.expect(b':', "Expected ':' after property name")?;
        Ok(key)
    }

    fn parse(mut self) -> Result<Json, JsonError> {
        let mut stack: Vec<Frame> = Vec::new();
        'produce: loop {
            let mut value = loop {
                self.skip_whitespace();
                match self.peek() {
                    Some(b'"') => {
                        self.at += 1;
                        break Json::String(self.string()?);
                    }
                    Some(b'-' | b'0'..=b'9') => break Json::Number(self.number()?),
                    Some(b'{') => {
                        self.at += 1;
                        if self.check(b'}') {
                            break Json::Object(Vec::new());
                        }
                        let key = self.key("Expected property name or '}'")?;
                        stack.push(Frame::Object(Vec::new(), key));
                    }
                    Some(b'[') => {
                        self.at += 1;
                        if self.check(b']') {
                            break Json::Array(Vec::new());
                        }
                        stack.push(Frame::Array(Vec::new()));
                    }
                    Some(b't') => {
                        self.literal("true")?;
                        break Json::Bool(true);
                    }
                    Some(b'f') => {
                        self.literal("false")?;
                        break Json::Bool(false);
                    }
                    Some(b'n') => {
                        self.literal("null")?;
                        break Json::Null;
                    }
                    _ => return Err(self.unexpected()),
                }
            };
            loop {
                match stack.pop() {
                    None => {
                        if self.check_end() {
                            return Ok(value);
                        }
                        return Err(self.error("Unexpected non-whitespace character after JSON"));
                    }
                    Some(Frame::Array(mut items)) => {
                        items.push(value);
                        if self.check(b',') {
                            stack.push(Frame::Array(items));
                            continue 'produce;
                        }
                        self.expect(b']', "Expected ',' or ']' after array element")?;
                        value = Json::Array(items);
                    }
                    Some(Frame::Object(mut members, key)) => {
                        members.push((key, value));
                        if self.check(b',') {
                            let key = self.key("Expected double-quoted property name")?;
                            stack.push(Frame::Object(members, key));
                            continue 'produce;
                        }
                        self.expect(b'}', "Expected ',' or '}' after property value")?;
                        value = Json::Object(last_value_wins(members));
                    }
                }
            }
        }
    }

    fn check_end(&mut self) -> bool {
        self.skip_whitespace();
        self.at == self.bytes.len()
    }

    /// V8 `ScanLiteral`; the first character matched already.
    fn literal(&mut self, word: &str) -> Result<(), JsonError> {
        for &expected in &word.as_bytes()[1..] {
            self.at += 1;
            match self.peek() {
                Some(c) if c == expected => {}
                _ => return Err(self.unexpected()),
            }
        }
        self.at += 1;
        Ok(())
    }

    fn skip_digits(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.at += 1;
        }
    }

    /// V8 `ParseJsonNumber`.
    fn number(&mut self) -> Result<f64, JsonError> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        if self.peek() == Some(b'0') {
            self.at += 1;
            if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(self.error("Unexpected number"));
            }
        } else {
            let digits = self.at;
            self.skip_digits();
            if self.at == digits {
                return Err(self.error("No number after minus sign"));
            }
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(self.error("Unterminated fractional number"));
            }
            self.skip_digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(self.error("Exponent part is missing a number"));
            }
            self.skip_digits();
        }
        // The grammar above is a subset of what `f64::from_str` takes.
        Ok(self.text[start..self.at].parse().unwrap_or(f64::NAN))
    }

    /// V8 `ScanJsonString`, after the opening quote. A `\u` escape of a lone surrogate gives U+FFFD.
    fn string(&mut self) -> Result<String, JsonError> {
        let mut out = String::new();
        let mut high: Option<u16> = None;
        loop {
            let run = self.at;
            while self
                .peek()
                .is_some_and(|c| c != b'"' && c != b'\\' && c >= 0x20)
            {
                self.at += 1;
            }
            if self.at > run {
                flush_high(&mut out, &mut high);
                out.push_str(&self.text[run..self.at]);
            }
            match self.peek() {
                None => return Err(self.error("Unterminated string")),
                Some(b'"') => {
                    self.at += 1;
                    flush_high(&mut out, &mut high);
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.at += 1;
                    let Some(c) = self.text[self.at..].chars().next() else {
                        return Err(self.unexpected());
                    };
                    let plain = match c {
                        '"' | '\\' | '/' => c,
                        'b' => '\u{8}',
                        'f' => '\u{c}',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'u' => {
                            let unit = self.unicode_escape()?;
                            push_unit(&mut out, &mut high, unit);
                            self.at += 1;
                            continue;
                        }
                        c if u32::from(c) > 0xff => return Err(self.unexpected()),
                        _ => return Err(self.error("Bad escaped character")),
                    };
                    flush_high(&mut out, &mut high);
                    out.push(plain);
                    self.at += 1;
                }
                Some(_) => return Err(self.error("Bad control character in string literal")),
            }
        }
    }

    /// The four hex digits after `\u`; leaves `at` on the last one.
    fn unicode_escape(&mut self) -> Result<u16, JsonError> {
        let mut unit = 0;
        for _ in 0..4 {
            self.at += 1;
            let digit = self.peek().and_then(|c| char::from(c).to_digit(16));
            let Some(digit) = digit else {
                return Err(self.error("Bad Unicode escape"));
            };
            unit = unit * 16 + digit as u16;
        }
        Ok(unit)
    }

    /// The UTF-16 position of `at`.
    fn position(&self) -> usize {
        js_length(&self.text[..self.at])
    }

    /// `<message> in JSON at position N (line L column C)`; V8 `CalculateFileLocation`.
    fn error(&self, message: &str) -> JsonError {
        let (mut line, mut line_start, mut i) = (1, 0, 0);
        while i < self.at {
            if self.bytes[i] == b'\r' && i + 1 < self.at && self.bytes[i + 1] == b'\n' {
                i += 1;
            }
            if matches!(self.bytes[i], b'\r' | b'\n') {
                line += 1;
                line_start = i + 1;
            }
            i += 1;
        }
        let column = 1 + js_length(&self.text[line_start..self.at]);
        JsonError(format!(
            "{message} in JSON at position {} (line {line} column {column})",
            self.position()
        ))
    }

    /// V8 `ReportUnexpectedCharacter` at `at`: the message names the token and quotes the text
    /// around it.
    fn unexpected(&self) -> JsonError {
        match self.peek() {
            None => JsonError("Unexpected end of JSON input".into()),
            Some(b'"') => self.error("Unexpected string"),
            Some(b'-' | b'0'..=b'9') => self.error("Unexpected number"),
            Some(_)
                if matches!(
                    self.text,
                    "[object Object]" | "undefined" | "Infinity" | "NaN"
                ) =>
            {
                JsonError(format!("\"{}\" is not valid JSON", self.text))
            }
            Some(_) => {
                const CONTEXT: usize = 10;
                let units: Vec<u16> = self.text.encode_utf16().collect();
                let pos = self.position();
                let token = String::from_utf16_lossy(&units[pos..pos + 1]);
                let text = |range: std::ops::Range<usize>| String::from_utf16_lossy(&units[range]);
                let len = units.len();
                let quoted = if len <= CONTEXT * 2 {
                    format!("\"{}\"", self.text)
                } else if pos < CONTEXT {
                    format!("\"{}\"...", text(0..pos + CONTEXT))
                } else if pos < len - CONTEXT {
                    format!("...\"{}\"...", text(pos - CONTEXT..pos + CONTEXT))
                } else {
                    format!("...\"{}\"", text(pos - CONTEXT..len))
                };
                JsonError(format!(
                    "Unexpected token '{token}', {quoted} is not valid JSON"
                ))
            }
        }
    }
}

/// Ends a pending high surrogate escape that no low one followed.
fn flush_high(out: &mut String, high: &mut Option<u16>) {
    if high.take().is_some() {
        out.push(char::REPLACEMENT_CHARACTER);
    }
}

/// Adds the UTF-16 unit of a `\u` escape: a surrogate pair joins, a lone surrogate is U+FFFD.
fn push_unit(out: &mut String, high: &mut Option<u16>, unit: u16) {
    if let Some(h) = high.take() {
        if (0xdc00..0xe000).contains(&unit) {
            let c = 0x10000 + ((u32::from(h) - 0xd800) << 10) + (u32::from(unit) - 0xdc00);
            out.push(char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER));
            return;
        }
        out.push(char::REPLACEMENT_CHARACTER);
    }
    if (0xd800..0xdc00).contains(&unit) {
        *high = Some(unit);
    } else {
        out.push(char::from_u32(unit.into()).unwrap_or(char::REPLACEMENT_CHARACTER));
    }
}

/// Object members as JS keeps them: a repeated key stays at its first position with its last value.
fn last_value_wins(mut members: Vec<(String, Json)>) -> Vec<(String, Json)> {
    if members.len() < 2 {
        return members;
    }
    let mut order: Vec<usize> = (0..members.len()).collect();
    order.sort_by(|&a, &b| members[a].0.cmp(&members[b].0));
    let mut keep = vec![true; members.len()];
    let mut swaps = Vec::new();
    for same in order.chunk_by(|&a, &b| members[a].0 == members[b].0) {
        if let [first, rest @ .., last] = same {
            swaps.push((*first, *last));
            keep[*last] = false;
            for &i in rest {
                keep[i] = false;
            }
        }
    }
    for (first, last) in swaps {
        members.swap(first, last);
    }
    let mut keeps = keep.into_iter();
    members.retain(|_| keeps.next().unwrap_or(true));
    members
}

// ---- templates --------------------------------------------------------------------------------

/// The text after `^https?:\/\/` (case-insensitive), when it is there.
fn after_web_scheme(text: &str) -> Option<&str> {
    let rest = text
        .get(..4)
        .filter(|s| s.eq_ignore_ascii_case("http"))
        .map(|_| &text[4..])?;
    let rest = rest.strip_prefix(['s', 'S']).unwrap_or(rest);
    rest.strip_prefix("://")
}

/// Port of `isValidBangTemplate` (`bangs.ts:71`): `VALID_TEMPLATE` (`bangs.ts:68`),
/// `^https?:\/\/[a-z0-9.-]+(?::\d+)?(?:[/?#]\S*)?$` with `i`.
pub fn is_valid_bang_template(template: &str) -> bool {
    let Some(rest) = after_web_scheme(template) else {
        return false;
    };
    let host = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
        .unwrap_or(rest.len());
    if host == 0 {
        return false;
    }
    let mut rest = &rest[host..];
    if let Some(port) = rest.strip_prefix(':') {
        let digits = port
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(port.len());
        if digits == 0 {
            return false;
        }
        rest = &port[digits..];
    }
    rest.is_empty() || (rest.starts_with(['/', '?', '#']) && !rest.contains(is_js_space))
}

/// `ORIGIN` (`bangs.ts:69`), `^([a-z][a-z0-9+.-]*):\/\/([^/?#\\]*)` with `i`: scheme and host.
fn origin(url: &str) -> Option<(&str, &str)> {
    if !url.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let scheme = url
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-')))
        .unwrap_or(url.len());
    let after = url[scheme..].strip_prefix("://")?;
    let host = after.find(['/', '?', '#', '\\']).unwrap_or(after.len());
    Some((&url[..scheme], &after[..host]))
}

/// Port of `staysOnSite` (`bangs.ts:73`): `url` has the scheme and host (with port) of `template`.
pub fn stays_on_site(url: &str, template: &str) -> bool {
    let (Some(to), Some(from)) = (origin(url), origin(template)) else {
        return false;
    };
    if from.1.is_empty() || url.contains(|c: char| is_js_space(c) || c <= '\u{1f}' || c == '\u{7f}')
    {
        return false;
    }
    to.0.eq_ignore_ascii_case(from.0) && to.1.to_lowercase() == from.1.to_lowercase()
}

/// `expandTemplate` (`bangs.ts:80`): a stored template back to a full one.
fn expand_template(stored: &str) -> String {
    let full = match after_web_scheme(stored) {
        Some(_) => stored.to_string(),
        None => format!("https://{stored}"),
    };
    full.replace(STORED_PLACEHOLDER, BANG_PLACEHOLDER)
}

// ---- slim lists -------------------------------------------------------------------------------

/// Port of `slimBangs` (`bangs.ts:83`): a Helium/Kagi bang file (a JSON array of `{s, ts, u}`)
/// as a slim list, one `triggers\tname\tstored template` line per bang; `None` when the file is
/// not JSON or has fewer than 1000 usable bangs.
pub fn slim_bangs(json: &str) -> Option<String> {
    let list = parse_loose_json(json).ok()?;
    let Json::Array(entries) = &list else {
        return None;
    };
    let mut seen = HashSet::new();
    let mut lines = Vec::new();
    for entry in entries {
        let (Some(Json::String(s)), Some(Json::Array(ts)), Some(Json::String(u))) =
            (entry.get("s"), entry.get("ts"), entry.get("u"))
        else {
            continue;
        };
        if !is_valid_bang_template(u) || u.contains(STORED_PLACEHOLDER) {
            continue;
        }
        let name = s
            .split(is_js_space)
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if name.is_empty() {
            continue;
        }
        let mut triggers = Vec::new();
        for t in ts {
            let Json::String(t) = t else { continue };
            let trigger = js_trim(t).to_lowercase();
            if trigger.is_empty()
                || trigger.contains(|c| is_js_space(c) || c == '!')
                || !seen.insert(trigger.clone())
            {
                continue;
            }
            triggers.push(trigger);
        }
        if triggers.is_empty() {
            continue;
        }
        let stored = match u.get(..8) {
            Some(scheme) if scheme.eq_ignore_ascii_case("https://") => &u[8..],
            _ => u.as_str(),
        };
        let stored = stored.replace(BANG_PLACEHOLDER, STORED_PLACEHOLDER);
        lines.push(format!("{}\t{name}\t{stored}", triggers.join(" ")));
    }
    (lines.len() >= MIN_BANGS).then(|| lines.join("\n"))
}

/// `homeOf` (`bangs.ts:115`): the origin of `template` with `/`, or `template` itself.
fn home_of(template: &str) -> String {
    match after_web_scheme(template) {
        Some(rest) => {
            let host = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            if host == 0 {
                return template.to_string();
            }
            let end = template.len() - rest.len() + host;
            format!("{}/", &template[..end])
        }
        None => template.to_string(),
    }
}

/// A trigger's line in a slim list, and its bang once looked up.
struct Line {
    start: usize,
    bang: OnceLock<Option<Box<Bang>>>,
}

/// One slim list and the line of each trigger in it (`bangs.ts:120-159`).
struct SlimTable {
    slim: String,
    lines: HashMap<String, Line>,
}

impl SlimTable {
    /// `indexTriggers` (`bangs.ts:120`): the first line of each trigger.
    fn new(slim: String) -> Self {
        let mut lines = HashMap::new();
        let mut at = 0;
        while at < slim.len() {
            let end = find_from(&slim, "\n", at).unwrap_or(slim.len());
            if let Some(tab) = slim[at..end].find('\t')
                && tab > 0
            {
                for trigger in slim[at..at + tab].split(' ') {
                    if !trigger.is_empty() && !lines.contains_key(trigger) {
                        let bang = OnceLock::new();
                        lines.insert(trigger.to_string(), Line { start: at, bang });
                    }
                }
            }
            at = end + 1;
        }
        Self { slim, lines }
    }

    /// The `get` of `bangTable` (`bangs.ts:143`) for a lower-cased trigger.
    fn get(&self, key: &str) -> Option<&Bang> {
        let line = self.lines.get(key)?;
        line.bang
            .get_or_init(|| self.parse_line(line.start, key).map(Box::new))
            .as_deref()
    }

    fn parse_line(&self, start: usize, key: &str) -> Option<Bang> {
        let end = find_from(&self.slim, "\n", start).unwrap_or(self.slim.len());
        let mut fields = self.slim[start..end].split('\t').skip(1);
        let name = fields.next().unwrap_or_default();
        let template = expand_template(fields.next().unwrap_or_default());
        (!name.is_empty() && is_valid_bang_template(&template)).then(|| Bang {
            trigger: key.to_string(),
            name: name.to_string(),
            home: home_of(&template),
            template,
        })
    }
}

/// Bangs by trigger, from one slim list or several joined (`bangs.ts:3`). A bang is parsed from
/// its line the first time it is looked up.
#[derive(Default)]
pub struct BangTable {
    lists: Vec<SlimTable>,
}

impl BangTable {
    /// The number of triggers, summed over joined lists.
    pub fn size(&self) -> usize {
        self.lists.iter().map(|list| list.lines.len()).sum()
    }

    /// The bang of `trigger` in any case, from the first list that has a valid one.
    pub fn get(&self, trigger: &str) -> Option<&Bang> {
        let key = trigger.to_lowercase();
        self.lists.iter().find_map(|list| list.get(&key))
    }
}

/// Port of `bangTable` (`bangs.ts:140`).
pub fn bang_table(slim: String) -> BangTable {
    BangTable {
        lists: vec![SlimTable::new(slim)],
    }
}

/// `^[^\s!]+(?: [^\s!]+)*$` (`bangs.ts:166`).
fn valid_triggers(triggers: &str) -> bool {
    triggers
        .split(' ')
        .all(|t| !t.is_empty() && !t.contains(|c| is_js_space(c) || c == '!'))
}

/// Port of `checkedBangTable` (`bangs.ts:161`): the table of a stored slim list when every line
/// is valid, there are at least 1000, `gh`, `w` and `yt` resolve, and it is not too big.
pub fn checked_bang_table(slim: &str) -> Option<BangTable> {
    if slim.len() > MAX_BANG_LIST_CHARS && js_length(slim) > MAX_BANG_LIST_CHARS {
        return None;
    }
    let mut count = 0;
    for line in slim.split('\n') {
        let mut fields = line.split('\t');
        let (Some(triggers), Some(name), Some(stored), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return None;
        };
        if triggers.is_empty()
            || name.is_empty()
            || stored.is_empty()
            || !valid_triggers(triggers)
            || !is_valid_bang_template(&expand_template(stored))
        {
            return None;
        }
        count += 1;
    }
    if count < MIN_BANGS {
        return None;
    }
    let table = bang_table(slim.to_string());
    REQUIRED_TRIGGERS
        .iter()
        .all(|t| table.get(t).is_some())
        .then_some(table)
}

/// Port of `bangDocument` (`bangs.ts:175`): a slim list stored with the ETag it came with.
pub fn bang_document(etag: Option<&str>, slim: &str) -> String {
    format!("{BANG_DOCUMENT}\t{}\n{slim}", etag.unwrap_or_default())
}

/// What `readBangDocument` reads (`bangs.ts:177`).
pub struct BangDocument {
    pub etag: Option<String>,
    pub table: BangTable,
}

/// Port of `readBangDocument` (`bangs.ts:177`).
pub fn read_bang_document(doc: Option<&str>) -> Option<BangDocument> {
    let doc = doc.filter(|doc| !doc.is_empty())?;
    let max = MAX_BANG_LIST_CHARS + 1024;
    if doc.len() > max && js_length(doc) > max {
        return None;
    }
    let first_line = doc.find('\n')?;
    let mut head = doc[..first_line].split('\t');
    let (Some(BANG_DOCUMENT), etag, None) = (head.next(), head.next(), head.next()) else {
        return None;
    };
    let table = checked_bang_table(&doc[first_line + 1..])?;
    Some(BangDocument {
        etag: etag.filter(|etag| !etag.is_empty()).map(str::to_string),
        table,
    })
}

/// Port of `joinBangTables` (`bangs.ts:187`): one table that looks in each of `tables` in order.
pub fn join_bang_tables(tables: Vec<BangTable>) -> BangTable {
    BangTable {
        lists: tables.into_iter().flat_map(|table| table.lists).collect(),
    }
}

// ---- finding and filling ----------------------------------------------------------------------

/// Port of `findBang` (`bangs.ts:200`): a known `!trigger` as the first or last word of `text`,
/// unless the text starts with `?`; the URL must stay on the bang's site.
pub fn find_bang<'a>(text: &str, table: Option<&'a BangTable>) -> Option<BangMatch<'a>> {
    let table = table?;
    if !text.contains('!') || text.trim_start_matches(is_js_space).starts_with('?') {
        return None;
    }
    let words: Vec<&str> = js_trim(text)
        .split(is_js_space)
        .filter(|w| !w.is_empty())
        .collect();
    let last = words.len().checked_sub(1)?;
    for at in std::iter::once(0).chain((last > 0).then_some(last)) {
        let Some(trigger) = words[at].strip_prefix('!').filter(|t| !t.is_empty()) else {
            continue;
        };
        let Some(bang) = table.get(trigger) else {
            continue;
        };
        let query = words
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != at)
            .map(|(_, w)| *w)
            .collect::<Vec<_>>()
            .join(" ");
        let url = if query.is_empty() {
            bang.home.clone()
        } else {
            fill_bang_template(&bang.template, &query)
        };
        if stays_on_site(&url, &bang.template) {
            return Some(BangMatch { bang, query, url });
        }
    }
    None
}

/// `PATH_UNSAFE` (`bangs.ts:220`): what is escaped in a path, besides non-printable ASCII.
const PATH_UNSAFE: &str = "\"#%:<>?[\\]^`{|}";

/// Port of `escapeSearchTerms` (`bangs.ts:222`): `encodeURIComponent` with `+` for spaces in a
/// query; in a path, `/` and the other safe printable ASCII stay.
pub fn escape_search_terms(terms: &str, in_query: bool) -> String {
    if in_query {
        return encode_uri_component(terms).replace("%20", "+");
    }
    let mut out = String::with_capacity(terms.len());
    for c in terms.chars() {
        if ('\u{21}'..='\u{7e}').contains(&c) && !PATH_UNSAFE.contains(c) {
            out.push(c);
        } else {
            out.push_str(&encode_uri_component(c.encode_utf8(&mut [0; 4])));
        }
    }
    out
}

/// Port of `fillBangTemplate` (`bangs.ts:227`): `terms` in place of each `{searchTerms}`, escaped
/// as a query when the first one is after a `?`.
pub fn fill_bang_template(template: &str, terms: &str) -> String {
    let Some(at) = template.find(BANG_PLACEHOLDER) else {
        return template.to_string();
    };
    let in_query = template.find('?').is_some_and(|question| at > question);
    template.replace(BANG_PLACEHOLDER, &escape_search_terms(terms, in_query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loose_json_drops_comments_and_trailing_commas() {
        let parsed = parse_loose_json(
            "// head\n[\n {\"u\": \"https://a//b\", /* c */ \"ts\": [\"x\",],},\n // tail\n]\n",
        );
        let expected = Json::Array(vec![Json::Object(vec![
            ("u".into(), Json::String("https://a//b".into())),
            ("ts".into(), Json::Array(vec![Json::String("x".into())])),
        ])]);
        assert_eq!(parsed, Ok(expected));
    }

    #[test]
    fn json_errors_read_like_v8() {
        let message = |text: &str| parse_loose_json(text).err().map(|e| e.0);
        assert_eq!(
            message("[1"),
            Some(
                "Expected ',' or ']' after array element in JSON at position 2 (line 1 column 3)"
                    .into()
            )
        );
        assert_eq!(
            message("[1,2,3,4,5,6,7,8,9,10,11,x]"),
            Some("Unexpected token 'x', ...\"8,9,10,11,x]\" is not valid JSON".into())
        );
        assert_eq!(
            message("\r\n[1 x]"),
            Some(
                "Expected ',' or ']' after array element in JSON at position 5 (line 2 column 4)"
                    .into()
            )
        );
        assert_eq!(message("NaN"), Some("\"NaN\" is not valid JSON".into()));
    }

    #[test]
    fn json_keys_and_surrogates() {
        let parsed = parse_loose_json(r#"{"a":1,"b":2,"a":3,"s":"\ud83d\ude00\ud800x"}"#);
        let expected = Json::Object(vec![
            ("a".into(), Json::Number(3.0)),
            ("b".into(), Json::Number(2.0)),
            ("s".into(), Json::String("😀\u{fffd}x".into())),
        ]);
        assert_eq!(parsed, Ok(expected));
    }

    #[test]
    fn deep_json_parses_and_drops() {
        let depth = 1_000_000;
        let text = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        assert!(matches!(parse_loose_json(&text), Ok(Json::Array(_))));
    }

    #[test]
    fn templates_and_sites() {
        assert!(is_valid_bang_template(
            "HTTPS://a.example:8080/?q={searchTerms}"
        ));
        assert!(!is_valid_bang_template("https://a.example/ x"));
        assert!(stays_on_site(
            "https://github.com\\@evil.example/",
            "https://github.com/{searchTerms}"
        ));
        assert!(!stays_on_site(
            "https://github.com@evil.example/",
            "https://github.com/{searchTerms}"
        ));
    }

    #[test]
    fn table_finds_bangs_at_either_end() {
        let table = bang_table(
            "gh git\tGitHub\tgithub.com/search?q={s}\nghr\tGitHub Repo\tgithub.com/{s}".into(),
        );
        assert_eq!(table.size(), 3);
        let url = |text: &str| find_bang(text, Some(&table)).map(|m| m.url);
        assert_eq!(
            url("!GH react hooks"),
            Some("https://github.com/search?q=react+hooks".into())
        );
        assert_eq!(
            url("a b?c#d%e !ghr"),
            Some("https://github.com/a%20b%3Fc%23d%25e".into())
        );
        assert_eq!(url("  !git  "), Some("https://github.com/".into()));
        assert_eq!(url("?!gh react"), None);
        let joined = join_bang_tables(vec![bang_table("x\tX\tx.example/{s}".into()), table]);
        assert_eq!(joined.size(), 4);
        assert_eq!(joined.get("GIT").map(|b| b.name.as_str()), Some("GitHub"));
    }
}
