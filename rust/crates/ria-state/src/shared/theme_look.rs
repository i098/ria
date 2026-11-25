// Ported from ardia apps/browser/src/lib/themeLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The look blocks of a theme file (`type`, `metrics`, `layout`, `icons`): their specs, the checks
//! that report each bad value, the look they resolve to, and their JSON Schema and defaults.
//!
//! Changes from Ardia:
//! - Untyped values (`unknown`, `Fields`) are `ria_core` `Json`, which has no `undefined`: a value
//!   left out is `None`. `LookBlocks` is a struct of the four optional blocks.
//! - The keyed constants (`TYPE_SIZES`, `WEIGHTS`, `RADII`, `THEME_RADII`) are lists of
//!   `(name, value)` in `Object.entries` order; `ThemeLook.sizes` and `ThemeLook.radii` are arrays in
//!   the order of `TYPE_SIZES` and `RADII`. `ROW_METRICS` is a `RowMetrics`.
//! - `ThemeLook.weights` and `ThemeLook.icons` are `Json` entries: `resolveLook` spreads whatever
//!   the blocks hold, unchecked. `ThemeLook.hidden` is the `Set` of the `layout.hide` items, as a
//!   list without repeats (`SameValueZero`; arrays and objects are never the same).
//! - `DEFAULT_LOOK` is a `LazyLock`; `withSidebarText` gives an equal look, not the same cached
//!   object (not ported: the `WeakMap` cache, which only keeps object identity).
//! - `FAMILY_PATTERN` and `SYMBOL_PATTERN` are checked by hand, on UTF-16 code units as JavaScript
//!   does (`.` and `\S` match one code unit).
//! - `checked` reports through `&mut dyn FnMut(String)` and gives `None` for `undefined`.

use std::sync::LazyLock;

use ria_core::identity::telemetry_wire::{Json, js_number_to_string, js_order, set_entry};

use super::decode::is_fields;
use super::js::{is_js_space, js_round};
use super::json_schema::JsonSchema;
use super::window_layout::DEFAULT_SIDEBAR_WIDTH;

/// `TYPE_SIZES` (`themeLook.ts:5`).
pub const TYPE_SIZES: [(&str, f64); 7] = [
    ("badge", 9.0),
    ("tiny", 10.0),
    ("caption", 11.0),
    ("small", 12.0),
    ("body", 13.0),
    ("title", 15.0),
    ("large", 17.0),
];
/// `WEIGHTS` (`themeLook.ts:6`).
pub const WEIGHTS: [(&str, &str); 4] = [
    ("regular", "400"),
    ("medium", "500"),
    ("semibold", "600"),
    ("bold", "700"),
];
/// `RADII` (`themeLook.ts:7`).
pub const RADII: [(&str, f64); 10] = [
    ("none", 0.0),
    ("hair", 2.0),
    ("control", 4.0),
    ("tile", 8.0),
    ("panel", 12.0),
    ("sheet", 16.0),
    ("hero", 20.0),
    ("chrome", 10.0),
    ("toolbar", 7.0),
    ("pinned", 10.0),
];

/// `RowMetrics` (`themeLook.ts:18`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowMetrics {
    pub row_height: f64,
    pub row_gap: f64,
    pub pinned_height: f64,
}

/// `ROW_METRICS` (`themeLook.ts:8`).
pub const ROW_METRICS: RowMetrics = RowMetrics {
    row_height: 34.0,
    row_gap: 3.0,
    pinned_height: 41.0,
};
/// `MONO_FAMILY` (`themeLook.ts:9`).
pub const MONO_FAMILY: &str = "Menlo";

/// `SidebarTextSize` (`themeLook.ts:11`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarTextSize {
    Default,
    Large,
    Larger,
}

/// `SIDEBAR_TEXT_SIZES` (`themeLook.ts:10`).
pub const SIDEBAR_TEXT_SIZES: [SidebarTextSize; 3] = [
    SidebarTextSize::Default,
    SidebarTextSize::Large,
    SidebarTextSize::Larger,
];

impl SidebarTextSize {
    /// The JS name of the size.
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Large => "large",
            Self::Larger => "larger",
        }
    }

    /// The size of a JS name; `None` for any other string.
    pub fn from_name(name: &str) -> Option<Self> {
        SIDEBAR_TEXT_SIZES.into_iter().find(|s| s.name() == name)
    }

    /// `SIDEBAR_TEXT_SCALE[size]` (`themeLook.ts:12`).
    fn scale(self) -> f64 {
        match self {
            Self::Default => 1.0,
            Self::Large => 15.0 / 13.0,
            Self::Larger => 17.0 / 13.0,
        }
    }
}

/// `CHROME_PARTS` (`themeLook.ts:20`).
pub const CHROME_PARTS: [&str; 7] = [
    "pinned",
    "newTab",
    "addressField",
    "navButtons",
    "sidebarButton",
    "downloads",
    "profile",
];

/// `Place` (`themeLook.ts:23`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Top,
    Bottom,
}

/// `PLACES` (`themeLook.ts:22`).
pub const PLACES: [Place; 2] = [Place::Top, Place::Bottom];

impl Place {
    /// The JS name of the place.
    pub fn name(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
}

/// `THEME_RADII` (`themeLook.ts:25`): a theme's radius key and the `RADII` token it sets.
pub const THEME_RADII: [(&str, &str); 7] = [
    ("row", "chrome"),
    ("pinned", "pinned"),
    ("tile", "tile"),
    ("control", "control"),
    ("toolbar", "toolbar"),
    ("panel", "panel"),
    ("sheet", "sheet"),
];

/// `ThemeLook` (`themeLook.ts:28`).
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeLook {
    pub family: Option<String>,
    pub mono: String,
    /// In `TYPE_SIZES` order.
    pub sizes: [f64; 7],
    pub weights: Vec<(String, Json)>,
    pub metrics: RowMetrics,
    pub sidebar_width: Option<f64>,
    pub sidebar_scale: f64,
    /// In `RADII` order.
    pub radii: [f64; 10],
    pub hidden: Vec<Json>,
    pub address_field: Place,
    pub new_tab: Option<Place>,
    pub icons: Vec<(String, Json)>,
}

/// `LookBlocks` (`themeLook.ts:43`): `None` is a block left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LookBlocks {
    pub type_: Option<Json>,
    pub metrics: Option<Json>,
    pub layout: Option<Json>,
    pub icons: Option<Json>,
}

impl LookBlocks {
    /// `blocks[key]`.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match key {
            "type" => self.type_.as_ref(),
            "metrics" => self.metrics.as_ref(),
            "layout" => self.layout.as_ref(),
            "icons" => self.icons.as_ref(),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: Json) {
        match key {
            "type" => self.type_ = Some(value),
            "metrics" => self.metrics = Some(value),
            "layout" => self.layout = Some(value),
            _ => self.icons = Some(value),
        }
    }
}

const FONT_WEIGHTS: [&str; 9] = [
    "100", "200", "300", "400", "500", "600", "700", "800", "900",
];
/// `SYMBOL_PATTERN` (`themeLook.ts:47`).
pub const SYMBOL_PATTERN: &str = "^[a-z0-9]+(\\.[a-z0-9]+)*$";
/// `FAMILY_PATTERN` (`themeLook.ts:49`).
pub const FAMILY_PATTERN: &str = "^\\S(.{0,62}\\S)?$";

/// `SYMBOL.test(name)` (`themeLook.ts:48`).
fn is_symbol(name: &str) -> bool {
    name.split('.').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    })
}

/// `FAMILY.test(value)` (`themeLook.ts:50`), on UTF-16 code units.
fn is_family(value: &str) -> bool {
    let units: Vec<u16> = value.encode_utf16().collect();
    // A lone surrogate half is `\S` and `.`.
    let space = |u: u16| char::from_u32(u32::from(u)).is_some_and(is_js_space);
    let line_end = |u: u16| matches!(u, 0x0a | 0x0d | 0x2028 | 0x2029);
    match units.as_slice() {
        [] => false,
        [one] => !space(*one),
        [first, middle @ .., last] => {
            !space(*first)
                && !space(*last)
                && middle.len() <= 62
                && !middle.iter().any(|u| line_end(*u))
        }
    }
}

/// `Spec` (`themeLook.ts:52`).
#[derive(Clone, Debug, PartialEq)]
pub enum Spec {
    Number {
        min: f64,
        max: f64,
        default: Option<f64>,
        about: String,
    },
    Family {
        default: Option<&'static str>,
        about: String,
    },
    Weight {
        default: &'static str,
        about: String,
    },
    Place {
        default: Option<Place>,
        about: String,
    },
    Parts {
        about: String,
    },
    Symbols {
        about: String,
    },
    Block {
        noun: &'static str,
        keys: Vec<(&'static str, Spec)>,
        about: String,
    },
}

impl Spec {
    fn about(&self) -> &str {
        match self {
            Spec::Number { about, .. }
            | Spec::Family { about, .. }
            | Spec::Weight { about, .. }
            | Spec::Place { about, .. }
            | Spec::Parts { about }
            | Spec::Symbols { about }
            | Spec::Block { about, .. } => about,
        }
    }
}

/// `number` (`themeLook.ts:61`).
fn number(min: f64, max: f64, value: Option<f64>, about: &str) -> Spec {
    Spec::Number {
        min,
        max,
        default: value,
        about: about.into(),
    }
}

/// `sizeRange` (`themeLook.ts:62`).
fn size_range(size: f64, about: &str) -> Spec {
    number(
        js_round(size * 0.75),
        js_round(size * 1.6),
        Some(size),
        about,
    )
}

/// `SIZE_ABOUT` (`themeLook.ts:64`), in `TYPE_SIZES` order.
const SIZE_ABOUT: [&str; 7] = [
    "Counts on badges",
    "The smallest labels",
    "Captions, section labels, URLs under titles",
    "Sidebar tab titles, secondary text",
    "Most text: menus, fields, folder and group names",
    "Panel and dialog titles, the command bar's results",
    "Large headings, the command bar's field",
];

/// `WEIGHT_ABOUT` (`themeLook.ts:74`), in `WEIGHTS` order.
const WEIGHT_ABOUT: [&str; 4] = [
    "What regular text draws with",
    "What medium text (group names, selected rows) draws with",
    "What semibold text (headings, badges) draws with",
    "What bold text draws with",
];

/// `RADIUS_ABOUT` (`themeLook.ts:81`), in `THEME_RADII` order.
const RADIUS_ABOUT: [&str; 7] = [
    "Sidebar rows (tabs, folders, New Tab), the sidebar address field, top-strip tabs",
    "Pinned tiles in the sidebar's top apps grid",
    "List rows, menu rows, fields, cards and icon tiles",
    "Small controls: steppers, chips, keys",
    "Toolbar button plates",
    "Popovers, menus and panels",
    "Sheets, the tab switcher, search and other large panels",
];

/// `RADII[token]`.
fn radius_of(token: &str) -> f64 {
    RADII
        .iter()
        .find(|(name, _)| *name == token)
        .map_or(f64::NAN, |(_, v)| *v)
}

/// `BLOCKS` (`themeLook.ts:91`), in `LOOK_KEYS` order.
pub static BLOCKS: LazyLock<[(&str, Spec); 4]> = LazyLock::new(|| {
    let block = |noun, about: &str, keys| Spec::Block {
        noun,
        keys,
        about: about.into(),
    };
    let sizes = TYPE_SIZES
        .iter()
        .zip(SIZE_ABOUT)
        .map(|((k, v), about)| (*k, size_range(*v, about)));
    let weights = WEIGHTS.iter().zip(WEIGHT_ABOUT).map(|((k, v), about)| {
        (
            *k,
            Spec::Weight {
                default: v,
                about: about.into(),
            },
        )
    });
    let radii = THEME_RADII
        .iter()
        .zip(RADIUS_ABOUT)
        .map(|((k, token), about)| {
            let max = match *k {
                "sheet" => 32.0,
                "panel" => 28.0,
                "control" => 12.0,
                _ => 24.0,
            };
            (*k, number(0.0, max, Some(radius_of(token)), about))
        });
    [
        (
            "type",
            block("type", "Fonts and text sizes", vec![
                ("family", Spec::Family {
                    default: None,
                    about: "The interface font family, like \"Avenir Next\"; left out, the system font. A family macOS doesn't have falls back to the system font".into(),
                }),
                ("mono", Spec::Family {
                    default: Some(MONO_FAMILY),
                    about: "The monospaced family for codes, keys and logs".into(),
                }),
                ("scale", number(0.85, 1.3, Some(1.0), "Multiplies every text size, after sizes")),
                ("sizes", block("text size", "Text sizes in points, each replacing one step of Ardia's scale", sizes.collect())),
                (
                    "weights",
                    block("weight", "What each of Ardia's weight names draws with, from \"100\" (thin) to \"900\" (black)", weights.collect()),
                ),
            ]),
        ),
        (
            "metrics",
            block("metrics", "Sizes and corner radii, in points", vec![
                (
                    "rowHeight",
                    number(24.0, 56.0, Some(ROW_METRICS.row_height), "Height of a sidebar row: tabs, folders, groups, the New Tab row and the sidebar address field"),
                ),
                ("rowGap", number(0.0, 12.0, Some(ROW_METRICS.row_gap), "Space between sidebar rows")),
                (
                    "pinnedHeight",
                    number(28.0, 72.0, Some(ROW_METRICS.pinned_height), "Height of a pinned tile in the sidebar's top apps grid"),
                ),
                (
                    "sidebarWidth",
                    number(160.0, 400.0, None, &format!(
                        "The sidebar's width. The sidebar takes it when you switch to the theme or change this value, and a double-click on its edge returns to it; left out, the width is yours (Ardia's default is {DEFAULT_SIDEBAR_WIDTH})"
                    )),
                ),
                ("radius", block("radius", "Corner radii", radii.collect())),
            ]),
        ),
        (
            "layout",
            block("layout", "Which parts of the sidebar show, and where", vec![
                ("hide", Spec::Parts { about: "Parts of the sidebar to hide".into() }),
                ("addressField", Spec::Place {
                    default: Some(Place::Top),
                    about: "Where the sidebar address field sits when the address bar is in the sidebar: top or bottom".into(),
                }),
                ("newTab", Spec::Place {
                    default: None,
                    about: "Where the New Tab row sits: top or bottom of the tab list; left out, Settings › Tabs decides".into(),
                }),
            ]),
        ),
        ("icons", Spec::Symbols { about: "SF Symbols to draw in place of Ardia's, by name: { \"plus\": \"plus.circle\" }".into() }),
    ]
});

/// `LOOK_KEYS` (`themeLook.ts:146`).
pub const LOOK_KEYS: [&str; 4] = ["type", "metrics", "layout", "icons"];

/// `list` (`themeLook.ts:148`).
fn list<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    names.into_iter().collect::<Vec<_>>().join(", ")
}

/// `ruleOf` (`themeLook.ts:149`).
fn rule_of(spec: &Spec) -> String {
    match spec {
        Spec::Number { min, max, .. } => {
            format!(
                "must be a number from {} to {}",
                js_number_to_string(*min),
                js_number_to_string(*max)
            )
        }
        Spec::Family { .. } => {
            "must be a font family name (1 to 64 characters, no space at either end)".into()
        }
        Spec::Weight { .. } => "must be a weight from \"100\" to \"900\" in steps of 100".into(),
        Spec::Place { .. } => format!("must be one of {}", list(PLACES.map(Place::name))),
        Spec::Parts { .. } => format!("must be a list of parts ({})", list(CHROME_PARTS)),
        Spec::Symbols { .. } => "must be an object of SF Symbol names".into(),
        Spec::Block { .. } => "must be an object".into(),
    }
}

/// `Object.entries(value)` of an object, in JavaScript order.
fn entries(value: &Json) -> Vec<(String, Json)> {
    match value {
        Json::Object(fields) => js_order(fields).into_iter().cloned().collect(),
        _ => Vec::new(),
    }
}

/// `checked` (`themeLook.ts:168`): the value, cleaned, when it meets `spec`; `None` (`undefined`)
/// after a report when it does not.
fn checked(spec: &Spec, value: &Json, path: &str, report: &mut dyn FnMut(String)) -> Option<Json> {
    let bad = |report: &mut dyn FnMut(String)| {
        report(format!("{path} {}", rule_of(spec)));
        None
    };
    match spec {
        Spec::Number { min, max, .. } => match value {
            Json::Number(n) if n.is_finite() && n >= min && n <= max => Some(value.clone()),
            _ => bad(report),
        },
        Spec::Family { .. } => match value {
            Json::String(s) if is_family(s) => Some(value.clone()),
            _ => bad(report),
        },
        Spec::Weight { .. } => {
            let weight = match value {
                Json::Number(n) => Some(js_number_to_string(*n)),
                Json::String(s) => Some(s.clone()),
                _ => None,
            };
            match weight {
                Some(w) if FONT_WEIGHTS.contains(&w.as_str()) => Some(Json::String(w)),
                _ => bad(report),
            }
        }
        Spec::Place { .. } => match value {
            Json::String(s) if PLACES.iter().any(|p| p.name() == s) => Some(value.clone()),
            _ => bad(report),
        },
        Spec::Parts { .. } => {
            let Json::Array(items) = value else {
                return bad(report);
            };
            let mut parts = Vec::new();
            for part in items {
                match part {
                    Json::String(s) if CHROME_PARTS.contains(&s.as_str()) => {
                        parts.push(part.clone())
                    }
                    _ => report(format!(
                        "{path} has {}, which isn't a part ({})",
                        part.stringify(),
                        list(CHROME_PARTS)
                    )),
                }
            }
            Some(Json::Array(parts))
        }
        Spec::Symbols { .. } => {
            if !is_fields(value) {
                return bad(report);
            }
            let mut icons = Vec::new();
            for (name, replacement) in entries(value) {
                if !is_symbol(&name) {
                    report(format!(
                        "{path} has {}, which isn't an SF Symbol name (lowercase letters, digits and dots)",
                        Json::String(name).stringify()
                    ));
                } else if !matches!(&replacement, Json::String(r) if is_symbol(r)) {
                    report(format!(
                        "{path}.{name} must be an SF Symbol name (lowercase letters, digits and dots)"
                    ));
                } else {
                    icons.push((name, replacement));
                }
            }
            Some(Json::Object(icons))
        }
        Spec::Block { noun, keys, .. } => {
            if !is_fields(value) {
                return bad(report);
            }
            let mut out = Vec::new();
            for (key, given) in entries(value) {
                let Some((_, sub)) = keys.iter().find(|(k, _)| *k == key) else {
                    report(format!(
                        "{path}.{key} isn't a {noun} key ({})",
                        list(keys.iter().map(|(k, _)| *k))
                    ));
                    continue;
                };
                if let Some(ok) = checked(sub, &given, &format!("{path}.{key}"), report) {
                    out.push((key, ok));
                }
            }
            Some(Json::Object(out))
        }
    }
}

/// `parseLookBlocks` (`themeLook.ts:221`): the look blocks of a parsed theme file `doc`; each problem
/// goes to `problems`, prefixed with `file`.
pub fn parse_look_blocks(
    file: &str,
    doc: &[(String, Json)],
    problems: &mut Vec<String>,
) -> LookBlocks {
    let mut blocks = LookBlocks::default();
    for (key, spec) in BLOCKS.iter() {
        let Some((_, given)) = doc.iter().find(|(k, _)| k == key) else {
            continue;
        };
        let mut report = |problem: String| problems.push(format!("{file}: {problem}"));
        if let Some(value) = checked(spec, given, key, &mut report) {
            blocks.set(key, value);
        }
    }
    blocks
}

/// `fieldsOf` (`themeLook.ts:231`).
fn fields_of(value: Option<&Json>) -> &[(String, Json)] {
    match value {
        Some(Json::Object(fields)) => fields,
        _ => &[],
    }
}

/// `fields[key]`.
fn field<'a>(fields: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `hasContent` (`themeLook.ts:232`): `None` is `undefined`.
fn has_content(value: Option<&Json>) -> bool {
    match value {
        Some(Json::Object(fields)) => fields.iter().any(|(_, v)| has_content(Some(v))),
        Some(Json::Array(items)) => !items.is_empty(),
        Some(_) => true,
        None => false,
    }
}

/// `numberOr` (`themeLook.ts:233`): any number, `NaN` too.
fn number_or(value: Option<&Json>, fallback: f64) -> f64 {
    match value {
        Some(Json::Number(n)) => *n,
        _ => fallback,
    }
}

/// `round` (`themeLook.ts:234`): to hundredths.
fn round(value: f64) -> f64 {
    js_round(value * 100.0) / 100.0
}

/// `DEFAULT_LOOK` (`themeLook.ts:239`).
pub static DEFAULT_LOOK: LazyLock<ThemeLook> = LazyLock::new(|| ThemeLook {
    family: None,
    mono: MONO_FAMILY.into(),
    sizes: TYPE_SIZES.map(|(_, v)| v),
    weights: WEIGHTS
        .iter()
        .map(|(k, v)| ((*k).into(), Json::String((*v).into())))
        .collect(),
    metrics: ROW_METRICS,
    sidebar_width: None,
    sidebar_scale: 1.0,
    radii: RADII.map(|(_, v)| v),
    hidden: Vec::new(),
    address_field: Place::Top,
    new_tab: None,
    icons: Vec::new(),
});

/// SameValueZero for `Set`: arrays and objects are never the same.
fn same_value_zero(a: &Json, b: &Json) -> bool {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => x == y || (x.is_nan() && y.is_nan()),
        (Json::Array(_) | Json::Object(_), _) => false,
        _ => a == b,
    }
}

/// `Object.entries(v)` for `blocks.icons ?? {}`: a string gives its UTF-16 code units, an array its items.
fn spread_entries(value: Option<&Json>) -> Vec<(String, Json)> {
    match value {
        Some(Json::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        Some(Json::String(s)) => s
            .encode_utf16()
            .enumerate()
            .map(|(i, u)| (i.to_string(), Json::String(String::from_utf16_lossy(&[u]))))
            .collect(),
        Some(v) => entries(v),
        None => Vec::new(),
    }
}

/// `resolveLook` (`themeLook.ts:254`): the look the blocks give, `DEFAULT_LOOK` when they hold nothing.
pub fn resolve_look(blocks: Option<&LookBlocks>) -> ThemeLook {
    let Some(blocks) = blocks.filter(|b| LOOK_KEYS.iter().any(|key| has_content(b.get(key))))
    else {
        return DEFAULT_LOOK.clone();
    };
    let type_ = fields_of(blocks.type_.as_ref());
    let metrics = fields_of(blocks.metrics.as_ref());
    let layout = fields_of(blocks.layout.as_ref());
    let scale = number_or(field(type_, "scale"), 1.0);
    let sizes = fields_of(field(type_, "sizes"));
    let weights = fields_of(field(type_, "weights"));
    let radius = fields_of(field(metrics, "radius"));
    let mut radii = RADII.map(|(_, v)| v);
    for (key, token) in THEME_RADII {
        let at = RADII
            .iter()
            .position(|(name, _)| *name == token)
            .unwrap_or_default();
        radii[at] = number_or(field(radius, key), RADII[at].1);
    }
    let mut all_weights = DEFAULT_LOOK.weights.clone();
    for (k, v) in js_order(weights) {
        set_entry(&mut all_weights, k, v.clone());
    }
    let string = |v: Option<&Json>| match v {
        Some(Json::String(s)) => Some(s.clone()),
        _ => None,
    };
    let place = |v: Option<&Json>| match v {
        Some(Json::String(s)) if s == "top" => Some(Place::Top),
        Some(Json::String(s)) if s == "bottom" => Some(Place::Bottom),
        _ => None,
    };
    let mut hidden: Vec<Json> = Vec::new();
    if let Some(Json::Array(items)) = field(layout, "hide") {
        for item in items {
            if !hidden.iter().any(|h| same_value_zero(h, item)) {
                hidden.push(item.clone());
            }
        }
    }
    ThemeLook {
        family: string(field(type_, "family")),
        mono: string(field(type_, "mono")).unwrap_or_else(|| MONO_FAMILY.into()),
        sizes: std::array::from_fn(|i| {
            round(number_or(field(sizes, TYPE_SIZES[i].0), TYPE_SIZES[i].1) * scale)
        }),
        weights: all_weights,
        metrics: RowMetrics {
            row_height: number_or(field(metrics, "rowHeight"), ROW_METRICS.row_height),
            row_gap: number_or(field(metrics, "rowGap"), ROW_METRICS.row_gap),
            pinned_height: number_or(field(metrics, "pinnedHeight"), ROW_METRICS.pinned_height),
        },
        sidebar_width: match field(metrics, "sidebarWidth") {
            Some(Json::Number(n)) => Some(*n),
            _ => None,
        },
        sidebar_scale: 1.0,
        radii,
        hidden,
        address_field: if place(field(layout, "addressField")) == Some(Place::Bottom) {
            Place::Bottom
        } else {
            Place::Top
        },
        new_tab: place(field(layout, "newTab")),
        icons: spread_entries(blocks.icons.as_ref().filter(|v| !matches!(v, Json::Null))),
    }
}

/// `withSidebarText` (`themeLook.ts:287`): the look with its sidebar rows scaled for the text size.
pub fn with_sidebar_text(look: &ThemeLook, size: SidebarTextSize) -> ThemeLook {
    let scale = size.scale();
    if scale == 1.0 {
        return look.clone();
    }
    let m = look.metrics;
    ThemeLook {
        sidebar_scale: scale,
        metrics: RowMetrics {
            row_height: js_round(m.row_height * scale),
            row_gap: js_round(m.row_gap * scale),
            pinned_height: js_round(m.pinned_height * scale),
        },
        ..look.clone()
    }
}

/// `LookUse` (`themeLook.ts:301`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LookUse {
    pub type_: bool,
    pub metrics: bool,
    pub layout: bool,
    pub icons: bool,
    pub hidden: usize,
    pub moved: usize,
    pub icon_overrides: usize,
}

/// `lookUse` (`themeLook.ts:303`): which blocks a theme sets, and how many parts it hides, moves and
/// redraws.
pub fn look_use(blocks: Option<&LookBlocks>) -> LookUse {
    let look = resolve_look(blocks);
    let has = |key: &str| has_content(blocks.and_then(|b| b.get(key)));
    LookUse {
        type_: has("type"),
        metrics: has("metrics"),
        layout: has("layout"),
        icons: has("icons"),
        hidden: look.hidden.len(),
        moved: usize::from(look.address_field != DEFAULT_LOOK.address_field)
            + usize::from(look.new_tab.is_some()),
        icon_overrides: look.icons.len(),
    }
}

/// An object from `(key, value)` pairs.
fn object<'a>(fields: impl IntoIterator<Item = (&'a str, Json)>) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn text(s: &str) -> Json {
    Json::String(s.into())
}

/// `specSchema` (`themeLook.ts:317`).
fn spec_schema(spec: &Spec) -> JsonSchema {
    let about = ("description", text(spec.about()));
    match spec {
        Spec::Number {
            min, max, default, ..
        } => object(
            [
                ("type", text("number")),
                ("minimum", Json::Number(*min)),
                ("maximum", Json::Number(*max)),
            ]
            .into_iter()
            .chain(default.map(|d| ("default", Json::Number(d))))
            .chain([about]),
        ),
        Spec::Family { default, .. } => object(
            [("type", text("string")), ("pattern", text(FAMILY_PATTERN))]
                .into_iter()
                .chain(default.map(|d| ("default", text(d))))
                .chain([about]),
        ),
        Spec::Weight { default, .. } => {
            let numbers = FONT_WEIGHTS
                .iter()
                .map(|w| Json::Number(w.parse().unwrap_or(f64::NAN)));
            let all = FONT_WEIGHTS.iter().map(|w| text(w)).chain(numbers);
            object([
                ("enum", Json::Array(all.collect())),
                ("default", text(default)),
                about,
            ])
        }
        Spec::Place { default, .. } => object(
            [
                ("type", text("string")),
                (
                    "enum",
                    Json::Array(PLACES.iter().map(|p| text(p.name())).collect()),
                ),
            ]
            .into_iter()
            .chain(default.map(|d| ("default", text(d.name()))))
            .chain([about]),
        ),
        Spec::Parts { .. } => object([
            ("type", text("array")),
            (
                "items",
                object([
                    ("type", text("string")),
                    ("enum", Json::Array(CHROME_PARTS.map(text).to_vec())),
                ]),
            ),
            ("default", Json::Array(Vec::new())),
            about,
        ]),
        Spec::Symbols { .. } => {
            let symbol = || object([("type", text("string")), ("pattern", text(SYMBOL_PATTERN))]);
            object([
                ("type", text("object")),
                ("propertyNames", symbol()),
                ("additionalProperties", symbol()),
                ("default", Json::Object(Vec::new())),
                about,
            ])
        }
        Spec::Block { keys, .. } => object([
            ("type", text("object")),
            (
                "properties",
                object(keys.iter().map(|(k, sub)| (*k, spec_schema(sub)))),
            ),
            ("additionalProperties", Json::Bool(false)),
            about,
        ]),
    }
}

/// `lookSchemas` (`themeLook.ts:337`): the JSON Schema of each look block, in `LOOK_KEYS` order.
pub fn look_schemas() -> Vec<(String, JsonSchema)> {
    BLOCKS
        .iter()
        .map(|(key, spec)| ((*key).into(), spec_schema(spec)))
        .collect()
}

/// `lookDefaults` (`themeLook.ts:341`): the default of every look key; `icons` is empty.
pub fn look_defaults() -> LookBlocks {
    fn defaults(spec: &Spec) -> Option<Json> {
        match spec {
            Spec::Block { keys, .. } => Some(Json::Object(
                keys.iter()
                    .filter_map(|(k, sub)| Some(((*k).into(), defaults(sub)?)))
                    .collect(),
            )),
            Spec::Parts { .. } => Some(Json::Array(Vec::new())),
            Spec::Symbols { .. } => Some(Json::Object(Vec::new())),
            Spec::Number { default, .. } => default.map(Json::Number),
            Spec::Family { default, .. } => default.map(text),
            Spec::Weight { default, .. } => Some(text(default)),
            Spec::Place { default, .. } => default.map(|p| text(p.name())),
        }
    }
    let block = |i: usize| defaults(&BLOCKS[i].1);
    LookBlocks {
        type_: block(0),
        metrics: block(1),
        layout: block(2),
        icons: Some(Json::Object(Vec::new())),
    }
}
