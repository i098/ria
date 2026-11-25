// Ported from ardia apps/browser/src/lib/pluginChecks.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Limits, patterns and field checks shared by the plugin manifest checks: each check adds a
//! problem to a [`Check`] and returns the value or a fallback.
//!
//! Changes from Ardia:
//! - `unknown` is `Option<&Json>` (`ria_core::identity::telemetry_wire::Json`): `None` is `undefined`,
//!   `Some(&Json::Null)` is `null`. `isArgument` takes `&Json`, because `undefined` is not a string either.
//! - `LIMITS` is the struct [`Limits`] with snake_case fields: counts are `usize`, ranges `[f64; 2]`.
//! - `at` takes a [`Key`] for `string | number`.
//! - Optional parameters (`required = true`, `fallback = false`, `fallback = "puzzlepiece"`,
//!   `extension?`) are plain parameters.
//! - Lengths count UTF-16 code units, as in JS. A `&str` cannot hold a lone surrogate, so the
//!   `isWellFormed` checks are always true here.
//! - `VERSION` is compiled with `[0-9]` for `\d`: JS `\d` is ASCII only, Rust's is Unicode.
//! - `only` reads keys in `Object.keys` order (array indexes first, with `js_order`).

use regex::Regex;
use ria_core::identity::telemetry_wire::{Json, js_number_to_string, js_order};
use std::sync::LazyLock;

use super::decode::Fields;
use super::js::js_trim;

/// The type of `LIMITS` (`pluginChecks.ts:4`).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub name: usize,
    pub title: usize,
    pub description: usize,
    pub page_title: usize,
    pub rows: usize,
    pub row_text: usize,
    pub choices: [f64; 2],
    pub commands: usize,
    pub keywords: usize,
    pub keyword: usize,
    pub plugin_keywords: usize,
    pub example: usize,
    pub telemetry_items: usize,
    pub telemetry_item: usize,
    pub tools: usize,
    pub tool_timeout: [f64; 2],
    pub argv: usize,
    pub arg_bytes: usize,
    pub memory_mb: [f64; 2],
    pub programs: usize,
    pub after_update: [f64; 2],
    pub hooks: usize,
    pub menu_items: usize,
    pub toolbar: usize,
    pub sidebar_views: usize,
    pub panels: usize,
    pub omnibox: usize,
    pub themes: usize,
    pub activation_events: usize,
    pub allowed_hosts: usize,
    pub when: usize,
    pub tooltip: usize,
}

/// `LIMITS` (`pluginChecks.ts:4`).
pub const LIMITS: Limits = Limits {
    name: 40,
    title: 60,
    description: 200,
    page_title: 40,
    rows: 20,
    row_text: 200,
    choices: [2.0, 12.0],
    commands: 20,
    keywords: 8,
    keyword: 40,
    plugin_keywords: 12,
    example: 120,
    telemetry_items: 16,
    telemetry_item: 800,
    tools: 32,
    tool_timeout: [1.0, 300.0],
    argv: 32,
    arg_bytes: 1024,
    memory_mb: [64.0, 4096.0],
    programs: 4,
    after_update: [4.0, 8.0],
    hooks: 64,
    menu_items: 8,
    toolbar: 4,
    sidebar_views: 4,
    panels: 8,
    omnibox: 4,
    themes: 8,
    activation_events: 32,
    allowed_hosts: 32,
    when: 200,
    tooltip: 120,
};

/// `DEFAULT_SIDECAR_MB` (`pluginChecks.ts:39`).
pub const DEFAULT_SIDECAR_MB: f64 = 1024.0;
/// `DEFAULT_MAIN_MB` (`pluginChecks.ts:40`).
pub const DEFAULT_MAIN_MB: f64 = 256.0;

/// `NAME_PATTERN` (`pluginChecks.ts:42`).
pub const NAME_PATTERN: &str = "^[a-z0-9][a-z0-9-]{0,39}$";
/// `KEY_PATTERN` (`pluginChecks.ts:43`).
pub const KEY_PATTERN: &str = "^[A-Za-z0-9_-]{1,40}$";
/// `VERSION_PATTERN` (`pluginChecks.ts:44`), in JS syntax.
pub const VERSION_PATTERN: &str = "^\\d{1,6}\\.\\d{1,6}\\.\\d{1,6}(-[0-9A-Za-z.-]{1,40})?$";
/// `SYMBOL_PATTERN` (`pluginChecks.ts:45`).
pub const SYMBOL_PATTERN: &str = "^[a-z0-9.]{1,64}$";
/// `KEYWORD_PATTERN` (`pluginChecks.ts:46`).
pub const KEYWORD_PATTERN: &str = "^[A-Za-z0-9][A-Za-z0-9_-]{0,31}$";

fn pattern(source: &str) -> Regex {
    Regex::new(source).expect("static pattern")
}

/// `NAME` (`pluginChecks.ts:47`).
pub static NAME: LazyLock<Regex> = LazyLock::new(|| pattern(NAME_PATTERN));
/// `KEY` (`pluginChecks.ts:48`).
pub static KEY: LazyLock<Regex> = LazyLock::new(|| pattern(KEY_PATTERN));
/// `VERSION` (`pluginChecks.ts:49`).
pub static VERSION: LazyLock<Regex> =
    LazyLock::new(|| pattern(&VERSION_PATTERN.replace("\\d", "[0-9]")));
/// `SYMBOL` (`pluginChecks.ts:50`).
pub static SYMBOL: LazyLock<Regex> = LazyLock::new(|| pattern(SYMBOL_PATTERN));
/// `KEYWORD` (`pluginChecks.ts:51`).
pub static KEYWORD: LazyLock<Regex> = LazyLock::new(|| pattern(KEYWORD_PATTERN));
/// `UNSAFE_TEXT` (`pluginChecks.ts:52`): control and text-direction characters.
pub static UNSAFE_TEXT: LazyLock<Regex> = LazyLock::new(|| {
    pattern(r"[\x00-\x1f\x7f-\x{9f}\x{61c}\x{200e}\x{200f}\x{202a}-\x{202e}\x{2066}-\x{2069}]")
});

/// `NAME_RULE` (`pluginChecks.ts:54`).
pub const NAME_RULE: &str =
    "1 to 40 lowercase letters, digits and dashes, starting with a letter or digit";
/// `KEY_RULE` (`pluginChecks.ts:55`).
pub const KEY_RULE: &str = "1 to 40 letters, digits, - and _";
/// `SYMBOL_RULE` (`pluginChecks.ts:56`).
pub const SYMBOL_RULE: &str = "an SF Symbol name";

/// `Check` (`pluginChecks.ts:58`): the problems found so far.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Check {
    pub problems: Vec<String>,
}

/// The `key` of `at`: `string | number`.
#[derive(Debug, Clone, Copy)]
pub enum Key<'a> {
    Name(&'a str),
    Index(f64),
}

impl<'a> From<&'a str> for Key<'a> {
    fn from(name: &'a str) -> Self {
        Key::Name(name)
    }
}

impl From<usize> for Key<'_> {
    fn from(index: usize) -> Self {
        Key::Index(index as f64)
    }
}

/// `at` (`pluginChecks.ts:60`): the path of `key` under `path`.
pub fn at<'a>(path: &str, key: impl Into<Key<'a>>) -> String {
    match key.into() {
        Key::Index(i) => format!("{path}[{}]", js_number_to_string(i)),
        Key::Name(k) if !path.is_empty() => format!("{path}.{k}"),
        Key::Name(k) => k.to_owned(),
    }
}

/// `only` (`pluginChecks.ts:62`): a problem for each key of `doc` not in `keys`.
pub fn only(c: &mut Check, path: &str, doc: &Fields, keys: &[&str]) {
    for (key, _) in js_order(doc) {
        if !keys.contains(&key.as_str()) {
            let of = if path.is_empty() {
                "of a plugin".to_owned()
            } else {
                format!("of {path}")
            };
            c.problems
                .push(format!("{} isn't a key {of}", at(path, key.as_str())));
        }
    }
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `text` (`pluginChecks.ts:66`): text of 1 to `max` UTF-16 code units, not blank, without
/// control or direction characters.
pub fn text<'a>(
    c: &mut Check,
    path: &str,
    value: Option<&'a Json>,
    max: usize,
    required: bool,
) -> Option<&'a str> {
    if value.is_none() && !required {
        return None;
    }
    let Some(Json::String(value)) = value.filter(|v| match v {
        Json::String(s) => !js_trim(s).is_empty() && utf16_len(s) <= max,
        _ => false,
    }) else {
        c.problems
            .push(format!("{path} must be text of 1 to {max} characters"));
        return None;
    };
    if UNSAFE_TEXT.is_match(value) {
        c.problems
            .push(format!("{path} has control or direction characters"));
        return None;
    }
    Some(value)
}

/// `matching` (`pluginChecks.ts:79`): a string that `pattern` matches.
pub fn matching<'a>(
    c: &mut Check,
    path: &str,
    value: Option<&'a Json>,
    pattern: &Regex,
    rule: &str,
    required: bool,
) -> Option<&'a str> {
    match value {
        None if !required => None,
        Some(Json::String(s)) if pattern.is_match(s) => Some(s),
        _ => {
            c.problems.push(format!("{path} must be {rule}"));
            None
        }
    }
}

/// `list` (`pluginChecks.ts:86`): an array of at most `max` items; `undefined` is empty.
pub fn list<'a>(c: &mut Check, path: &str, value: Option<&'a Json>, max: usize) -> &'a [Json] {
    match value {
        None => &[],
        Some(Json::Array(items)) if items.len() <= max => items,
        _ => {
            c.problems
                .push(format!("{path} must be a list of at most {max}"));
            &[]
        }
    }
}

/// `object` (`pluginChecks.ts:93`).
pub fn object<'a>(c: &mut Check, path: &str, value: Option<&'a Json>) -> Option<&'a Fields> {
    match value {
        Some(Json::Object(fields)) => Some(fields),
        _ => {
            c.problems.push(format!("{path} must be an object"));
            None
        }
    }
}

/// `integer` (`pluginChecks.ts:99`): a whole number in `[min, max]`; `undefined` is `fallback`.
pub fn integer(
    c: &mut Check,
    path: &str,
    value: Option<&Json>,
    [min, max]: [f64; 2],
    fallback: f64,
) -> f64 {
    match value {
        None => fallback,
        Some(&Json::Number(n)) if n.is_finite() && n.trunc() == n && n >= min && n <= max => n,
        _ => {
            c.problems.push(format!(
                "{path} must be a whole number from {} to {}",
                js_number_to_string(min),
                js_number_to_string(max)
            ));
            fallback
        }
    }
}

/// `flag` (`pluginChecks.ts:106`): a boolean; `undefined` is `fallback`.
pub fn flag(c: &mut Check, path: &str, value: Option<&Json>, fallback: bool) -> bool {
    match value {
        None => fallback,
        Some(&Json::Bool(b)) => b,
        _ => {
            c.problems.push(format!("{path} must be true or false"));
            fallback
        }
    }
}

/// `unique` (`pluginChecks.ts:113`): a problem for each name seen before; `None` is skipped.
pub fn unique<S: AsRef<str>>(
    c: &mut Check,
    path: &str,
    names: impl IntoIterator<Item = Option<S>>,
    what: &str,
) {
    let mut seen = std::collections::HashSet::new();
    for name in names.into_iter().flatten() {
        let name = name.as_ref();
        if !seen.insert(name.to_owned()) {
            c.problems
                .push(format!("{path} has two {what} named {name}"));
        }
    }
}

/// `symbolOf` (`pluginChecks.ts:122`): an SF Symbol name, else `fallback` (Ardia's default is
/// `"puzzlepiece"`).
pub fn symbol_of<'a>(
    c: &mut Check,
    path: &str,
    value: Option<&'a Json>,
    fallback: &'a str,
) -> &'a str {
    matching(c, path, value, &SYMBOL, SYMBOL_RULE, false).unwrap_or(fallback)
}

/// `isPluginPath` (`pluginChecks.ts:124`): a relative path in the plugin folder.
pub fn is_plugin_path(value: &str) -> bool {
    if value.is_empty()
        || utf16_len(value) > LIMITS.arg_bytes
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains('\0')
    {
        return false;
    }
    value
        .split('/')
        .all(|part| !matches!(part, "" | "." | ".."))
}

/// `isArgument` (`pluginChecks.ts:129`): a string of 1 to `LIMITS.arg_bytes` UTF-8 bytes, no NUL.
pub fn is_argument(arg: &Json) -> bool {
    matches!(arg, Json::String(s) if !s.is_empty() && s.len() <= LIMITS.arg_bytes && !s.contains('\0'))
}

/// `pluginFile` (`pluginChecks.ts:131`): a plugin path that ends in `extension` (any case).
pub fn plugin_file<'a>(
    c: &mut Check,
    path: &str,
    value: Option<&'a Json>,
    rule: &str,
    extension: Option<&str>,
) -> Option<&'a str> {
    match value {
        Some(Json::String(s))
            if is_plugin_path(s)
                && extension.is_none_or(|e| e.is_empty() || s.to_lowercase().ends_with(e)) =>
        {
            Some(s)
        }
        _ => {
            c.problems.push(format!("{path} must be {rule}"));
            None
        }
    }
}
