// Ria original, no Ardia source.

//! The Rust side of `scope-keys.harness.ts`: the same steps against `ria_sync::logic::scope_keys`.

#![forbid(unsafe_code)]

use ria_sync::logic::canon::canonical;
use ria_sync::logic::scope_keys::{
    Tracker, base_keys_with, drop_base, drop_record, put_base, put_record, records_with,
    track_touched_keys, visible, with_base_index,
};
use ria_sync::logic::scope_state::{ScopeState, empty_scope};
use serde_json::{Value, json};

struct World {
    state: ScopeState,
    trackers: Vec<Tracker>,
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

fn to_value(v: &impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// `{ ...emptyScope(), ...partial }`.
fn state_from(partial: &Value) -> Result<ScopeState, String> {
    let mut state = to_value(&empty_scope())?;
    if let (Some(state), Some(partial)) = (state.as_object_mut(), partial.as_object()) {
        state.extend(partial.clone());
    }
    serde_json::from_value(state).map_err(|e| e.to_string())
}

fn tracker<'a>(w: &'a World, i: &Value) -> Result<&'a Tracker, String> {
    i.as_u64()
        .and_then(|i| w.trackers.get(i as usize))
        .ok_or_else(|| "bad tracker".to_string())
}

async fn run(w: &mut World, step: &[Value]) -> Result<Value, String> {
    let (a, b) = (
        step.get(1).unwrap_or(&Value::Null),
        step.get(2).unwrap_or(&Value::Null),
    );
    Ok(
        match step.first().and_then(Value::as_str).unwrap_or_default() {
            "state" => {
                w.state = state_from(a)?;
                Value::Null
            }
            "visible" => json!(
                visible(&w.state, text(a))
                    .into_iter()
                    .map(|(k, v)| json!([k, v]))
                    .collect::<Vec<_>>()
            ),
            "recordsWith" => json!(
                records_with(&w.state, text(a))
                    .into_iter()
                    .map(|(k, r)| json!([k, r]))
                    .collect::<Vec<_>>()
            ),
            "track" => {
                w.trackers.push(track_touched_keys(&mut w.state));
                json!(w.trackers.len() - 1)
            }
            "stop" => {
                let touched = tracker(w, a)?.touched.clone();
                Tracker { touched }.stop(&mut w.state);
                Value::Null
            }
            "touched" => {
                let touched = tracker(w, a)?.touched.borrow();
                json!({"records": touched.records, "base": touched.base})
            }
            "putRecord" => {
                put_record(
                    &mut w.state,
                    text(a),
                    serde_json::from_value(b.clone()).map_err(|e| e.to_string())?,
                );
                Value::Null
            }
            "dropRecord" => {
                drop_record(&mut w.state, text(a));
                Value::Null
            }
            "putBase" => {
                put_base(&mut w.state, text(a), text(b).to_string());
                Value::Null
            }
            "dropBase" => {
                drop_base(&mut w.state, text(a));
                Value::Null
            }
            "baseKeysWith" => json!(base_keys_with(&mut w.state, text(a))),
            "withBaseIndex" => {
                let steps = a.as_array().cloned().unwrap_or_default();
                let mut state = std::mem::replace(&mut w.state, empty_scope());
                let trackers = std::mem::take(&mut w.trackers);
                let mut inner = World {
                    state: empty_scope(),
                    trackers,
                };
                let out = with_base_index(&mut state, async |state: &mut ScopeState| {
                    // The nested steps run on the same state, which `with_base_index` lends.
                    std::mem::swap(&mut inner.state, state);
                    let out = Box::pin(replay_steps(&mut inner, &steps)).await;
                    std::mem::swap(&mut inner.state, state);
                    out
                })
                .await;
                w.state = state;
                w.trackers = inner.trackers;
                json!(out?)
            }
            "show" => json!(canonical(&to_value(&w.state)?)),
            _ => json!({"error": "bad step"}),
        },
    )
}

async fn replay_steps(w: &mut World, steps: &[Value]) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for step in steps {
        out.push(match step.as_array() {
            Some(step) => run(w, step).await?,
            None => json!({"error": "bad step"}),
        });
    }
    Ok(out)
}

/// Port of `replayScopeKeys` (`packages/sync/src/scope-keys.harness.ts:57`).
pub async fn replay_scope_keys(input: String) -> Result<Vec<Value>, String> {
    let Ok(Value::Array(steps)) = serde_json::from_str::<Value>(&input) else {
        return Ok(vec![json!({"error": "bad script"})]);
    };
    futures::executor::block_on(replay_steps(
        &mut World {
            state: empty_scope(),
            trackers: Vec::new(),
        },
        &steps,
    ))
}
