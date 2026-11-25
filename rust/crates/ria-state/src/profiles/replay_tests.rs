// Ria original, no Ardia source.

//! Check S: replays the sequences that `rust/replay/profiles/record-profiles.mjs` recorded from Ardia (`cases.jsonl`)
//! and compares the state after each action. A `seed` action sets the profile records; the picker, the create request
//! and the sheet are the other compared fields.

use serde_json::{Value, json};

use super::{Action, State, apply};
use crate::kernel::{AppState, Effects};

const CASES: &str = include_str!("../../../../replay/profiles/cases.jsonl");

fn shown(app: &AppState) -> Value {
    let s = serde_json::to_value(app.get::<State>()).expect("state is JSON");
    let mut out = serde_json::Map::new();
    for key in [
        "profiles",
        "profileOrder",
        "orphanedProfileData",
        "picker",
        "request",
        "sheet",
    ] {
        out.insert(key.into(), s[key].clone());
    }
    Value::Object(out)
}

fn seed(app: &mut AppState, a: &Value) {
    let s = app.get_mut::<State>();
    s.profiles = serde_json::from_value(a["profiles"].clone()).expect("seed profiles");
    // `Option<Value>` reads `"seenTheme": null` as absent, so restore the null that Ardia has.
    for (id, p) in &mut s.profiles {
        if a["profiles"][id]["seenTheme"].is_null() && a["profiles"][id].get("seenTheme").is_some()
        {
            p.seen_theme = Some(Value::Null);
        }
    }
    s.profile_order = serde_json::from_value(a["profileOrder"].clone()).expect("seed order");
    s.orphaned_profile_data =
        serde_json::from_value(a.get("orphanedProfileData").cloned().unwrap_or(json!([])))
            .expect("seed orphans");
}

#[test]
fn recorded_ardia_sequences_match() {
    let mut count = 0;
    let mut failures = Vec::new();
    for line in CASES.lines() {
        let case: Value = serde_json::from_str(line).expect("case line");
        let actions = case["args"][0].as_array().expect("actions");
        let steps = case["ok"].as_array().expect("steps");
        let mut app = AppState::default();
        let mut fx = Effects::default();
        for (action, step) in actions.iter().zip(steps) {
            if action["type"] == "seed" {
                seed(&mut app, action);
            } else {
                let parsed: Action = serde_json::from_value(action.clone())
                    .unwrap_or_else(|e| panic!("{e}: {action}"));
                apply(&mut app, parsed, &mut fx);
            }
            if shown(&app) != step["state"] {
                failures.push(format!(
                    "{action}\n  got  {}\n  want {}",
                    shown(&app),
                    step["state"]
                ));
                break;
            }
        }
        count += 1;
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(count >= 100, "only {count} cases");
}
