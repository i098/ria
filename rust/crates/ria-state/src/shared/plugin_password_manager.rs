// Ported from ardia apps/browser/src/lib/pluginPasswordManager.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The check of a plugin's `passwordManager` entry: which fields it fills, and with what.
//!
//! Changes from Ardia:
//! - `unknown` is `Option<&Json>`: `None` is `undefined`, `Some(&Json::Null)` is `null`.
//! - The `"system"` and field literals are `&'static str`; `verificationCodeSetup` is `verification_code_setup`.
//! - Keys are read in `Object.keys` order (`js_order`); a key read by name reads its first entry.

use ria_core::identity::telemetry_wire::{Json, js_order};
use serde::Serialize;

use super::decode::Fields;

/// `PASSWORD_FIELDS` (`pluginPasswordManager.ts:3`).
pub const PASSWORD_FIELDS: [&str; 2] = ["login", "oneTimeCode"];
/// `PasswordField` (`pluginPasswordManager.ts:4`): one of [`PASSWORD_FIELDS`].
pub type PasswordField = &'static str;
/// `PASSWORD_FILLS` (`pluginPasswordManager.ts:5`).
pub const PASSWORD_FILLS: [&str; 1] = ["system"];
/// `RESERVED_PASSWORD_FILLS` (`pluginPasswordManager.ts:6`).
pub const RESERVED_PASSWORD_FILLS: [&str; 1] = ["tool"];
/// `PASSWORD_FIELD_COUNT` (`pluginPasswordManager.ts:7`).
pub const PASSWORD_FIELD_COUNT: [usize; 2] = [1, 2];

/// `PasswordManagerSpec` (`pluginPasswordManager.ts:8`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordManagerSpec {
    pub fill: &'static str,
    pub fields: Vec<PasswordField>,
    pub verification_code_setup: Option<&'static str>,
    pub passkeys: Option<&'static str>,
}

const KEYS: [&str; 4] = ["fill", "fields", "verificationCodeSetup", "passkeys"];
const PATH: &str = "passwordManager";

fn is_system(value: Option<&Json>) -> bool {
    matches!(value, Some(Json::String(s)) if s == "system")
}

/// `systemOnly` (`pluginPasswordManager.ts:13`).
fn system_only(
    problems: &mut Vec<String>,
    key: &str,
    value: Option<&Json>,
) -> Option<&'static str> {
    value?;
    if is_system(value) {
        return Some("system");
    }
    problems.push(format!("{PATH}.{key} must be system"));
    None
}

/// `fillOf` (`pluginPasswordManager.ts:20`).
fn fill_of(problems: &mut Vec<String>, value: Option<&Json>) -> Option<&'static str> {
    if is_system(value) {
        return Some("system");
    }
    match value {
        Some(Json::String(s)) if RESERVED_PASSWORD_FILLS.contains(&s.as_str()) => {
            problems.push(format!("{PATH}.fill: {s} isn't available yet; use system"));
        }
        _ => problems.push(format!("{PATH}.fill must be system")),
    }
    None
}

/// `fieldsOf` (`pluginPasswordManager.ts:27`).
fn fields_of(problems: &mut Vec<String>, value: Option<&Json>) -> Vec<PasswordField> {
    let [least, most] = PASSWORD_FIELD_COUNT;
    let items = match value {
        Some(Json::Array(items)) if (least..=most).contains(&items.len()) => items,
        _ => {
            problems.push(format!(
                "{PATH}.fields must be a list of {least} to {most} of {}",
                PASSWORD_FIELDS.join(", ")
            ));
            return Vec::new();
        }
    };
    let mut fields: Vec<PasswordField> = Vec::new();
    for (i, f) in items.iter().enumerate() {
        let known = match f {
            Json::String(s) => PASSWORD_FIELDS.iter().find(|p| **p == s.as_str()),
            _ => None,
        };
        match known {
            None => problems.push(format!(
                "{PATH}.fields[{i}] must be {}",
                PASSWORD_FIELDS.join(" or ")
            )),
            Some(p) if fields.contains(p) => problems.push(format!("{PATH}.fields has {p} twice")),
            Some(p) => fields.push(p),
        }
    }
    fields
}

/// `doc[key]`.
fn field<'a>(doc: &'a Fields, key: &str) -> Option<&'a Json> {
    doc.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `passwordManagerOf` (`pluginPasswordManager.ts:42`).
pub fn password_manager_of(
    problems: &mut Vec<String>,
    value: Option<&Json>,
) -> Option<PasswordManagerSpec> {
    let doc = match value? {
        Json::Object(doc) => doc,
        _ => {
            problems.push(format!("{PATH} must be an object"));
            return None;
        }
    };
    for (key, _) in js_order(doc) {
        if !KEYS.contains(&key.as_str()) {
            problems.push(format!("{PATH}.{key} isn't a key of {PATH}"));
        }
    }
    let fill = fill_of(problems, field(doc, "fill"));
    let fields = fields_of(problems, field(doc, "fields"));
    let verification_code_setup = system_only(
        problems,
        "verificationCodeSetup",
        field(doc, "verificationCodeSetup"),
    );
    let passkeys = system_only(problems, "passkeys", field(doc, "passkeys"));
    match fill {
        Some(fill) if !fields.is_empty() => Some(PasswordManagerSpec {
            fill,
            fields,
            verification_code_setup,
            passkeys,
        }),
        _ => None,
    }
}
