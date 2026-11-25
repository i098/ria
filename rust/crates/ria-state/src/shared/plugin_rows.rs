// Ported from ardia apps/browser/src/lib/pluginRows.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The settings rows a plugin declares in `plugin.json` (`contributes.settings`): checking them and
//! reading a stored value back as the row's type.
//!
//! Changes from Ardia:
//! - `RowSpec` is a struct with the shared fields and a `RowKind` for the type; the secret row's
//!   `default` is always `""` and is not stored. `RowValue` is an enum.
//! - `unknown` is `Option<&Json>`: `None` is `undefined`.
//! - `json_value` and `fields_value` (not in the TS) write `Json` for serde, for the `Serialize`
//!   derives of the plugin specs.

use super::decode::Fields;
use super::plugin_checks::{
    Check, KEY, KEY_RULE, LIMITS, UNSAFE_TEXT, at, list, matching, object, only, text, unique,
};
use ria_core::identity::telemetry_wire::{Json, js_order};
use serde::{Serialize, Serializer};

/// `ROW_TYPES` (`pluginRows.ts:4`).
pub const ROW_TYPES: [&str; 5] = ["toggle", "text", "number", "choice", "secret"];

/// `RowValue` (`pluginRows.ts:6`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RowValue {
    Text(String),
    Bool(bool),
    Number(f64),
}

/// A choice of a choice row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowChoice {
    pub value: String,
    pub title: String,
}

/// The `type` of a row and the fields that go with it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RowKind {
    Toggle {
        default: bool,
    },
    Text {
        default: String,
    },
    Number {
        default: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
    },
    Choice {
        default: String,
        choices: Vec<RowChoice>,
    },
    Secret {
        #[serde(serialize_with = "empty")]
        default: (),
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },
}

fn empty<S: Serializer>(_: &(), s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("")
}

/// `RowSpec` (`pluginRows.ts:8-15`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowSpec {
    pub key: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub kind: RowKind,
}

impl RowSpec {
    /// The row's `type`.
    pub fn row_type(&self) -> &'static str {
        match self.kind {
            RowKind::Toggle { .. } => "toggle",
            RowKind::Text { .. } => "text",
            RowKind::Number { .. } => "number",
            RowKind::Choice { .. } => "choice",
            RowKind::Secret { .. } => "secret",
        }
    }

    /// The row's `default`.
    pub fn default_json(&self) -> Json {
        match &self.kind {
            RowKind::Toggle { default } => Json::Bool(*default),
            RowKind::Number { default, .. } => Json::Number(*default),
            RowKind::Text { default } | RowKind::Choice { default, .. } => {
                Json::String(default.clone())
            }
            RowKind::Secret { .. } => Json::String(String::new()),
        }
    }
}

/// `SettingsSpec` (`pluginRows.ts:17`).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SettingsSpec {
    #[serde(serialize_with = "fields_value")]
    pub defaults: Fields,
    pub rows: Vec<RowSpec>,
}

/// `Json` as a serde value, object keys in JS order; a number that is not finite is `null`.
pub fn json_value(j: &Json) -> serde_json::Value {
    match j {
        Json::Null => serde_json::Value::Null,
        Json::Bool(b) => serde_json::Value::Bool(*b),
        Json::Number(n) => serde_json::Value::from(*n),
        Json::String(s) => serde_json::Value::String(s.clone()),
        Json::Array(items) => items.iter().map(json_value).collect(),
        Json::Object(entries) => serde_json::Value::Object(
            js_order(entries)
                .into_iter()
                .map(|(k, v)| (k.clone(), json_value(v)))
                .collect(),
        ),
    }
}

/// Serializes `Fields` as the JS object.
pub fn fields_value<S: Serializer>(fields: &Fields, s: S) -> Result<S::Ok, S::Error> {
    json_value(&Json::Object(fields.clone())).serialize(s)
}

const ROW_KEYS: [&str; 10] = [
    "key",
    "title",
    "description",
    "type",
    "default",
    "choices",
    "min",
    "max",
    "step",
    "placeholder",
];
const NUMBER_LIMIT: f64 = 1e12;

fn field<'a>(doc: &'a Fields, key: &str) -> Option<&'a Json> {
    doc.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `isNumber` (`pluginRows.ts:22`): a finite number of at most `NUMBER_LIMIT` in size.
fn is_number(v: Option<&Json>) -> Option<f64> {
    match v {
        Some(&Json::Number(n)) if n.is_finite() && n.abs() <= NUMBER_LIMIT => Some(n),
        _ => None,
    }
}

/// `optionalNumber` (`pluginRows.ts:24`).
fn optional_number(c: &mut Check, path: &str, value: Option<&Json>) -> Option<f64> {
    value?;
    let n = is_number(value);
    if n.is_none() {
        c.problems.push(format!("{path} must be a number"));
    }
    n
}

/// `numberRow` (`pluginRows.ts:31`).
fn number_row(
    c: &mut Check,
    path: &str,
    doc: &Fields,
    key: String,
    title: String,
    description: Option<String>,
) -> Option<RowSpec> {
    let min = optional_number(c, &format!("{path}.min"), field(doc, "min"));
    let max = optional_number(c, &format!("{path}.max"), field(doc, "max"));
    let step = optional_number(c, &format!("{path}.step"), field(doc, "step"));
    if step.is_some_and(|s| s <= 0.0) {
        c.problems.push(format!("{path}.step must be above 0"));
        return None;
    }
    if let (Some(lo), Some(hi)) = (min, max)
        && lo > hi
    {
        c.problems
            .push(format!("{path}.min must be at most its max"));
        return None;
    }
    let default = is_number(field(doc, "default"))
        .filter(|d| min.is_none_or(|lo| *d >= lo) && max.is_none_or(|hi| *d <= hi));
    let Some(default) = default else {
        c.problems.push(format!(
            "{path}.default must be a number between its min and max"
        ));
        return None;
    };
    Some(RowSpec {
        key,
        title,
        description,
        kind: RowKind::Number {
            default,
            min,
            max,
            step,
        },
    })
}

/// `choiceRow` (`pluginRows.ts:42`).
fn choice_row(
    c: &mut Check,
    path: &str,
    doc: &Fields,
    key: String,
    title: String,
    description: Option<String>,
) -> Option<RowSpec> {
    let [least, most] = LIMITS.choices;
    let given = match field(doc, "choices") {
        Some(Json::Array(items)) if items.len() as f64 >= least && items.len() as f64 <= most => {
            items
        }
        _ => {
            c.problems.push(format!(
                "{path}.choices must be a list of {least} to {most} choices"
            ));
            return None;
        }
    };
    let mut choices = Vec::new();
    for (i, choice) in given.iter().enumerate() {
        let p = at(&format!("{path}.choices"), i);
        let Some(fields) = object(c, &p, Some(choice)) else {
            continue;
        };
        only(c, &p, fields, &["value", "title"]);
        let v = matching(
            c,
            &format!("{p}.value"),
            field(fields, "value"),
            &KEY,
            KEY_RULE,
            true,
        );
        let t = text(
            c,
            &format!("{p}.title"),
            field(fields, "title"),
            LIMITS.title,
            true,
        );
        if let (Some(value), Some(title)) = (v, t) {
            choices.push(RowChoice {
                value: value.to_owned(),
                title: title.to_owned(),
            });
        }
    }
    unique(
        c,
        &format!("{path}.choices"),
        choices.iter().map(|x| Some(&x.value)),
        "choices",
    );
    let default = match field(doc, "default") {
        Some(Json::String(d)) if choices.iter().any(|x| &x.value == d) => d.clone(),
        _ => {
            c.problems
                .push(format!("{path}.default must be one of its choices' values"));
            return None;
        }
    };
    Some(RowSpec {
        key,
        title,
        description,
        kind: RowKind::Choice { default, choices },
    })
}

/// `misplaced` (`pluginRows.ts:61`).
fn misplaced(c: &mut Check, path: &str, doc: &Fields, row_type: &str) {
    let belongs: [(&str, &[&str]); 5] = [
        ("choices", &["choice"]),
        ("min", &["number"]),
        ("max", &["number"]),
        ("step", &["number"]),
        ("placeholder", &["secret", "text"]),
    ];
    for (key, types) in belongs {
        if field(doc, key).is_some() && !types.contains(&row_type) {
            c.problems.push(format!(
                "{path}.{key} only goes with type {}",
                types.join(" or ")
            ));
        }
    }
}

/// `row` (`pluginRows.ts:66`).
pub fn row(c: &mut Check, path: &str, value: Option<&Json>) -> Option<RowSpec> {
    let doc = object(c, path, value)?;
    only(c, path, doc, &ROW_KEYS);
    let key = matching(
        c,
        &format!("{path}.key"),
        field(doc, "key"),
        &KEY,
        KEY_RULE,
        true,
    );
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.title,
        true,
    );
    let description = text(
        c,
        &format!("{path}.description"),
        field(doc, "description"),
        LIMITS.description,
        false,
    );
    let row_type = match field(doc, "type") {
        Some(Json::String(t)) if ROW_TYPES.contains(&t.as_str()) => t.as_str(),
        _ => {
            c.problems.push(format!(
                "{path}.type must be toggle, text, number, choice or secret"
            ));
            return None;
        }
    };
    misplaced(c, path, doc, row_type);
    let (key, title) = (key?.to_owned(), title?.to_owned());
    let description = description.map(str::to_owned);
    let default = field(doc, "default");
    let kind = match row_type {
        "toggle" => match default {
            Some(&Json::Bool(default)) => RowKind::Toggle { default },
            _ => {
                c.problems
                    .push(format!("{path}.default must be true or false"));
                return None;
            }
        },
        "text" => match default {
            Some(Json::String(d))
                if d.encode_utf16().count() <= LIMITS.row_text && !UNSAFE_TEXT.is_match(d) =>
            {
                RowKind::Text { default: d.clone() }
            }
            _ => {
                c.problems.push(format!(
                    "{path}.default must be text of at most {} characters",
                    LIMITS.row_text
                ));
                return None;
            }
        },
        "number" => return number_row(c, path, doc, key, title, description),
        "secret" => {
            if default.is_some() {
                c.problems.push(format!("{path}.default can't go with a secret: it's kept in the Keychain, never in plugin.json"));
            }
            let placeholder = text(
                c,
                &format!("{path}.placeholder"),
                field(doc, "placeholder"),
                LIMITS.title,
                false,
            );
            RowKind::Secret {
                default: (),
                placeholder: placeholder.map(str::to_owned),
            }
        }
        _ => return choice_row(c, path, doc, key, title, description),
    };
    Some(RowSpec {
        key,
        title,
        description,
        kind,
    })
}

/// `settingsOf` (`pluginRows.ts:99`).
pub fn settings_of(c: &mut Check, path: &str, value: Option<&Json>) -> SettingsSpec {
    if value.is_none() {
        return SettingsSpec::default();
    }
    let Some(doc) = object(c, path, value) else {
        return SettingsSpec::default();
    };
    only(c, path, doc, &["defaults", "rows"]);
    let defaults = match field(doc, "defaults") {
        None => Fields::new(),
        given => object(c, &format!("{path}.defaults"), given)
            .cloned()
            .unwrap_or_default(),
    };
    let rows_path = format!("{path}.rows");
    let rows: Vec<RowSpec> = list(c, &rows_path, field(doc, "rows"), LIMITS.rows)
        .iter()
        .enumerate()
        .filter_map(|(i, r)| row(c, &at(&rows_path, i), Some(r)))
        .collect();
    unique(c, &rows_path, rows.iter().map(|r| Some(&r.key)), "rows");
    SettingsSpec { defaults, rows }
}

/// `storedRows` (`pluginRows.ts:110`).
pub fn stored_rows(settings: &SettingsSpec) -> Vec<&RowSpec> {
    settings
        .rows
        .iter()
        .filter(|r| !matches!(r.kind, RowKind::Secret { .. }))
        .collect()
}

/// `secretRows` (`pluginRows.ts:112`).
pub fn secret_rows(settings: &SettingsSpec) -> Vec<&RowSpec> {
    settings
        .rows
        .iter()
        .filter(|r| matches!(r.kind, RowKind::Secret { .. }))
        .collect()
}

/// `rowValue` (`pluginRows.ts:114`): `value` when it fits the row, else the row's default.
pub fn row_value(r: &RowSpec, value: Option<&Json>) -> RowValue {
    match (&r.kind, value) {
        (RowKind::Toggle { .. }, Some(&Json::Bool(b))) => RowValue::Bool(b),
        (RowKind::Toggle { default }, _) => RowValue::Bool(*default),
        (RowKind::Text { .. }, Some(Json::String(s)))
            if s.encode_utf16().count() <= LIMITS.row_text && !UNSAFE_TEXT.is_match(s) =>
        {
            RowValue::Text(s.clone())
        }
        (RowKind::Text { default }, _) => RowValue::Text(default.clone()),
        (RowKind::Secret { .. }, _) => RowValue::Text(String::new()),
        (
            RowKind::Number {
                default, min, max, ..
            },
            _,
        ) => RowValue::Number(
            is_number(value)
                .filter(|v| min.is_none_or(|lo| *v >= lo) && max.is_none_or(|hi| *v <= hi))
                .unwrap_or(*default),
        ),
        (RowKind::Choice { choices, .. }, Some(Json::String(s)))
            if choices.iter().any(|x| &x.value == s) =>
        {
            RowValue::Text(s.clone())
        }
        (RowKind::Choice { default, .. }, _) => RowValue::Text(default.clone()),
    }
}
