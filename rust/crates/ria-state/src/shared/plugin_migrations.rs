// Ported from ardia apps/browser/src/lib/pluginMigrations.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Upgrades an older `plugin.json` to the current form, and words a problem in the v1 field names.
//!
//! Changes from Ardia:
//! - `MANIFEST_MIGRATIONS` is `manifest_migrations()`, since a `Migrations` map holds boxed
//!   closures and cannot be a `const`.

use super::decode::Fields;
use super::plugin_contributions::PAGE_PANEL_ID;
use super::plugin_permissions::legacy_permissions;
use super::saved_document::Migrations;
use regex::Regex;
use ria_core::identity::telemetry_wire::{Json, set_entry};
use std::sync::LazyLock;

/// `MOVED_INTO_CONTRIBUTES` (`pluginMigrations.ts:6`).
const MOVED_INTO_CONTRIBUTES: [&str; 6] = [
    "commands",
    "tools",
    "hooks",
    "settings",
    "programs",
    "passwordManager",
];

/// `pageAsPanel` (`pluginMigrations.ts:8`).
fn page_as_panel(page: Json) -> Json {
    let Json::Object(page) = page else {
        return page;
    };
    let mut out = vec![
        ("id".to_owned(), Json::String(PAGE_PANEL_ID.to_owned())),
        ("mode".to_owned(), Json::String("tab".to_owned())),
    ];
    for (k, v) in page {
        set_entry(&mut out, &k, v);
    }
    Json::Object(out)
}

/// Takes `key` out of `doc`, as a destructuring rest leaves it out.
fn take(doc: &mut Fields, key: &str) -> Option<Json> {
    let i = doc.iter().position(|(k, _)| k == key)?;
    Some(doc.remove(i).1)
}

/// `contributesFromV1` (`pluginMigrations.ts:12`).
pub fn contributes_from_v1(doc: Fields) -> Fields {
    if doc
        .iter()
        .any(|(k, _)| k == "contributes" || k == "sidecar" || k == "main")
    {
        return doc;
    }
    let mut rest = doc;
    let permissions = take(&mut rest, "permissions");
    let process = take(&mut rest, "process");
    let page = take(&mut rest, "page");
    let mut contributes = Fields::new();
    for key in MOVED_INTO_CONTRIBUTES {
        if let Some(value) = take(&mut rest, key) {
            contributes.push((key.to_owned(), value));
        }
    }
    if let Some(page) = page {
        contributes.push(("panels".to_owned(), Json::Array(vec![page_as_panel(page)])));
    }
    if let Some(permissions) = permissions {
        rest.push((
            "permissions".to_owned(),
            legacy_permissions(Some(&permissions)).unwrap_or(Json::Null),
        ));
    }
    if let Some(process) = process {
        rest.push(("sidecar".to_owned(), process));
    }
    rest.push(("contributes".to_owned(), Json::Object(contributes)));
    rest
}

/// `MANIFEST_MIGRATIONS` (`pluginMigrations.ts:30`).
pub fn manifest_migrations() -> Migrations {
    let mut migrations = Migrations::new();
    migrations.insert(
        0,
        Box::new(|mut doc: Fields| {
            let settings = take(&mut doc, "settings");
            take(&mut doc, "reset");
            if let Some(settings) = settings {
                doc.push((
                    "settings".to_owned(),
                    Json::Object(vec![("defaults".to_owned(), settings)]),
                ));
            }
            Json::Object(doc)
        }),
    );
    migrations.insert(
        1,
        Box::new(|doc: Fields| Json::Object(contributes_from_v1(doc))),
    );
    migrations
}

static V1_PATHS: LazyLock<[(Regex, &str, bool); 4]> = LazyLock::new(|| {
    let re = |p: &str| Regex::new(p).expect("V1_PATHS");
    // The last field is the `g` flag: replace every match, not only the first.
    [
        (re(r"contributes\.panels\[0\]"), "page", true),
        (re(r"contributes\."), "", true),
        (re(r"^sidecar(?-u:\b)"), "process", false),
        (
            re(r" needs a main or a sidecar$"),
            " needs a process",
            false,
        ),
    ]
});

/// `v1Problem` (`pluginMigrations.ts:45`): `problem` with the v1 names of the fields.
pub fn v1_problem(problem: &str) -> String {
    V1_PATHS.iter().fold(
        problem.to_owned(),
        |text, (pattern, replacement, global)| {
            if *global {
                pattern.replace_all(&text, *replacement).into_owned()
            } else {
                pattern.replace(&text, *replacement).into_owned()
            }
        },
    )
}
