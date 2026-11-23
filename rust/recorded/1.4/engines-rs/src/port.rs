// Ported from ardia packages/core/src/engines.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `search::engines`: it reads the JSON arguments of the
//! recording as JS would (`typeof` checks, property reads, the JS errors of null and missing
//! values) into typed values, and writes the results in the shape the original returns. The
//! logic is in `ria_core::search::engines`; `JSON.parse` of `suggestReplyJson` is emulated here.
//!
//! shortcut: nested values the TS types describe (engine fields, extension fields) are read
//! leniently: a missing or non-string field reads as "" where the JS would see `undefined`. Upgrade
//! when a caller passes such values.

#![forbid(unsafe_code)]
use std::collections::{HashMap, HashSet};

use ria_core::search::engines::{
    self as core, CustomSearchEngine, EngineExtension, EngineInput, ExtensionSearchEngine,
    SearchEngine, StoredEngine, SuggestReply,
};
use serde_json::{Map, Value, json};

use crate::js;

const LONE: &str =
    "TypeError: a string in the arguments has a lone surrogate, which a Rust String cannot hold";

/// The keys `record` uses for JS values JSON cannot hold; a plain object with one of them as its
/// only key crosses wrapped in `$object`.
const MARKERS: [&str; 11] = [
    "$undefined",
    "$num",
    "$bigint",
    "$utf16",
    "$map",
    "$set",
    "$date",
    "$bytes",
    "$buffer",
    "$typed",
    "$object",
];

/// A plain JS object: not an array, nor a marker for another JS value.
fn object(v: &Value) -> Option<&Map<String, Value>> {
    let o = v.as_object()?;
    match o.iter().next() {
        Some((k, inner)) if o.len() == 1 && k == "$object" => inner.as_object(),
        Some((k, _)) if o.len() == 1 && MARKERS.contains(&k.as_str()) => None,
        _ => Some(o),
    }
}

/// "null" or "undefined" for those values: reading a property of them throws.
fn nullish(v: &Value) -> Option<&'static str> {
    match v {
        Value::Null => Some("null"),
        _ if js::is_undefined(v) => Some("undefined"),
        _ => None,
    }
}

fn read_error(of: &str, key: &str) -> String {
    format!("TypeError: Cannot read properties of {of} (reading '{key}')")
}

/// `v[key]` for a named key, `None` for `undefined`, with the JS error when `v` is null or undefined.
fn get<'a>(v: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    if let Some(n) = nullish(v) {
        return Err(read_error(n, key));
    }
    Ok(object(v)
        .and_then(|o| o.get(key))
        .filter(|x| !js::is_undefined(x)))
}

/// The JS string `v` holds, if it is one.
fn string(v: Option<&Value>) -> Result<Option<&str>, String> {
    match v {
        Some(Value::String(s)) => Ok(Some(s)),
        Some(x @ Value::Object(_)) if js::string_units(x).is_some() => Err(LONE.into()),
        _ => Ok(None),
    }
}

/// `!!v`.
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0),
        Some(x) => match js::as_number(x) {
            Some(n) => n != 0.0 && !n.is_nan(),
            None => !js::is_undefined(x),
        },
    }
}

/// A field that the TS type makes a string, read leniently (see the module shortcut).
fn text(v: &Value, key: &str) -> Result<String, String> {
    Ok(string(get(v, key)?)?.unwrap_or_default().to_owned())
}

/// The string fields of `raw` when it is an object (`customEngineFrom`, `engines.ts:92`).
fn stored_engine(raw: &Value) -> Result<StoredEngine<'_>, String> {
    let Some(o) = object(raw) else {
        return Ok(StoredEngine::default());
    };
    Ok(StoredEngine {
        id: string(o.get("id"))?,
        name: string(o.get("name"))?,
        keyword: string(o.get("keyword"))?,
        url: string(o.get("url"))?,
    })
}

/// `customEnginesFrom`: only an array has engines.
fn stored_engines(raw: &Value) -> Result<Vec<CustomSearchEngine>, String> {
    let Value::Array(items) = raw else {
        return Ok(vec![]);
    };
    let stored = items
        .iter()
        .map(stored_engine)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(core::custom_engines_from(&stored))
}

fn custom_json(e: &CustomSearchEngine) -> Value {
    json!({ "id": e.id, "name": e.name, "keyword": e.keyword, "url": e.url })
}

/// An engine as the original builds it: built-in engines have `suggestUrl` when they suggest,
/// custom ones `custom: true`, extension ones `suggestUrl` (maybe undefined) and `extension`.
fn engine_json(e: &SearchEngine) -> Value {
    let mut o = Map::new();
    o.insert("id".into(), e.id.clone().into());
    o.insert("name".into(), e.name.clone().into());
    o.insert("keyword".into(), e.keyword.clone().into());
    o.insert("url".into(), e.url.clone().into());
    match &e.suggest_url {
        Some(s) => _ = o.insert("suggestUrl".into(), s.clone().into()),
        None if e.extension.is_some() => _ = o.insert("suggestUrl".into(), js::undefined()),
        None => {}
    }
    if e.custom {
        o.insert("custom".into(), true.into());
    }
    if let Some(x) = &e.extension {
        o.insert("extension".into(), json!({ "id": x.id, "name": x.name }));
    }
    Value::Object(o)
}

/// An engine object of the arguments.
fn search_engine(v: &Value) -> Result<SearchEngine, String> {
    let extension = match get(v, "extension")? {
        Some(x) if nullish(x).is_none() => Some(EngineExtension {
            id: text(x, "id")?,
            name: text(x, "name")?,
        }),
        _ => None,
    };
    Ok(SearchEngine {
        id: text(v, "id")?,
        name: text(v, "name")?,
        keyword: text(v, "keyword")?,
        url: text(v, "url")?,
        suggest_url: string(get(v, "suggestUrl")?)?.map(Into::into),
        custom: truthy(get(v, "custom")?),
        extension,
    })
}

/// An extension engine object of the arguments.
fn extension_engine(v: &Value) -> Result<ExtensionSearchEngine, String> {
    Ok(ExtensionSearchEngine {
        profile: text(v, "profile")?,
        extension_id: text(v, "extensionId")?,
        extension_name: text(v, "extensionName")?,
        name: text(v, "name")?,
        keyword: text(v, "keyword")?,
        url: text(v, "url")?,
        suggest_url: string(get(v, "suggestUrl")?)?.map(Into::into),
        is_default: truthy(get(v, "isDefault")?),
    })
}

/// A value that is not an object nor null or undefined: all its named properties are undefined.
fn is_hole(v: &Value) -> bool {
    nullish(v).is_none() && object(v).is_none()
}

/// `input.<key>.trim()` of `validateEngine` and `makeCustomEngine`, with its JS errors.
fn engine_input(input: &Value) -> Result<EngineInput, String> {
    let field = |key: &str| -> Result<String, String> {
        match get(input, key)? {
            None => Err(read_error("undefined", "trim")),
            Some(Value::Null) => Err(read_error("null", "trim")),
            v => string(v)?
                .map(Into::into)
                .ok_or_else(|| format!("TypeError: input.{key}.trim is not a function")),
        }
    };
    Ok(EngineInput {
        name: field("name")?,
        keyword: field("keyword")?,
        url: field("url")?,
    })
}

/// `engine.url` where `searchUrlFor` calls `.includes` on it, with its JS errors.
fn template_of(engine: &Value) -> Result<&str, String> {
    match get(engine, "url")? {
        None => Err(read_error("undefined", "includes")),
        Some(Value::Null) => Err(read_error("null", "includes")),
        v => string(v)?.ok_or_else(|| "TypeError: template.includes is not a function".into()),
    }
}

/// Port of `validSearchTemplate` (`packages/core/src/engines.ts:89`).
pub fn valid_search_template(url: Value) -> Result<bool, String> {
    // A lone surrogate matches `[^/?#\s]` and is not `%s`, as U+FFFD: the answer is the same.
    Ok(match &url {
        Value::String(s) => core::valid_search_template(s),
        v => js::string_units(v)
            .is_some_and(|u| core::valid_search_template(&String::from_utf16_lossy(&u))),
    })
}

/// Port of `customEngineFrom` (`packages/core/src/engines.ts:91`).
pub fn custom_engine_from(raw: Value) -> Result<Value, String> {
    Ok(core::custom_engine_from(&stored_engine(&raw)?).map_or(Value::Null, |e| custom_json(&e)))
}

/// Port of `customEnginesFrom` (`packages/core/src/engines.ts:98`).
pub fn custom_engines_from(raw: Value) -> Result<Vec<Value>, String> {
    Ok(stored_engines(&raw)?.iter().map(custom_json).collect())
}

/// Port of `allEngines` (`packages/core/src/engines.ts:101`).
pub fn all_engines(custom: Value, extension: Value) -> Result<Vec<Value>, String> {
    let custom = stored_engines(&custom)?;
    // `for (const e of extension)`: an array, or a string (its characters); not an object.
    let items = match &extension {
        v if js::is_undefined(v) => vec![],
        Value::Array(items) => items.iter().collect(),
        Value::String(s) if s.is_empty() => vec![],
        v if js::string_units(v).is_some() => vec![v],
        Value::String(_) => vec![&extension],
        _ => return Err("TypeError: extension is not iterable".into()),
    };
    if let Some(n) = items.iter().find_map(|v| nullish(v)) {
        return Err(read_error(n, "extensionId"));
    }
    // Values without properties all have the extension id `undefined`: the first one adds an
    // engine of undefined fields.
    let hole = items.iter().position(|v| is_hole(v));
    let engines = items
        .iter()
        .filter(|v| !is_hole(v))
        .map(|v| extension_engine(v))
        .collect::<Result<Vec<_>, _>>()?;
    let mut out: Vec<Value> = core::all_engines(&custom, &engines)
        .iter()
        .map(engine_json)
        .collect();
    if let Some(hole) = hole {
        // After the built-in and custom engines, and the extensions added before the hole.
        let before = items[..hole].iter().filter(|v| !is_hole(v)).count();
        let ids: HashSet<&str> = engines[..before]
            .iter()
            .map(|e| e.extension_id.as_str())
            .collect();
        let at = core::BUILT_IN_ENGINES.len() + custom.len() + ids.len();
        let u = js::undefined();
        out.insert(
            at,
            json!({ "id": "extension:undefined", "name": u, "keyword": u, "url": u, "suggestUrl": u,
                "extension": { "id": u, "name": u } }),
        );
    }
    Ok(out)
}

/// Port of `extensionEngineId` (`packages/core/src/engines.ts:119`).
pub fn extension_engine_id(extension_id: String) -> Result<String, String> {
    Ok(core::extension_engine_id(&extension_id))
}

/// Port of `controllingExtensionEngine` (`packages/core/src/engines.ts:121`).
pub fn controlling_extension_engine(extension: Value) -> Result<Value, String> {
    if let Some(n) = nullish(&extension) {
        return Err(read_error(n, "find"));
    }
    let Value::Array(items) = &extension else {
        return Err("TypeError: extension.find is not a function".into());
    };
    let stop = items
        .iter()
        .position(|v| nullish(v).is_some())
        .unwrap_or(items.len());
    let objects: Vec<&Value> = items[..stop].iter().filter(|v| !is_hole(v)).collect();
    let engines = objects
        .iter()
        .map(|v| extension_engine(v))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(found) = core::controlling_extension_engine(&engines) {
        let i = engines
            .iter()
            .position(|e| std::ptr::eq(e, found))
            .unwrap_or(0);
        return Ok(objects[i].clone());
    }
    match items.get(stop).and_then(nullish) {
        Some(n) => Err(read_error(n, "isDefault")),
        None => Ok(js::undefined()),
    }
}

/// Port of `engineById` (`packages/core/src/engines.ts:125`).
pub fn engine_by_id(engines: Value, id: Option<String>) -> Result<Value, String> {
    if let Some(n) = nullish(&engines) {
        return Err(read_error(n, "find"));
    }
    let Value::Array(items) = &engines else {
        return Err("TypeError: engines.find is not a function".into());
    };
    // `find` stops at a null item (it throws) and, when `id` is undefined, at the first value
    // without properties (its `id` is undefined too).
    let stop = items
        .iter()
        .position(|v| nullish(v).is_some() || (id.is_none() && is_hole(v)))
        .unwrap_or(items.len());
    let objects: Vec<&Value> = items[..stop].iter().filter(|v| !is_hole(v)).collect();
    let typed = objects
        .iter()
        .map(|v| search_engine(v))
        .collect::<Result<Vec<_>, _>>()?;
    let found = core::engine_by_id(&typed, id.as_deref());
    if let Some(i) = typed.iter().position(|e| std::ptr::eq(e, found)) {
        return Ok(objects[i].clone());
    }
    match items.get(stop) {
        Some(v) => match nullish(v) {
            Some(n) => Err(read_error(n, "id")),
            None => Ok(v.clone()),
        },
        None => Ok(engine_json(found)),
    }
}

/// Port of `searchUrl` (`packages/core/src/engines.ts:129`).
pub fn search_url(engine: Value, query: String) -> Result<String, String> {
    Ok(core::search_url(template_of(&engine)?, &query))
}

/// Port of `suggestRequestUrl` (`packages/core/src/engines.ts:131`).
pub fn suggest_request_url(engine: Value, query: String) -> Result<Option<String>, String> {
    let suggest_url = get(&engine, "suggestUrl")?;
    if !truthy(suggest_url) {
        return Ok(None);
    }
    let engine = SearchEngine {
        suggest_url: string(suggest_url)?.map(Into::into),
        custom: truthy(get(&engine, "custom")?),
        ..core::BUILT_IN_ENGINES[0].clone()
    };
    Ok(core::suggest_request_url(&engine, &query))
}

/// Port of `engineHost` (`packages/core/src/engines.ts:134`).
pub fn engine_host(engine: Value) -> Result<String, String> {
    let url = match get(&engine, "url")? {
        None => return Err(read_error("undefined", "replace")),
        Some(Value::Null) => return Err(read_error("null", "replace")),
        v => string(v)?.ok_or("TypeError: engine.url.replace is not a function")?,
    };
    Ok(core::engine_host(url))
}

/// Port of `suggestReplyJson` (`packages/core/src/engines.ts:142`): the prefix is cut by
/// `ria-core`, `JSON.parse` is emulated with V8's messages.
pub fn suggest_reply_json(text: String) -> Result<Value, String> {
    let units: Vec<u16> = core::strip_reply_prefix(&text).encode_utf16().collect();
    JsonParser { s: &units, pos: 0 }.parse()
}
fn strings(items: &[Value]) -> Result<Vec<Option<&str>>, String> {
    items.iter().map(|v| string(Some(v))).collect()
}

/// The parts of a suggest reply `parseRemoteSuggestions` reads (`engines.ts:145`).
fn suggest_reply(body: &Value) -> Result<SuggestReply<'_>, String> {
    let (Value::Array(body), Some(Value::Array(items))) = (body, body.get(1)) else {
        return Ok(SuggestReply::default());
    };
    let titles = match body.get(2) {
        Some(Value::Array(t)) => strings(t)?,
        _ => vec![],
    };
    let types = match body
        .get(4)
        .filter(|x| nullish(x).is_none())
        .and_then(object)
    {
        Some(extras) => match extras.get("google:suggesttype") {
            Some(Value::Array(t)) => t.iter().map(js_string).collect(),
            _ => vec![],
        },
        None => vec![],
    };
    Ok(SuggestReply {
        suggestions: strings(items)?,
        titles,
        types,
    })
}

/// `String(v)` as far as comparing it with a suggest type needs: numbers print as some digits.
fn js_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if nullish(x).is_some() {
                    String::new()
                } else {
                    js_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ if js::is_undefined(v) => "undefined".into(),
        _ => match js::string_units(v) {
            Some(u) => String::from_utf16_lossy(&u),
            None if js::as_number(v).is_some() => "NaN".into(),
            None => "[object Object]".into(),
        },
    }
}

/// Port of `parseRemoteSuggestions` (`packages/core/src/engines.ts:144`).
pub fn parse_remote_suggestions(body: Value) -> Result<Value, String> {
    let got = core::parse_remote_suggestions(&suggest_reply(&body)?);
    let sites: Vec<Value> = got
        .sites
        .iter()
        .map(|s| json!({ "url": s.url, "title": s.title }))
        .collect();
    Ok(json!({ "queries": got.queries, "sites": sites }))
}

/// Port of `parseSuggestions` (`packages/core/src/engines.ts:161`).
pub fn parse_suggestions(body: Value) -> Result<Vec<String>, String> {
    Ok(core::parse_suggestions(&suggest_reply(&body)?))
}

/// Port of `validateEngine` (`packages/core/src/engines.ts:165`).
pub fn validate_engine(
    input: Value,
    engines: Value,
    ignore_id: Option<String>,
) -> Result<Option<String>, String> {
    let input = engine_input(&input)?;
    let ignore = ignore_id.as_deref();
    // `engines.some` runs only after the other checks pass, and throws at a null item, or at a
    // value without properties when `ignoreId` is a string (`undefined.toLowerCase()`).
    let (typed, fail) = match (&engines, nullish(&engines)) {
        (_, Some(n)) => (vec![], Some(read_error(n, "some"))),
        (Value::Array(items), None) => {
            let stop = items
                .iter()
                .position(|v| nullish(v).is_some() || (ignore.is_some() && is_hole(v)));
            let typed = items[..stop.unwrap_or(items.len())]
                .iter()
                .filter(|v| !is_hole(v))
                .map(search_engine)
                .collect::<Result<Vec<_>, _>>()?;
            let fail = stop.map(|i| match nullish(&items[i]) {
                Some(n) => read_error(n, "id"),
                None => read_error("undefined", "toLowerCase"),
            });
            (typed, fail)
        }
        _ => (
            vec![],
            Some("TypeError: engines.some is not a function".into()),
        ),
    };
    match (core::validate_engine(&input, &typed, ignore), fail) {
        (None, Some(e)) => Err(e),
        (message, _) => Ok(message),
    }
}

/// Port of `makeCustomEngine` (`packages/core/src/engines.ts:178`).
pub fn make_custom_engine(input: Value, id: String) -> Result<Value, String> {
    Ok(custom_json(&core::make_custom_engine(
        &engine_input(&input)?,
        &id,
    )))
}

/// `JSON.parse` over UTF-16 code units, with V8's results and error messages.
struct JsonParser<'a> {
    s: &'a [u16],
    pos: usize,
}

/// V8's error messages that name a position.
const PROP_NAME_OR_BRACE: &str = "Expected property name or '}'";
const DOUBLE_QUOTED_NAME: &str = "Expected double-quoted property name";
const COLON: &str = "Expected ':' after property name";
const COMMA_OR_BRACE: &str = "Expected ',' or '}' after property value";
const COMMA_OR_BRACKET: &str = "Expected ',' or ']' after array element";

impl JsonParser<'_> {
    fn parse(mut self) -> Result<Value, String> {
        self.skip_space();
        let v = self.value()?;
        self.skip_space();
        if self.pos < self.s.len() {
            return Err(self.at("Unexpected non-whitespace character after JSON"));
        }
        Ok(v)
    }

    fn peek(&self) -> Option<u16> {
        self.s.get(self.pos).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(0x20 | 0x09 | 0x0a | 0x0d)) {
            self.pos += 1;
        }
    }

    /// `<message> in JSON at position N (line L column C)` at the current position.
    fn at(&self, message: &str) -> String {
        let (mut line, mut line_start, mut i) = (1, 0, 0);
        while i < self.pos {
            if self.s[i] == 0x0d && i + 1 < self.pos && self.s[i + 1] == 0x0a {
                i += 1;
            }
            if matches!(self.s[i], 0x0a | 0x0d) {
                line += 1;
                line_start = i + 1;
            }
            i += 1;
        }
        let column = 1 + self.pos - line_start;
        format!(
            "SyntaxError: {message} in JSON at position {} (line {line} column {column})",
            self.pos
        )
    }

    /// V8's message for the unexpected character at the current position.
    fn unexpected(&self) -> String {
        let s = self.s;
        let pos = self.pos;
        let Some(c) = self.peek() else {
            return "SyntaxError: Unexpected end of JSON input".into();
        };
        if c == u16::from(b'-') || (u16::from(b'0')..=u16::from(b'9')).contains(&c) {
            return self.at("Unexpected number");
        }
        if c == u16::from(b'"') {
            return self.at("Unexpected string");
        }
        let text = |u: &[u16]| String::from_utf16_lossy(u);
        let source = text(s);
        if ["undefined", "NaN", "Infinity", "[object Object]"].contains(&source.as_str()) {
            return format!("SyntaxError: \"{source}\" is not valid JSON");
        }
        let token = text(&[c]);
        const CONTEXT: usize = 10;
        let (pre, start, end, post) = if s.len() <= 2 * CONTEXT {
            ("", 0, s.len(), "")
        } else if pos < CONTEXT {
            ("", 0, pos + CONTEXT, "...")
        } else if pos < s.len() - CONTEXT {
            ("...", pos - CONTEXT, pos + CONTEXT, "...")
        } else {
            ("...", pos - CONTEXT, s.len(), "")
        };
        format!(
            "SyntaxError: Unexpected token '{token}', {pre}\"{}\"{post} is not valid JSON",
            text(&s[start..end])
        )
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek().and_then(|c| u8::try_from(c).ok()) {
            Some(b'"') => Ok(js::utf16_string(&self.string()?)),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            _ => Err(self.unexpected()),
        }
    }

    fn literal(&mut self, word: &str, v: Value) -> Result<Value, String> {
        for b in word.bytes() {
            if self.peek() != Some(u16::from(b)) {
                return Err(self.unexpected());
            }
            self.pos += 1;
        }
        Ok(v)
    }

    fn digit(&self) -> bool {
        self.peek()
            .is_some_and(|c| (u16::from(b'0')..=u16::from(b'9')).contains(&c))
    }

    fn digits(&mut self) {
        while self.digit() {
            self.pos += 1;
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some(u16::from(b'-')) {
            self.pos += 1;
        }
        if self.peek() == Some(u16::from(b'0')) {
            self.pos += 1;
            if self.digit() {
                return Err(self.unexpected());
            }
        } else if self.digit() {
            self.digits();
        } else {
            return Err(self.at("No number after minus sign"));
        }
        if self.peek() == Some(u16::from(b'.')) {
            self.pos += 1;
            if !self.digit() {
                return Err(self.at("Unterminated fractional number"));
            }
            self.digits();
        }
        if matches!(self.peek(), Some(0x65 | 0x45)) {
            self.pos += 1;
            if matches!(self.peek(), Some(0x2b | 0x2d)) {
                self.pos += 1;
            }
            if !self.digit() {
                return Err(self.at("Exponent part is missing a number"));
            }
            self.digits();
        }
        let literal = String::from_utf16_lossy(&self.s[start..self.pos]);
        Ok(js::number(literal.parse::<f64>().unwrap_or(f64::NAN)))
    }

    /// A string literal at `"`, as UTF-16 code units.
    fn string(&mut self) -> Result<Vec<u16>, String> {
        self.pos += 1;
        let mut out = Vec::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(self.at("Unterminated string"));
            };
            match c {
                0x22 => {
                    self.pos += 1;
                    return Ok(out);
                }
                0x5c => {
                    self.pos += 1;
                    let Some(e) = self.peek() else {
                        return Err(self.unexpected());
                    };
                    let simple = match e {
                        0x22 | 0x5c | 0x2f => Some(e),
                        0x62 => Some(0x08),
                        0x66 => Some(0x0c),
                        0x6e => Some(0x0a),
                        0x72 => Some(0x0d),
                        0x74 => Some(0x09),
                        _ => None,
                    };
                    if let Some(u) = simple {
                        out.push(u);
                        self.pos += 1;
                    } else if e == 0x75 {
                        self.pos += 1;
                        let mut unit = 0u16;
                        for _ in 0..4 {
                            let Some(d) = self
                                .peek()
                                .and_then(|d| char::from_u32(d.into())?.to_digit(16))
                            else {
                                return Err(self.at("Bad Unicode escape"));
                            };
                            unit = unit * 16 + d as u16;
                            self.pos += 1;
                        }
                        out.push(unit);
                    } else {
                        return Err(self.at("Bad escaped character"));
                    }
                }
                c if c < 0x20 => return Err(self.at("Bad control character in string literal")),
                c => {
                    out.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.pos += 1;
        self.skip_space();
        let mut items = Vec::new();
        if self.peek() == Some(u16::from(b']')) {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_space();
            match self.peek() {
                Some(0x2c) => {
                    self.pos += 1;
                    self.skip_space();
                }
                Some(0x5d) => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.at(COMMA_OR_BRACKET)),
            }
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.pos += 1;
        self.skip_space();
        let mut entries: Vec<(String, Value)> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        if self.peek() == Some(u16::from(b'}')) {
            self.pos += 1;
        } else {
            let mut expected = PROP_NAME_OR_BRACE;
            loop {
                if self.peek() != Some(u16::from(b'"')) {
                    return Err(self.at(expected));
                }
                let key = String::from_utf16(&self.string()?).map_err(|_| LONE.to_string())?;
                self.skip_space();
                if self.peek() != Some(u16::from(b':')) {
                    return Err(self.at(COLON));
                }
                self.pos += 1;
                self.skip_space();
                let v = self.value()?;
                match index.get(&key) {
                    Some(&i) => entries[i].1 = v,
                    None => {
                        index.insert(key.clone(), entries.len());
                        entries.push((key, v));
                    }
                }
                self.skip_space();
                match self.peek() {
                    Some(0x2c) => {
                        self.pos += 1;
                        self.skip_space();
                        expected = DOUBLE_QUOTED_NAME;
                    }
                    Some(0x7d) => {
                        self.pos += 1;
                        break;
                    }
                    _ => return Err(self.at(COMMA_OR_BRACE)),
                }
            }
        }
        // JS objects list array-index keys first, in numeric order.
        entries.sort_by_key(|(k, _)| array_index(k).map_or((1, 0), |i| (0, i)));
        let wrap = entries.len() == 1 && MARKERS.contains(&entries[0].0.as_str());
        let o = Value::Object(entries.into_iter().collect());
        Ok(if wrap { json!({ "$object": o }) } else { o })
    }
}

/// The value of a key that is a JS array index: a canonical number below 2^32 - 1.
fn array_index(key: &str) -> Option<u32> {
    let canonical =
        key == "0" || (!key.starts_with('0') && key.bytes().all(|b| b.is_ascii_digit()));
    key.parse::<u32>()
        .ok()
        .filter(|&i| canonical && i < u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Value, String> {
        suggest_reply_json(text.into())
    }

    #[test]
    fn json_parse_messages_match_v8() {
        let cases = [
            ("", "SyntaxError: Unexpected end of JSON input"),
            (
                "[1",
                "SyntaxError: Expected ',' or ']' after array element in JSON at position 2 (line 1 column 3)",
            ),
            (
                "{\"a\":1,}",
                "SyntaxError: Expected double-quoted property name in JSON at position 7 (line 1 column 8)",
            ),
            (
                "tr1",
                "SyntaxError: Unexpected number in JSON at position 2 (line 1 column 3)",
            ),
            (
                "1.e",
                "SyntaxError: Unterminated fractional number in JSON at position 2 (line 1 column 3)",
            ),
            (
                "{\"a\"\n:1,\n\"b\" 2}",
                "SyntaxError: Expected ':' after property name in JSON at position 13 (line 3 column 5)",
            ),
            (
                "[1,2,3,4,5,6,7,8,9,x]",
                "SyntaxError: Unexpected token 'x', ...\"5,6,7,8,9,x]\" is not valid JSON",
            ),
            ("NaN", "SyntaxError: \"NaN\" is not valid JSON"),
        ];
        for (text, message) in cases {
            assert_eq!(parse(text), Err(message.into()), "{text:?}");
        }
    }

    #[test]
    fn json_parse_orders_index_keys_and_keeps_js_values() {
        let got = parse(")]}'\n{\"b\":1,\"2\":2,\"a\":-0,\"1\":4,\"b\":5}").unwrap_or_default();
        assert_eq!(got.to_string(), r#"{"1":4,"2":2,"b":5,"a":{"$num":"-0"}}"#);
        assert_eq!(parse("\"\\ud800\""), Ok(json!({ "$utf16": [0xd800] })));
    }
}
