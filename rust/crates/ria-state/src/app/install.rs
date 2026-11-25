// Ported from ardia apps/browser/src/state/app/linuxCommandInstall.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The command install: `useCommandInstall` and the saved record `ardia-cli.json`.
//!
//! Changes from Ardia:
//! - A synchronous native read (`installedCommandVersion`, `canInstallCommandUnattended`, `systemInfo`, `readDocument`)
//!   or the clock cannot happen inside `apply`. The host reads them and sends them in `InstallReads`.
//! - `installCommand` ends with `CommandInstallFinished`, and each `onInstallLine` event is `CommandInstallLine`.
//! - `Local::saved` keeps the record, so `readSaved()` after `save()` is the record that `save()` wrote.
//!   A failed `writeDocument` is not seen here; Ardia only warns for it.
//! - `startCommandInstall` (a timer after the first window) and `noteCliInstall` (a telemetry counter) are not ported.

use ria_core::identity::telemetry_wire::{Json, set_entry};

use super::control::{program_control_on, set_program_control, user_turned_control_off};
use super::{CommandInstallFinished, InstallReads, State, StateInstalling};
use crate::kernel::{AppState, Effects};
use crate::platform::PlatformCall;
use crate::shared::saved_document::{Migrations, parse_saved, unchanged, versioned_json};

const DOC: &str = "ardia-cli.json";
const DOC_VERSION: f64 = 1.0;
const SCHEMA_FIELD: &str = "schemaVersion";
const AUTO_INSTALL_TRIES: f64 = 3.0;

fn field<'a>(record: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    record.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// A truthy number field, as `!!record.field` tests it.
fn count(record: &[(String, Json)], key: &str) -> f64 {
    match field(record, key) {
        Some(Json::Number(n)) if !n.is_nan() => *n,
        _ => 0.0,
    }
}

fn truthy_text(record: &[(String, Json)], key: &str) -> bool {
    matches!(field(record, key), Some(Json::String(s)) if !s.is_empty())
}

fn never_installed(record: &[(String, Json)]) -> bool {
    count(record, "autoInstalledAt") == 0.0
        && count(record, "autoInstallTries") == 0.0
        && !truthy_text(record, "version")
        && !truthy_text(record, "binPath")
}

fn auto_install_unfinished(record: &[(String, Json)]) -> bool {
    count(record, "autoInstallTries") != 0.0 && count(record, "autoInstalledAt") == 0.0
}

/// `save`: merges `patch` and `at` into the record and asks for the write.
fn save(app: &mut AppState, fx: &mut Effects, patch: &[(&str, Json)], now: f64) {
    let local = &mut app.get_mut::<State>().local;
    for (key, value) in patch {
        set_entry(&mut local.saved, key, value.clone());
    }
    set_entry(&mut local.saved, "at", Json::Number(now));
    // `versionedJson` puts the version field in the written text, and `readSaved()` reads it back.
    set_entry(&mut local.saved, SCHEMA_FIELD, Json::Number(DOC_VERSION));
    let contents = versioned_json(
        &Json::Object(local.saved.clone()),
        DOC_VERSION,
        SCHEMA_FIELD,
        None,
    );
    fx.push(PlatformCall::DocumentsWriteDocument {
        name: DOC.into(),
        contents,
    });
}

fn text_or_null(value: &Option<String>) -> Json {
    value.clone().map_or(Json::Null, Json::String)
}

pub(super) fn install_command_in_background(
    app: &mut AppState,
    fx: &mut Effects,
    reason: crate::telemetry::CliInstallReason,
) {
    if app.get::<State>().local.running {
        return;
    }
    let st = app.get_mut::<State>();
    st.local.running = true;
    st.installing = Some(match reason {
        crate::telemetry::CliInstallReason::Settings => StateInstalling::Settings,
        crate::telemetry::CliInstallReason::Auto => StateInstalling::Auto,
        crate::telemetry::CliInstallReason::Upgrade => StateInstalling::Upgrade,
    });
    st.step = None;
    fx.push(PlatformCall::CliInstall {});
}

pub(super) fn command_install_line(app: &mut AppState, line: &str) {
    if !app.get::<State>().local.running {
        return;
    }
    let step = line
        .strip_prefix("step ")
        .and_then(|rest| rest.split(char::is_whitespace).next())
        .filter(|s| !s.is_empty());
    if let Some(step) = step {
        app.get_mut::<State>().step = Some(step.to_owned());
    }
}

pub(super) fn command_install_finished(
    app: &mut AppState,
    fx: &mut Effects,
    done: CommandInstallFinished,
) {
    let now = done.now.0;
    let auto_first = std::mem::take(&mut app.get_mut::<State>().local.auto_first_install);
    app.get_mut::<State>().local.running = false;
    if done.ok {
        let version = done.version.clone().or(done.installed_version.clone());
        save(
            app,
            fx,
            &[
                ("binPath", text_or_null(&done.bin_path)),
                ("version", text_or_null(&version)),
                ("error", Json::Null),
            ],
            now,
        );
        let st = app.get_mut::<State>();
        st.installing = None;
        st.step = None;
        st.installed_version = done.installed_version;
        st.bin_path = done.bin_path;
        st.error = None;
        if auto_first {
            finish_auto_install(app, fx, now);
        }
    } else {
        let error = done
            .error
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| "The installer stopped.".into());
        save(app, fx, &[("error", Json::String(error.clone()))], now);
        let st = app.get_mut::<State>();
        st.installing = None;
        st.step = None;
        st.installed_version = done.installed_version;
        st.error = Some(error);
    }
}

fn finish_auto_install(app: &mut AppState, fx: &mut Effects, now: f64) {
    save(app, fx, &[("autoInstalledAt", Json::Number(now))], now);
    if program_control_on(app) || user_turned_control_off(app) {
        return;
    }
    set_program_control(app, fx, true);
}

fn install_for_the_first_time(app: &mut AppState, fx: &mut Effects, reads: &InstallReads) {
    if user_turned_control_off(app) || !reads.can_install_unattended {
        return;
    }
    let tries = count(&app.get::<State>().local.saved, "autoInstallTries");
    if tries >= AUTO_INSTALL_TRIES {
        return;
    }
    save(
        app,
        fx,
        &[("autoInstallTries", Json::Number(tries + 1.0))],
        reads.now.0,
    );
    app.get_mut::<State>().local.auto_first_install = true;
    install_command_in_background(app, fx, crate::telemetry::CliInstallReason::Auto);
}

pub(super) fn install_or_upgrade_command(
    app: &mut AppState,
    fx: &mut Effects,
    reads: InstallReads,
) {
    let installed = reads.installed_version.clone().filter(|v| !v.is_empty());
    let migrations: Migrations = [(0, Box::new(|doc| Json::Object(unchanged(doc))) as _)].into();
    let record = parse_saved(
        reads.saved.as_deref(),
        DOC_VERSION,
        &migrations,
        SCHEMA_FIELD,
    )
    .unwrap_or_default();
    let st = app.get_mut::<State>();
    st.installed_version = reads.installed_version.clone();
    st.local.saved = record.clone();
    if installed.is_none() {
        if never_installed(&record) || auto_install_unfinished(&record) {
            install_for_the_first_time(app, fx, &reads);
        }
        return;
    }
    if auto_install_unfinished(&record) {
        finish_auto_install(app, fx, reads.now.0);
    }
    let current = reads.app_version.as_deref().unwrap_or("");
    if current.is_empty() || installed.as_deref() == Some(current) {
        return;
    }
    install_command_in_background(app, fx, crate::telemetry::CliInstallReason::Upgrade);
}

/// `describeCommandInstall`.
pub fn describe_command_install(state: &State) -> String {
    if state.installing.is_some() {
        return match &state.step {
            Some(step) if !step.is_empty() => format!("Installing ({step})…"),
            _ => "Installing…".into(),
        };
    }
    if let Some(error) = state.error.as_ref().filter(|e| !e.is_empty()) {
        return error.clone();
    }
    match (&state.installed_version, &state.bin_path) {
        (Some(v), bin) if !v.is_empty() => match bin.as_ref().filter(|b| !b.is_empty()) {
            Some(bin) => format!("Installed at {bin}"),
            None => format!("Version {v} is installed."),
        },
        _ => "Not installed.".into(),
    }
}
