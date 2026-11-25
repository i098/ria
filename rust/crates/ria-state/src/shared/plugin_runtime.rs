// Ported from ardia apps/browser/src/lib/pluginRuntime.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Checks of the processes a plugin runs: its sidecar, its main script and its programs.
//!
//! Changes from Ardia:
//! - `unknown` is `Option<&Json>`: `None` is `undefined`, `Some(&Json::Null)` is `null`.
//! - `memoryMB` and `afterUpdate` are `memory_mb` and `after_update`.
//! - A plain object read twice by key (`doc.run`) reads its first entry with that key.

use ria_core::identity::telemetry_wire::Json;
use serde::Serialize;

use super::decode::Fields;
use super::plugin_checks::{
    Check, DEFAULT_MAIN_MB, DEFAULT_SIDECAR_MB, LIMITS, NAME, NAME_RULE, at, flag, integer,
    is_argument, is_plugin_path, matching, object, only, plugin_file, text,
};

/// `ProcessSpec` (`pluginRuntime.ts:3`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSpec {
    pub run: Vec<String>,
    #[serde(rename = "memoryMB")]
    pub memory_mb: f64,
}

/// `MainSpec` (`pluginRuntime.ts:4`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MainSpec {
    pub entry: String,
    #[serde(rename = "memoryMB")]
    pub memory_mb: f64,
}

/// `ProgramSpec` (`pluginRuntime.ts:5`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramSpec {
    pub name: String,
    pub path: String,
    pub after_update: Vec<Vec<String>>,
    pub example: Option<String>,
    pub claim: bool,
}

/// `doc[key]`.
fn field<'a>(doc: &'a Fields, key: &str) -> Option<&'a Json> {
    doc.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// The strings of `value` when it is a list of 1 to `max` arguments.
fn arguments(value: Option<&Json>, max: f64) -> Option<Vec<String>> {
    match value {
        Some(Json::Array(items))
            if !items.is_empty() && items.len() as f64 <= max && items.iter().all(is_argument) =>
        {
            Some(
                items
                    .iter()
                    .filter_map(|a| match a {
                        Json::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect(),
            )
        }
        _ => None,
    }
}

/// `sidecarOf` (`pluginRuntime.ts:7`).
pub fn sidecar_of(c: &mut Check, path: &str, value: Option<&Json>) -> Option<ProcessSpec> {
    value?;
    let doc = object(c, path, value)?;
    only(c, path, doc, &["run", "memoryMB"]);
    let Some(run) = arguments(field(doc, "run"), LIMITS.argv as f64) else {
        c.problems.push(format!(
            "{path}.run must be a list of 1 to {} arguments, the program first",
            LIMITS.argv
        ));
        return None;
    };
    let program = &run[0];
    if !program.starts_with('/') && program.contains('/') && !is_plugin_path(program) {
        c.problems.push(format!(
            "{path}.run[0] must be a file in the plugin's folder, an absolute path or a program name"
        ));
    }
    let memory_mb = integer(
        c,
        &format!("{path}.memoryMB"),
        field(doc, "memoryMB"),
        LIMITS.memory_mb,
        DEFAULT_SIDECAR_MB,
    );
    Some(ProcessSpec { run, memory_mb })
}

/// `mainOf` (`pluginRuntime.ts:24`).
pub fn main_of(c: &mut Check, entry: Option<&Json>, memory: Option<&Json>) -> Option<MainSpec> {
    if entry.is_none() {
        if memory.is_some() {
            c.problems.push("memoryMB only goes with main".to_owned());
        }
        return None;
    }
    let file = plugin_file(
        c,
        "main",
        entry,
        "a JavaScript file inside the plugin's folder, like dist/index.js",
        None,
    );
    if let Some(f) = file {
        let lower = f.to_ascii_lowercase();
        if ![".js", ".mjs", ".cjs"].iter().any(|e| lower.ends_with(e)) {
            c.problems
                .push("main must be a .js, .mjs or .cjs file".to_owned());
        }
    }
    let memory_mb = integer(c, "memoryMB", memory, LIMITS.memory_mb, DEFAULT_MAIN_MB);
    file.map(|f| MainSpec {
        entry: f.to_owned(),
        memory_mb,
    })
}

/// `program` (`pluginRuntime.ts:35`).
pub fn program(c: &mut Check, path: &str, value: Option<&Json>) -> Option<ProgramSpec> {
    let doc = object(c, path, value)?;
    only(
        c,
        path,
        doc,
        &["name", "path", "afterUpdate", "example", "claim"],
    );
    let name = matching(
        c,
        &format!("{path}.name"),
        field(doc, "name"),
        &NAME,
        NAME_RULE,
        true,
    );
    let file = match field(doc, "path") {
        Some(Json::String(s)) if is_plugin_path(s) => Some(s),
        _ => {
            c.problems.push(format!(
                "{path}.path must be a file in the plugin's folder, like bin/tool"
            ));
            None
        }
    };
    let [most, argv] = LIMITS.after_update;
    let after = match field(doc, "afterUpdate") {
        None => Some(Vec::new()),
        Some(Json::Array(lists)) if lists.len() as f64 <= most => lists
            .iter()
            .map(|a| arguments(Some(a), argv))
            .collect::<Option<Vec<_>>>(),
        Some(_) => None,
    };
    if after.is_none() {
        c.problems.push(format!(
            "{} must be a list of at most {most} argument lists of 1 to {argv}",
            at(path, "afterUpdate")
        ));
    }
    let example = text(
        c,
        &format!("{path}.example"),
        field(doc, "example"),
        LIMITS.example,
        false,
    );
    let claim = flag(c, &format!("{path}.claim"), field(doc, "claim"), false);
    Some(ProgramSpec {
        name: name?.to_owned(),
        path: file?.clone(),
        after_update: after?,
        example: example.map(str::to_owned),
        claim,
    })
}
