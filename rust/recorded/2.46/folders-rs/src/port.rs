// Ria original, no Ardia source.

//! Replays the cases of `folders.harness.ts` on `ria_sync::logic::folders`.

#![forbid(unsafe_code)]

use indexmap::IndexMap;
use ria_sync::logic::folders::folder_children;
use serde_json::{Value, json};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    parent_of: Vec<(String, String)>,
    keys: Vec<String>,
    fallback: String,
}

/// Port of `replayFolders` (`folders.harness.ts:7`).
pub fn replay_folders(input: String) -> Result<Vec<Value>, String> {
    let cases: Vec<Case> = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    Ok(cases
        .into_iter()
        .map(|c| {
            let mut parent_of: IndexMap<String, String> = c.parent_of.into_iter().collect();
            let children = folder_children(&mut parent_of, &c.keys, &c.fallback);
            json!({
                "parentOf": parent_of.into_iter().collect::<Vec<_>>(),
                "children": children.into_iter().collect::<Vec<_>>(),
            })
        })
        .collect())
}
