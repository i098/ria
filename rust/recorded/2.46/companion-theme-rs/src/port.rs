// Ria original, no Ardia source.

//! Calls `ria_sync::logic::companion::theme` for the cases of `companion/theme.ts`.

#![forbid(unsafe_code)]

use ria_sync::logic::scope_state::{SyncRecord, empty_scope};
use serde_json::Value;

/// Port of `themeOf` (`packages/sync/src/companion/theme.ts:53`).
///
/// `themeOf` reads only `app.records[key]?.v`, so a record without a `v` is left out.
pub fn theme_of(
    app: serde_json::Value,
    sync_id: String,
    profile_theme: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let records = match app.get("records") {
        None => {
            return Err(format!(
                "Cannot read properties of undefined (reading 'pth:{sync_id}')"
            ));
        }
        Some(Value::Null) => {
            return Err(format!(
                "Cannot read properties of null (reading 'pth:{sync_id}')"
            ));
        }
        Some(records) => records,
    };
    let mut state = empty_scope();
    for (k, record) in records.as_object().into_iter().flatten() {
        if let Some(v) = record.as_object().and_then(|r| r.get("v")) {
            state.records.insert(
                k.clone(),
                SyncRecord {
                    h: String::new(),
                    v: v.clone(),
                    o: (String::new(), 0),
                },
            );
        }
    }
    let theme = ria_sync::logic::companion::theme::theme_of(&state, &sync_id, Some(&profile_theme));
    theme
        .map(|t| serde_json::to_value(t).map_err(|e| e.to_string()))
        .transpose()
}
