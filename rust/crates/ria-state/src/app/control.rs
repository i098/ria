// Ported from ardia apps/browser/src/state/app/programControl.ts, programControlSetting.ts and controlCaller.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of those files, with changes.

//! The control indicator, the control server status, and the program-control sync.
//!
//! Changes from Ardia:
//! - A `setTimeout` or a store subscription is not state. The 3000 ms hold of the indicator is `Local::hide_pending`:
//!   `ria-app` starts one timer when it turns on and sends `IndicatorHoldElapsed` when it ends.
//!   A store subscription is the call of [`after_settings_change`] after each change of the settings.
//! - A native call that returns a value ends with a result action (plan section 6.5): `ControlCallSettled` for
//!   `setControl` and `controlStatus`, `ControlChanged` for the `engine.cliControl` event.
//!   A rejected call is `ControlCallSettled` with `error`, and the server status of that error is built here.
//! - The telemetry counters (`noteCliPillShown`, `noteCliConnections`, `noteDragsEnded`) write no store, so they are not ported.
//! - `shownReadyTab` reads the state of `browser`, `peek` and `pages`; those folders have no fields for it yet.

use serde_json::{Value, json};

use super::{ControlCaller, State};
use crate::kernel::{AppState, Effects};
use crate::platform::PlatformCall;
use crate::settings;

/// The private variables of `programControl.ts` that are not store fields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Control {
    pub(super) socket_runs: i64,
    pub(super) socket_serving: bool,
    pub(super) indicator_step: i64,
    pub(super) caller_step: i64,
    pub(super) hidden_step: i64,
    pub(super) hide_pending: bool,
    /// `sentActive`: `None` is `undefined`, `Some(None)` is `null`.
    pub(super) sent_active: Option<Option<String>>,
    pub(super) started: bool,
    pub(super) latest: i64,
    pub(super) in_flight: i64,
    pub(super) newest: f64,
    pub(super) applied: Option<bool>,
    pub(super) watching: bool,
}

fn server_text(state: &State, key: &str) -> Option<String> {
    state.server.get(key)?.as_str().map(str::to_owned)
}

fn server_number(server: &Value, key: &str) -> f64 {
    server.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn show_indicator(app: &mut AppState, shown: bool) {
    let st = &mut app.get_mut::<State>();
    if !shown {
        st.control.indicator_step += 1;
        st.control.hidden_step = st.control.indicator_step;
    }
    let keep = !(shown && st.control.caller_step < st.control.hidden_step);
    if !keep {
        st.caller = None;
    }
    st.shown = shown;
}

pub(super) fn note_control_caller(app: &mut AppState, caller: ControlCaller) {
    let st = app.get_mut::<State>();
    st.control.indicator_step += 1;
    st.control.caller_step = st.control.indicator_step;
    if st.caller.as_ref() != Some(&caller) {
        st.caller = Some(caller);
    }
}

/// `updateIndicator`, which runs after each change of `server` and of the socket counters.
pub(super) fn update_indicator(app: &mut AppState) {
    let st = app.get::<State>();
    let cdp_on = server_text(st, "state").as_deref() == Some("on");
    let connections = if cdp_on {
        server_number(&st.server, "connections") as i64
    } else {
        0
    };
    let on = cdp_on || st.control.socket_serving;
    let connected = on && connections + st.control.socket_runs > 0;
    if connected || !on {
        let shown = st.shown;
        app.get_mut::<State>().control.hide_pending = false;
        if shown != connected {
            show_indicator(app, connected);
        }
        return;
    }
    if !st.shown || st.control.hide_pending {
        return;
    }
    app.get_mut::<State>().control.hide_pending = true;
}

pub(super) fn indicator_hold_elapsed(app: &mut AppState) {
    if app.get::<State>().control.hide_pending {
        app.get_mut::<State>().control.hide_pending = false;
        show_indicator(app, false);
    }
}

pub(super) fn note_socket_run(app: &mut AppState, delta: i64) {
    let c = &mut app.get_mut::<State>().control;
    c.socket_runs = (c.socket_runs + delta).max(0);
    update_indicator(app);
}

pub(super) fn note_socket_serving(app: &mut AppState, serving: bool) {
    app.get_mut::<State>().control.socket_serving = serving;
    update_indicator(app);
}

/// `shownReadyTab`. It needs the focused window, the shown page and the page readiness of other folders.
fn shown_ready_tab(app: &AppState) -> Option<String> {
    let _ = app;
    todo!("blocked by the browser, peek and pages folders: PENDING.md")
}

pub(super) fn push_active_tab(app: &mut AppState, fx: &mut Effects) {
    let state = server_text(app.get::<State>(), "state");
    if !matches!(state.as_deref(), Some("on" | "starting")) {
        app.get_mut::<State>().control.sent_active = None;
        return;
    }
    let active = shown_ready_tab(app);
    if app.get::<State>().control.sent_active.as_ref() == Some(&active) {
        return;
    }
    app.get_mut::<State>().control.sent_active = Some(active.clone());
    fx.push(PlatformCall::EngineSetCliActiveTab { tab_id: active });
}

pub(super) fn note_tab_ready(app: &mut AppState, fx: &mut Effects, tab_id: &str) {
    let c = &mut app.get_mut::<State>().control;
    if c.sent_active.as_ref() == Some(&Some(tab_id.to_owned())) {
        c.sent_active = None;
    }
    push_active_tab(app, fx);
}

/// `programControlOn`.
pub(super) fn program_control_on(app: &AppState) -> bool {
    app.get::<settings::State>().settings.cli_control
}

/// `userChoseControl`.
pub(super) fn user_chose_control(app: &AppState) -> bool {
    let meta = &app.get::<settings::State>().settings_meta;
    meta.chosen.iter().any(|k| k == "cliControl")
        && !meta.inferred.iter().any(|k| k == "cliControl")
}

/// `userTurnedControlOff`.
pub(super) fn user_turned_control_off(app: &AppState) -> bool {
    !program_control_on(app) && user_chose_control(app)
}

/// `setProgramControl`: `updateSettings({ cliControl })`, then the store subscription of `startProgramControl`.
pub(super) fn set_program_control(app: &mut AppState, fx: &mut Effects, on: bool) {
    let patch = crate::onboarding::StateDraft {
        cli_control: Some(on),
        ..Default::default()
    };
    settings::apply(
        app,
        settings::Action::UpdateSettings {
            patch,
            options: None,
        },
        fx,
    );
    after_settings_change(app, fx);
}

/// The store subscription of `startProgramControl`: `sync()` and `pushActiveTab()`.
pub(super) fn after_settings_change(app: &mut AppState, fx: &mut Effects) {
    if !app.get::<State>().control.started {
        return;
    }
    sync(app, fx);
    push_active_tab(app, fx);
}

fn sync(app: &mut AppState, fx: &mut Effects) {
    let on = program_control_on(app);
    let c = &mut app.get_mut::<State>().control;
    if c.applied == Some(on) {
        return;
    }
    let first = c.applied.is_none();
    c.applied = Some(on);
    c.latest += 1;
    c.in_flight += 1;
    fx.push(if first && !on {
        PlatformCall::EngineCliControlStatus {}
    } else {
        PlatformCall::EngineSetCliControl { on }
    });
}

pub(super) fn start_program_control(app: &mut AppState, fx: &mut Effects) {
    if app.get::<State>().control.started {
        return;
    }
    app.get_mut::<State>().control.started = true;
    sync(app, fx);
}

/// `apply` inside `startProgramControl`.
fn apply_status(app: &mut AppState, fx: &mut Effects, server: Value, may_reconcile: bool) {
    let c = &app.get::<State>().control;
    let generation = if server.get("state").and_then(Value::as_str) == Some("external") {
        c.newest
    } else {
        server_number(&server, "generation")
    };
    if generation < c.newest {
        return;
    }
    let stopped_itself = server.get("state").and_then(Value::as_str) == Some("off")
        && server
            .get("reason")
            .is_some_and(|r| r.as_str().is_none_or(|s| !s.is_empty()) && !r.is_null());
    let st = app.get_mut::<State>();
    st.control.newest = generation;
    st.server = server;
    update_indicator(app);
    if app.get::<State>().control.started {
        push_active_tab(app, fx);
    }
    if may_reconcile && stopped_itself && program_control_on(app) {
        set_program_control(app, fx, false);
    }
}

pub(super) fn control_changed(app: &mut AppState, fx: &mut Effects, server: Value) {
    let idle = app.get::<State>().control.in_flight == 0;
    apply_status(app, fx, server, idle);
}

pub(super) fn control_call_settled(
    app: &mut AppState,
    fx: &mut Effects,
    request: i64,
    server: Value,
    error: Option<String>,
) {
    let c = &mut app.get_mut::<State>().control;
    c.in_flight -= 1;
    if request != c.latest {
        return;
    }
    let server = match error {
        Some(reason) => {
            json!({ "state": "off", "connections": 0, "generation": c.newest, "file": "", "reason": reason })
        }
        None => server,
    };
    apply_status(app, fx, server, true);
}

/// `ControlCaller` from `parseControlCaller`.
pub fn parse_control_caller(raw: &Value) -> Option<ControlCaller> {
    use std::sync::LazyLock;
    static SPACES: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"[\s\p{Z}]+").unwrap());
    static INVISIBLE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\p{C}").unwrap());
    let fields = raw.as_object()?;
    let flat = SPACES.replace_all(fields.get("name")?.as_str()?, " ");
    let flat = INVISIBLE.replace_all(&flat, "");
    let flat = SPACES.replace_all(&flat, " ");
    let name: String = flat.trim().chars().take(40).collect();
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let app = fields.get("app").and_then(Value::as_str).filter(|raw| {
        let parts: Vec<&str> = raw.split('/').collect();
        raw.encode_utf16().count() <= 1024
            && raw.starts_with('/')
            && raw.ends_with(".app")
            && raw.len() > 5
            && !parts.iter().any(|p| *p == ".." || *p == ".")
            && !parts[..parts.len() - 1].iter().any(|p| p.ends_with(".app"))
    });
    Some(ControlCaller {
        name: name.to_owned(),
        app: app.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_names_are_flattened_and_capped() {
        let c = parse_control_caller(
            &json!({"name": "  a\u{200b}\n b  ", "app": "/Applications/X.app"}),
        )
        .unwrap();
        assert_eq!(
            c,
            ControlCaller {
                name: "a b".into(),
                app: Some("/Applications/X.app".into())
            }
        );
        assert!(parse_control_caller(&json!({"name": "\u{200b}"})).is_none());
        let long = parse_control_caller(&json!({"name": "é".repeat(60)})).unwrap();
        assert_eq!(long.name.chars().count(), 40);
    }

    #[test]
    fn caller_app_must_be_a_plain_bundle_path() {
        for bad in [
            "/a/../b.app",
            "/x.app/y.app",
            "relative.app",
            "/a/b",
            "/.app",
        ] {
            let c = parse_control_caller(&json!({"name": "n", "app": bad})).unwrap();
            assert_eq!(c.app, None, "{bad}");
        }
    }
}
