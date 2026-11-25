// Ria original, no Ardia source.

//! Replays the steps of `applied.harness.ts` on `ria_sync::logic::applied`.

#![forbid(unsafe_code)]

use ria_sync::logic::applied::{covered, is_applied, mark_applied, raise_applied};
use ria_sync::logic::scope_state::{ScopeState, empty_scope};
use serde_json::{Value, json};

fn seq(v: &Value) -> Result<u64, String> {
    v.as_u64()
        .ok_or_else(|| format!("not a sequence number: {v}"))
}

/// `Object.assign(state, patch)` for `vv` and `extra`.
fn assign(state: &mut ScopeState, patch: &Value) -> Result<(), String> {
    let mut whole = serde_json::to_value(&*state).map_err(|e| e.to_string())?;
    for (k, v) in patch.as_object().into_iter().flatten() {
        whole[k] = v.clone();
    }
    *state = serde_json::from_value(whole).map_err(|e| e.to_string())?;
    Ok(())
}

/// Port of `replayApplied` (`applied.harness.ts:8`).
pub fn replay_applied(input: String) -> Result<Vec<Value>, String> {
    let steps: Vec<Vec<Value>> = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    let mut state = empty_scope();
    let mut out = Vec::new();
    for step in &steps {
        let arg = |i: usize| step.get(i).unwrap_or(&Value::Null);
        let device = arg(1).as_str().unwrap_or_default();
        let mut result = Value::Null;
        match arg(0).as_str() {
            Some("set") => assign(&mut state, arg(1))?,
            Some("covered") => {
                let vv = serde_json::from_value(arg(1).clone()).map_err(|e| e.to_string())?;
                result = json!(covered(
                    &vv,
                    (arg(2)[0].as_str().unwrap_or_default(), seq(&arg(2)[1])?)
                ));
            }
            Some("isApplied") => result = json!(is_applied(&state, (device, seq(arg(2))?))),
            Some("markApplied") => mark_applied(&mut state, device, seq(arg(2))?),
            _ => raise_applied(&mut state, device, seq(arg(2))?),
        }
        out.push(json!({"result": result, "vv": state.vv, "extra": state.extra}));
    }
    Ok(out)
}
