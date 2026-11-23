// Ported from ardia packages/core/src/bangs.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of the `BangTable` functions of `ria-core` `search::bangs`, as the recording
//! harness `bangs.harness.ts` calls them: a table goes in as slim lists and comes out as its size
//! and the answers of `get`. The logic is in `ria_core::search::bangs`.

#![forbid(unsafe_code)]

use ria_core::search::bangs::{self, Bang, BangTable};
use serde_json::{Value, json};

fn bang_value(bang: &Bang) -> Value {
    json!({ "trigger": bang.trigger, "name": bang.name, "template": bang.template, "home": bang.home })
}

/// `view` of the harness: the size and `get` of each trigger, `null` when there is none.
fn view(table: &BangTable, triggers: &[String]) -> Value {
    let bangs: Vec<Value> = triggers
        .iter()
        .map(|t| table.get(t).map_or(Value::Null, bang_value))
        .collect();
    json!({ "size": table.size(), "bangs": bangs })
}

/// `bangTable` (`packages/core/src/bangs.ts:140`) through `bangTableView`.
pub fn bang_table_view(slim: String, triggers: Vec<String>) -> Result<serde_json::Value, String> {
    Ok(view(&bangs::bang_table(slim), &triggers))
}

/// `checkedBangTable` (`packages/core/src/bangs.ts:161`) through `checkedBangTableView`.
pub fn checked_bang_table_view(
    slim: String,
    triggers: Vec<String>,
) -> Result<serde_json::Value, String> {
    Ok(bangs::checked_bang_table(&slim).map_or(Value::Null, |table| view(&table, &triggers)))
}

/// `readBangDocument` (`packages/core/src/bangs.ts:177`) through `readBangDocumentView`.
pub fn read_bang_document_view(
    doc: Option<String>,
    triggers: Vec<String>,
) -> Result<serde_json::Value, String> {
    let Some(read) = bangs::read_bang_document(doc.as_deref()) else {
        return Ok(Value::Null);
    };
    let mut out = json!({ "etag": read.etag });
    if let (Some(out), Value::Object(view)) = (out.as_object_mut(), view(&read.table, &triggers)) {
        out.extend(view);
    }
    Ok(out)
}

/// `joinBangTables` (`packages/core/src/bangs.ts:187`) through `joinBangTablesView`.
pub fn join_bang_tables_view(
    slims: Vec<String>,
    triggers: Vec<String>,
) -> Result<serde_json::Value, String> {
    let table = bangs::join_bang_tables(slims.into_iter().map(bangs::bang_table).collect());
    Ok(view(&table, &triggers))
}

/// `findBang` (`packages/core/src/bangs.ts:200`) through `findBangIn`; `slims` null or
/// `undefined` is no table (`slims && ...` of the harness).
pub fn find_bang_in(
    texts: Vec<String>,
    slims: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let slims: Option<Vec<String>> = if crate::js::is_undefined(&slims) {
        None
    } else {
        serde_json::from_value(slims).map_err(|e| format!("TypeError: {e}"))?
    };
    let table = slims
        .map(|slims| bangs::join_bang_tables(slims.into_iter().map(bangs::bang_table).collect()));
    let found: Vec<Value> = texts
        .iter()
        .map(|text| match bangs::find_bang(text, table.as_ref()) {
            Some(m) => json!({ "bang": bang_value(m.bang), "query": m.query, "url": m.url }),
            None => Value::Null,
        })
        .collect();
    Ok(Value::Array(found))
}
