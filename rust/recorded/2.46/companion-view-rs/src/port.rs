// Ria original, no Ardia source.

//! Replays the steps of `companion-view.harness.ts` on `ria_sync::logic::companion`.

#![forbid(unsafe_code)]

use indexmap::IndexMap;
use ria_sync::logic::companion::{legacy, theme, view};
use ria_sync::logic::scope_state::{ScopeState, empty_scope};
use serde::Serialize;
use serde_json::{Value, json};

/// `scope` of the harness: an empty scope with the given `records` and `seen`; `null` is no scope.
fn scope(v: &Value) -> Result<Option<ScopeState>, String> {
    if v.is_null() {
        return Ok(None);
    }
    let mut full = serde_json::to_value(empty_scope()).map_err(|e| e.to_string())?;
    if let (Some(full), Some(part)) = (full.as_object_mut(), v.as_object()) {
        full.extend(part.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    serde_json::from_value(full)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn to_json(v: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

fn step(s: &Value) -> Result<Value, String> {
    let arg = |i: usize| s.get(i).unwrap_or(&Value::Null);
    let text = |i: usize| arg(i).as_str().unwrap_or_default().to_owned();
    match s.get(0).and_then(Value::as_str).unwrap_or_default() {
        "buildView" => {
            let mut scopes = IndexMap::new();
            for (k, v) in arg(1).as_object().into_iter().flatten() {
                scopes.insert(k.clone(), scope(v)?.unwrap_or_else(empty_scope));
            }
            to_json(view::build_view(&scopes, &text(2)))
        }
        "liveProfiles" => Ok(json!(
            view::live_profiles(scope(arg(1))?.as_ref())
                .into_iter()
                .collect::<Vec<_>>()
        )),
        "defaultProfileId" => {
            let entries = arg(1).as_array().into_iter().flatten();
            let profiles: IndexMap<String, Value> = entries
                .filter_map(|e| Some((e.get(0)?.as_str()?.to_owned(), e.get(1)?.clone())))
                .collect();
            Ok(json!(view::default_profile_id(&profiles)))
        }
        "dataOwner" => Ok(json!(view::data_owner(scope(arg(1))?.as_ref(), &text(2)))),
        "pinnedArea" => to_json(view::pinned_area(scope(arg(1))?.as_ref())),
        "bookmarkTree" => to_json(view::bookmark_tree(scope(arg(1))?.as_ref())),
        "historyList" => to_json(view::history_list(scope(arg(1))?.as_ref())),
        "themeOf" => {
            let app = scope(arg(1))?.unwrap_or_else(empty_scope);
            to_json(theme::theme_of(&app, &text(2), s.get(3)))
        }
        "fromLegacy" => Ok(s
            .get(1)
            .map_or_else(crate::js::undefined, legacy::from_legacy)),
        "fromLegacyUrl" => Ok(json!(legacy::from_legacy_url(&text(1)))),
        name => Err(format!("calls[name] is not a function: {name}")),
    }
}

/// Port of `replayCompanionView` (`companion-view.harness.ts:26`).
pub fn replay_companion_view(input: String) -> Result<Vec<serde_json::Value>, String> {
    let steps: Vec<Value> = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    steps.iter().map(step).collect()
}
