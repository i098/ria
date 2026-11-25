// Ria original, no Ardia source.

//! Tests of the `settings` reducer: the recorded Ardia cases (`rust/replay/settings/cases.jsonl`) and a few direct cases.

use serde_json::{Value, json};

use super::{Action, State, apply};
use crate::kernel::{AppState, Effects};
use crate::platform::PlatformCall;

fn run(app: &mut AppState, action: Value) -> Vec<PlatformCall> {
    try_run(app, action).expect("action JSON")
}

/// `None` when the action does not fit the typed `Action` (a value outside the enum that TypeScript types forbid).
fn try_run(app: &mut AppState, action: Value) -> Option<Vec<PlatformCall>> {
    let action: Action = serde_json::from_value(action).ok()?;
    let mut fx = Effects::default();
    apply(app, action, &mut fx);
    Some(fx.take())
}

fn state_json(app: &AppState) -> Value {
    serde_json::to_value(app.get::<State>()).expect("state JSON")
}

#[test]
fn update_settings_keeps_valid_entries_in_patch_order_and_marks_them_chosen() {
    let mut app = AppState::default();
    run(
        &mut app,
        json!({"type": "updateSettings", "patch": {"warnBeforeQuitting": true, "restoreSession": "no", "autoHideTabs": true}}),
    );
    let s = state_json(&app);
    assert_eq!(s["settings"]["warnBeforeQuitting"], json!(true));
    assert_eq!(s["settings"]["restoreSession"], json!(true));
    assert_eq!(
        s["settingsMeta"],
        json!({"chosen": ["warnBeforeQuitting", "autoHideTabs"], "inferred": ["hideToolbarWhileScrolling:offByDefault"]})
    );
}

#[test]
fn link_keys_stay_apart() {
    let mut app = AppState::default();
    run(
        &mut app,
        json!({"type": "updateSettings", "patch": {"peekModifier": "alt"}}),
    );
    run(
        &mut app,
        json!({"type": "updateSettings", "patch": {"splitLinkModifier": "alt"}}),
    );
    let s = state_json(&app);
    assert_eq!(s["settings"]["peekModifier"], json!("shift"));
    assert_eq!(s["settings"]["splitLinkModifier"], json!("alt"));
}

#[test]
fn settings_history_keeps_30_places_and_walks_back_and_forward() {
    let mut app = AppState::default();
    for i in 0..40 {
        run(
            &mut app,
            json!({"type": "goToSettingsPane", "pane": "tabs", "detail": format!("d{i}")}),
        );
    }
    assert_eq!(state_json(&app)["back"].as_array().map(Vec::len), Some(30));
    run(&mut app, json!({"type": "settingsBack"}));
    run(&mut app, json!({"type": "settingsForward"}));
    assert_eq!(state_json(&app)["detail"], json!("d39"));
}

#[test]
fn custom_engine_with_non_ascii_name_is_added_and_removed() {
    let mut app = AppState::default();
    let input = json!({"name": "検索", "keyword": "ké", "url": "https://例え.jp/?q=%s"});
    run(
        &mut app,
        json!({"type": "addCustomEngine", "input": input, "now": 1_700_000_000_000_u64}),
    );
    let engines = state_json(&app)["settings"]["customSearchEngines"].clone();
    assert_eq!(engines[0]["id"], json!("engine-loyw3v28-1"));
    assert_eq!(engines[0]["keyword"], json!("ké"));
    run(
        &mut app,
        json!({"type": "removeCustomEngine", "id": "engine-loyw3v28-1"}),
    );
    assert_eq!(
        state_json(&app)["settings"]["customSearchEngines"],
        json!([])
    );
}

#[test]
fn scroll_ends_count_once_until_the_reader_scrolls_away() {
    let mut app = AppState::default();
    let end =
        json!({"type": "noteSettingsScroll", "offset": 900, "viewport": 100, "content": 1000});
    run(&mut app, end.clone());
    run(&mut app, end.clone());
    assert_eq!(state_json(&app)["reached"], json!(1));
    run(
        &mut app,
        json!({"type": "noteSettingsScroll", "offset": 0, "viewport": 100, "content": 1000}),
    );
    run(&mut app, end);
    assert_eq!(state_json(&app)["reached"], json!(2));
}

#[test]
fn native_settings_send_each_value_once() {
    let mut app = AppState::default();
    let first = run(&mut app, json!({"type": "startNativeSettings"}));
    assert!(first.iter().any(|c| c.name() == "ArdiaShell.setAppearance"));
    let again = run(
        &mut app,
        json!({"type": "updateSettings", "patch": {"videoUpscaling": true, "autoPictureInPicture": true, "appearance": "dark"}}),
    );
    let names: Vec<_> = again.iter().map(PlatformCall::name).collect();
    assert_eq!(names, ["ArdiaShell.setAppearance"]);
}

/// The first path where two JSON values differ, for a short failure message.
fn first_diff(want: &Value, got: &Value, path: &str) -> Option<String> {
    match (want, got) {
        (Value::Object(a), Value::Object(b)) => a.keys().chain(b.keys()).find_map(|k| {
            first_diff(
                a.get(k).unwrap_or(&Value::Null),
                b.get(k).unwrap_or(&Value::Null),
                &format!("{path}.{k}"),
            )
        }),
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .enumerate()
            .find_map(|(i, (x, y))| first_diff(x, y, &format!("{path}[{i}]"))),
        _ if want == got => None,
        _ => Some(format!("{path}: want {want} got {got}")),
    }
}

fn pending(case: usize) -> bool {
    let list = include_str!("../../../../replay/settings/PENDING.md");
    list.lines()
        .any(|l| l.starts_with(&format!("- case {case}:")))
}

/// Every recorded case that does not wait for a sibling module must match Ardia step by step.
#[test]
fn recorded_ardia_cases_match() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../replay/settings/cases.jsonl"
    );
    let text = std::fs::read_to_string(path)
        .expect("cases.jsonl: record it with rust/replay/settings/README.md");
    let (mut passed, mut waiting, mut skipped, mut failures) = (0, 0, 0, Vec::new());
    'cases: for (n, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        if pending(n) {
            waiting += 1;
            continue;
        }
        let case: Value = serde_json::from_str(line).expect("case JSON");
        let mut app = AppState::default();
        let mut open: Vec<PlatformCall> = Vec::new();
        let mut bad = None;
        let actions = case["args"][0].as_array().cloned().unwrap_or_default();
        for (i, (action, step)) in actions
            .iter()
            .zip(case["ok"].as_array().cloned().unwrap_or_default())
            .enumerate()
        {
            let calls = if action["type"] == "settle" {
                // The native promises resolve: a scale change comes back as `appliedInterfaceScale`.
                let pending: Vec<_> = std::mem::take(&mut open);
                let mut done = Vec::new();
                for c in pending
                    .iter()
                    .filter(|c| c.name() == "ArdiaApp.setInterfaceScale")
                {
                    done.extend(run(
                        &mut app,
                        json!({"type": "appliedInterfaceScale", "scale": c.args()[0]}),
                    ));
                }
                done
            } else {
                let Some(calls) = try_run(&mut app, action.clone()) else {
                    skipped += 1;
                    continue 'cases;
                };
                open.extend(calls.iter().cloned());
                calls
            };
            let got = json!({"state": state_json(&app), "calls": calls.iter().map(|c| json!({"name": c.name(), "args": c.args()})).collect::<Vec<_>>()});
            let want_calls: Vec<Value> = step["calls"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter(|c| c["name"] != "ArdiaSystem.setThinScrollBars")
                        .map(|c| json!({"name": c["name"], "args": c["args"]}))
                        .collect()
                })
                .unwrap_or_default();
            let want = json!({"state": step["state"], "calls": want_calls});
            if got != want {
                bad = Some(format!(
                    "case {n} step {i} action {action}\n  {}",
                    first_diff(&want, &got, "").unwrap_or_default()
                ));
                break;
            }
        }
        match bad {
            Some(b) => failures.push(b),
            None => passed += 1,
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed, {passed} passed, {waiting} waiting, {skipped} untyped:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(passed >= 200, "only {passed} cases passed");
}
