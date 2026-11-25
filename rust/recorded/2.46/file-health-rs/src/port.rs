// Ria original, no Ardia source.

//! Replays the steps of `file-health.harness.ts` on `ria_sync::logic::file_health`.

#![forbid(unsafe_code)]

use ria_sync::logic::file_health::{PulledFiles, holds_back, note_file_health, pull_settled};
use ria_sync::logic::scope_state::{ScopeState, empty_scope};
use serde_json::{Value, json};

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(str::to_owned))
        .collect()
}

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// `Object.assign(state, patch)`.
fn assign(state: &mut ScopeState, patch: &Value) -> Result<(), String> {
    let mut whole = serde_json::to_value(&*state).map_err(|e| e.to_string())?;
    for (k, v) in patch.as_object().into_iter().flatten() {
        whole[k] = v.clone();
    }
    *state = serde_json::from_value(whole).map_err(|e| e.to_string())?;
    Ok(())
}

/// Port of `replayFileHealth` (`file-health.harness.ts:9`).
pub fn replay_file_health(input: String) -> Result<Vec<Value>, String> {
    let steps: Vec<Vec<Value>> = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    let mut state = empty_scope();
    let mut out = Vec::new();
    for step in &steps {
        let arg = |i: usize| step.get(i).unwrap_or(&Value::Null);
        let mut result = Value::Null;
        match arg(0).as_str() {
            Some("set") => assign(&mut state, arg(1))?,
            Some("note") => {
                let f = arg(1);
                let files = PulledFiles {
                    read: strings(&f["read"]),
                    unreadable: strings(&f["unreadable"]),
                    pending: strings(&f["pending"]),
                    present: strings(&f["present"]).into_iter().collect(),
                };
                note_file_health(&mut state, &files, number(arg(2)), number(arg(3)));
            }
            Some("holdsBack") => result = json!(holds_back(&state, number(arg(1)), number(arg(2)))),
            _ => result = json!(pull_settled(&state)),
        }
        out.push(json!({
            "result": result,
            "files": state.files,
            "damaged": state.damaged,
            "pendingSince": state.pending_since,
            "pending": state.pending,
        }));
    }
    Ok(out)
}
