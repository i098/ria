// Ported from ardia apps/browser/src/lib/themeFile.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Theme files: the colour roles a theme replaces per appearance, checked against Ardia's own
//! palettes, merged over them, and described as a JSON Schema with the look blocks of `theme_look`.
//!
//! Changes from Ardia:
//! - `RoleValue` is an enum; a palette (`Record<string, RoleValue>`) is a list of `(role, value)` in
//!   `Object.entries` order. Values outside the TS type (`null`, objects) cannot be in a palette.
//! - `THEME_NAME` is a `regex::Regex`. `COLOR_PATTERN` keeps the JavaScript text for the schema; the
//!   check compiles it with JavaScript's `\s` set and an ASCII `\d`, as JavaScript reads them.
//! - `JSON.parse` is `ria_core::search::bangs::parse_json`, with V8's error messages. Its result
//!   converts to `Json` by recursion. shortcut: recursive conversion, upgrade when theme files nest
//!   deeply. A string with a lone surrogate (`"\ud800"`) cannot be a Rust `String`.
//! - `parseThemeFile` and `checkedRole` give `Result` for the `{ value } | { error }` union.

use std::sync::LazyLock;

use regex::Regex;
use ria_core::identity::telemetry_wire::{Json, js_number_to_string, js_order};
use ria_core::search::bangs::{Json as Parsed, parse_json};

use super::color::to_hex;
use super::js::js_number;
use super::json_schema::{JSON_SCHEMA_DIALECT, JsonSchema};
use super::theme_look::{LOOK_KEYS, LookBlocks, look_schemas, parse_look_blocks};

/// `RoleValue` (`themeFile.ts:6`).
#[derive(Clone, Debug, PartialEq)]
pub enum RoleValue {
    Color(String),
    Number(f64),
    Bool(bool),
    Pair([String; 2]),
}

impl RoleValue {
    /// The value as JSON.
    pub fn to_json(&self) -> Json {
        match self {
            RoleValue::Color(s) => Json::String(s.clone()),
            RoleValue::Number(n) => Json::Number(*n),
            RoleValue::Bool(b) => Json::Bool(*b),
            RoleValue::Pair(p) => Json::Array(p.iter().map(|s| Json::String(s.clone())).collect()),
        }
    }
}

/// `RolePalette` (`themeFile.ts:7`).
pub type RolePalette = Vec<(String, RoleValue)>;

/// `RolePalettes` (`themeFile.ts:8`); also `ThemeRoles` (`themeFile.ts:10`), which has the same shape.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RolePalettes {
    pub dark: RolePalette,
    pub light: RolePalette,
}

/// `ThemeRoles` (`themeFile.ts:10`).
pub type ThemeRoles = RolePalettes;

/// `BasePalette` (`themeFile.ts:9`).
#[derive(Clone, Debug, PartialEq)]
pub struct BasePalette {
    pub revision: f64,
    pub dark: RolePalette,
    pub light: RolePalette,
}

/// `ThemeMode` (`themeFile.ts:11`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    /// The JS name of the mode.
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

impl RolePalettes {
    /// `palettes[mode]`.
    pub fn of(&self, mode: ThemeMode) -> &RolePalette {
        match mode {
            ThemeMode::Dark => &self.dark,
            ThemeMode::Light => &self.light,
        }
    }

    fn of_mut(&mut self, mode: ThemeMode) -> &mut RolePalette {
        match mode {
            ThemeMode::Dark => &mut self.dark,
            ThemeMode::Light => &mut self.light,
        }
    }
}

/// `ParsedTheme` (`themeFile.ts:12`).
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedTheme {
    pub roles: Option<ThemeRoles>,
    pub look: Option<LookBlocks>,
    pub problems: Vec<String>,
}

/// `THEME_NAME` (`themeFile.ts:14`).
pub static THEME_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[a-z0-9][a-z0-9_-]*$").expect("valid pattern"));
/// `THEME_MODES` (`themeFile.ts:15`).
pub const THEME_MODES: [ThemeMode; 2] = [ThemeMode::Dark, ThemeMode::Light];

macro_rules! channel {
    () => {
        r"\s*(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\s*"
    };
}
macro_rules! alpha {
    () => {
        r"\s*(0|1|0?\.\d+|1\.0+)\s*"
    };
}

/// `COLOR_PATTERN` (`themeFile.ts:19`), as JavaScript text.
pub const COLOR_PATTERN: &str = concat!(
    r"^(?:#[0-9A-Fa-f]{3}|#[0-9A-Fa-f]{6}|#[0-9A-Fa-f]{8}|rgb\(",
    channel!(),
    ",",
    channel!(),
    ",",
    channel!(),
    r"\)|rgba\(",
    channel!(),
    ",",
    channel!(),
    ",",
    channel!(),
    ",",
    alpha!(),
    r"\))$"
);

/// JavaScript's `\s` (`String.prototype.trim` white space and line terminators).
const JS_SPACE: &str = r"[\t\n\x0B\x0C\r \xA0\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]";

/// `COLOR` (`themeFile.ts:20`).
static COLOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        &COLOR_PATTERN
            .replace(r"\s", JS_SPACE)
            .replace(r"\d", "[0-9]"),
    )
    .expect("valid pattern")
});

const COLOR_RULE: &str = "a colour (#RGB, #RRGGBB, #RRGGBBAA, or rgb or rgba with r, g, b from 0 to 255 and a from 0 to 1)";

/// `themeColor` (`themeFile.ts:23`): colour text in Ardia's form (`#RRGGBB` or `#RRGGBBAA`), or `None`.
pub fn theme_color(value: &Json) -> Option<String> {
    let Json::String(value) = value else {
        return None;
    };
    let m = COLOR.captures(value)?;
    if let Some(digits) = value.strip_prefix('#') {
        let full = if digits.len() == 3 {
            digits.chars().flat_map(|c| [c, c]).collect()
        } else {
            digits.to_owned()
        };
        return Some(format!("#{}", full.to_uppercase()));
    }
    let group =
        |a: usize, b: usize| js_number(m.get(a).or_else(|| m.get(b)).map_or("", |g| g.as_str()));
    let (r, g, b) = (group(1, 4), group(2, 5), group(3, 6));
    let a = m.get(7).map_or(1.0, |g| js_number(g.as_str()));
    Some(to_hex(&if a == 1.0 {
        vec![r, g, b]
    } else {
        vec![r, g, b, a * 255.0]
    }))
}

/// `RoleKind` (`themeFile.ts:37`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoleKind {
    Color,
    Pair,
    Number {
        minimum: Option<f64>,
        maximum: Option<f64>,
    },
}

/// `numberRange` (`themeFile.ts:39`).
fn number_range(role: &str) -> (Option<f64>, Option<f64>) {
    if role == "grain" || role.ends_with("Opacity") {
        return (Some(0.0), Some(1.0));
    }
    if role.ends_with("Radius") {
        return (Some(0.0), Some(100.0));
    }
    (None, None)
}

/// `roleKind` (`themeFile.ts:45`): what kind of value a role holds, from Ardia's value for it.
pub fn role_kind(role: &str, value: &RoleValue) -> Option<RoleKind> {
    match value {
        RoleValue::Color(_) => Some(RoleKind::Color),
        RoleValue::Number(_) => {
            let (minimum, maximum) = number_range(role);
            Some(RoleKind::Number { minimum, maximum })
        }
        RoleValue::Pair(_) => Some(RoleKind::Pair),
        RoleValue::Bool(_) => None,
    }
}

/// `checkedRole` (`themeFile.ts:54`).
fn checked_role(kind: RoleKind, value: &Json) -> Result<RoleValue, String> {
    match kind {
        RoleKind::Color => theme_color(value)
            .map(RoleValue::Color)
            .ok_or_else(|| format!("must be {COLOR_RULE}")),
        RoleKind::Pair => match value {
            Json::Array(items) if items.len() == 2 => {
                match (theme_color(&items[0]), theme_color(&items[1])) {
                    (Some(top), Some(bottom)) => Ok(RoleValue::Pair([top, bottom])),
                    _ => Err(format!("must be two colours, each {COLOR_RULE}")),
                }
            }
            _ => Err(format!("must be two colours, each {COLOR_RULE}")),
        },
        RoleKind::Number { minimum, maximum } => {
            let in_range = matches!(value, Json::Number(n) if n.is_finite()
                && minimum.is_none_or(|min| *n >= min)
                && maximum.is_none_or(|max| *n <= max));
            match (value, minimum) {
                (Json::Number(n), _) if in_range => Ok(RoleValue::Number(*n)),
                (_, None) => Err("must be a number".into()),
                (_, Some(min)) => Err(format!(
                    "must be a number from {} to {}",
                    js_number_to_string(min),
                    maximum.map_or("undefined".into(), js_number_to_string)
                )),
            }
        }
    }
}

/// `JSON.parse` output as `Json`.
fn from_parsed(value: &Parsed) -> Json {
    match value {
        Parsed::Null => Json::Null,
        Parsed::Bool(b) => Json::Bool(*b),
        Parsed::Number(n) => Json::Number(*n),
        Parsed::String(s) => Json::String(s.clone()),
        Parsed::Array(items) => Json::Array(items.iter().map(from_parsed).collect()),
        Parsed::Object(entries) => Json::Object(
            entries
                .iter()
                .map(|(k, v)| (k.clone(), from_parsed(v)))
                .collect(),
        ),
    }
}

/// `parseThemeFile` (`themeFile.ts:69`): the roles and look blocks of theme file `text`, with each
/// problem found; `roles` and `look` are `None` when the text is not a JSON object.
pub fn parse_theme_file(file: &str, text: &str, defaults: &RolePalettes) -> ParsedTheme {
    let doc = match parse_json(text) {
        Ok(doc) => from_parsed(&doc),
        Err(error) => {
            return ParsedTheme {
                roles: None,
                look: None,
                problems: vec![format!("{file}: isn't valid JSON ({error})")],
            };
        }
    };
    let Json::Object(doc) = doc else {
        return ParsedTheme {
            roles: None,
            look: None,
            problems: vec![format!("{file}: must be an object")],
        };
    };
    let mut problems = Vec::new();
    let keys: Vec<&str> = THEME_MODES
        .iter()
        .map(|m| m.name())
        .chain(LOOK_KEYS)
        .collect();
    for (key, _) in js_order(&doc) {
        if key != "$schema" && !keys.contains(&key.as_str()) {
            problems.push(format!(
                "{file}: {key} isn't a theme key ({})",
                keys.join(", ")
            ));
        }
    }
    let mut roles = ThemeRoles::default();
    for mode in THEME_MODES {
        let Some((_, given)) = doc.iter().find(|(k, _)| k == mode.name()) else {
            continue;
        };
        let Json::Object(given) = given else {
            problems.push(format!(
                "{file}: {} must be an object of colour roles",
                mode.name()
            ));
            continue;
        };
        for (role, value) in js_order(given) {
            let known = defaults
                .of(mode)
                .iter()
                .find(|(k, _)| k == role)
                .and_then(|(_, v)| role_kind(role, v));
            let Some(known) = known else {
                problems.push(format!(
                    "{file}: {}.{role} isn't a colour role",
                    mode.name()
                ));
                continue;
            };
            match checked_role(known, value) {
                Err(error) => problems.push(format!("{file}: {}.{role} {error}", mode.name())),
                Ok(v) => set_role(roles.of_mut(mode), role, v),
            }
        }
    }
    let look = parse_look_blocks(file, &doc, &mut problems);
    ParsedTheme {
        roles: Some(roles),
        look: Some(look),
        problems,
    }
}

/// `palette[role] = value`: a role already set keeps its place.
fn set_role(palette: &mut RolePalette, role: &str, value: RoleValue) {
    match palette.iter_mut().find(|(k, _)| k == role) {
        Some(entry) => entry.1 = value,
        None => palette.push((role.to_owned(), value)),
    }
}

/// `{ ...defaults, ...roles }`.
fn merged(defaults: &RolePalette, roles: &RolePalette) -> RolePalette {
    let mut out = defaults.clone();
    for (role, value) in roles {
        set_role(&mut out, role, value.clone());
    }
    out
}

/// `mergedPalettes` (`themeFile.ts:102`): Ardia's palettes with a theme's roles over them.
pub fn merged_palettes(
    defaults: &RolePalettes,
    roles: Option<&ThemeRoles>,
    revision: f64,
) -> BasePalette {
    let Some(roles) = roles else {
        return BasePalette {
            revision,
            dark: defaults.dark.clone(),
            light: defaults.light.clone(),
        };
    };
    BasePalette {
        revision,
        dark: merged(&defaults.dark, &roles.dark),
        light: merged(&defaults.light, &roles.light),
    }
}

fn object<'a>(fields: impl IntoIterator<Item = (&'a str, Json)>) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn text(s: &str) -> Json {
    Json::String(s.into())
}

/// `roleSchema` (`themeFile.ts:107`).
fn role_schema(kind: RoleKind, value: &RoleValue) -> JsonSchema {
    let color = || object([("type", text("string")), ("pattern", text(COLOR_PATTERN))]);
    match kind {
        RoleKind::Color => object([
            ("type", text("string")),
            ("pattern", text(COLOR_PATTERN)),
            ("default", value.to_json()),
        ]),
        RoleKind::Pair => object([
            ("type", text("array")),
            ("prefixItems", Json::Array(vec![color(), color()])),
            ("minItems", Json::Number(2.0)),
            ("maxItems", Json::Number(2.0)),
            ("default", value.to_json()),
            (
                "description",
                text("Two colours: the top and bottom of the border"),
            ),
        ]),
        RoleKind::Number { minimum, maximum } => {
            let range = minimum.zip(maximum).map(|(min, max)| {
                [
                    ("minimum", Json::Number(min)),
                    ("maximum", Json::Number(max)),
                ]
            });
            object(
                [("type", text("number"))]
                    .into_iter()
                    .chain(range.into_iter().flatten())
                    .chain([("default", value.to_json())]),
            )
        }
    }
}

/// `modeSchema` (`themeFile.ts:114`).
fn mode_schema(mode: ThemeMode, palette: &RolePalette) -> JsonSchema {
    let properties = palette.iter().filter_map(|(role, value)| {
        Some((role.as_str(), role_schema(role_kind(role, value)?, value)))
    });
    object([
        ("type", text("object")),
        (
            "description",
            Json::String(format!(
                "Roles for the {} appearance; a role left out keeps Ardia's value",
                mode.name()
            )),
        ),
        ("properties", object(properties)),
        ("additionalProperties", Json::Bool(false)),
    ])
}

/// `themeSchema` (`themeFile.ts:123`): the JSON Schema of a theme file.
pub fn theme_schema(defaults: &RolePalettes) -> JsonSchema {
    let properties = [
        ("$schema".to_owned(), object([("type", text("string"))])),
        (
            "dark".to_owned(),
            mode_schema(ThemeMode::Dark, &defaults.dark),
        ),
        (
            "light".to_owned(),
            mode_schema(ThemeMode::Light, &defaults.light),
        ),
    ]
    .into_iter()
    .chain(look_schemas());
    object([
        ("$schema", text(JSON_SCHEMA_DIALECT)),
        (
            "description",
            text(
                "An Ardia theme: colour roles that replace Ardia's own, per appearance, and the type, metrics, layout and icons it draws with",
            ),
        ),
        ("type", text("object")),
        ("properties", Json::Object(properties.collect())),
        ("additionalProperties", Json::Bool(false)),
    ])
}
