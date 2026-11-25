// Ported from ardia apps/browser/src/lib/pluginActivation.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! When a plugin starts: checking the `activationEvents` it lists, and the events its
//! contributions imply.
//!
//! Changes from Ardia:
//! - `unknown` is `Option<&Json>`: `None` is `undefined`.
//! - `derivedActivationEvents` takes `main` and `sidecar` as two arguments, not one object.

use super::plugin_checks::{Check, KEY, LIMITS, NAME, at, list};
use super::plugin_contributions::{MENU_SLOTS, PluginContributions};
use super::plugin_network::is_host_pattern;
use regex::Regex;
use ria_core::identity::telemetry_wire::Json;
use std::sync::LazyLock;

/// `STARTUP_EVENTS` (`pluginActivation.ts:5`).
pub const STARTUP_EVENTS: [&str; 2] = ["*", "onStartupFinished"];
/// `ACTIVATION_PREFIXES` (`pluginActivation.ts:6`).
pub const ACTIVATION_PREFIXES: [&str; 8] = [
    "onCommand",
    "onView",
    "onPanel",
    "onToolbar",
    "onOmnibox",
    "onMenu",
    "onEvent",
    "onUrl",
];
/// `ActivationPrefix` (`pluginActivation.ts:7`).
pub type ActivationPrefix = &'static str;

static EVENT_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-zA-Z]*(\.[a-z][a-zA-Z]*)+$").expect("EVENT_NAME"));

/// `argumentOk` (`pluginActivation.ts:11`).
fn argument_ok(prefix: &str, arg: &str) -> bool {
    match prefix {
        "onCommand" => NAME.is_match(arg),
        "onMenu" => MENU_SLOTS.contains(&arg),
        "onEvent" => arg == "*" || EVENT_NAME.is_match(arg),
        "onUrl" => is_host_pattern(&Json::String(arg.to_owned())) && arg != "*",
        _ => KEY.is_match(arg),
    }
}

/// `activationEventProblem` (`pluginActivation.ts:26`).
pub fn activation_event_problem(event: Option<&Json>) -> Option<String> {
    let Some(Json::String(event)) = event else {
        return Some("must be text".to_owned());
    };
    if STARTUP_EVENTS.contains(&event.as_str()) {
        return None;
    }
    let colon = event.find(':');
    let prefix = colon.map_or(event.as_str(), |i| &event[..i]);
    if !ACTIVATION_PREFIXES.contains(&prefix) {
        let list: Vec<String> = ACTIVATION_PREFIXES
            .iter()
            .map(|p| format!("{p}:"))
            .collect();
        return Some(format!(
            "must be *, onStartupFinished or start with {}",
            list.join(", ")
        ));
    }
    match colon {
        Some(i) if argument_ok(prefix, &event[i + 1..]) => None,
        _ => Some(format!("has a bad {prefix} argument")),
    }
}

/// `activationEventsOf` (`pluginActivation.ts:36`).
pub fn activation_events_of(c: &mut Check, value: Option<&Json>) -> Option<Vec<String>> {
    value?;
    let mut out: Vec<String> = Vec::new();
    for (i, event) in list(c, "activationEvents", value, LIMITS.activation_events)
        .iter()
        .enumerate()
    {
        if let Some(problem) = activation_event_problem(Some(event)) {
            c.problems
                .push(format!("{} {problem}", at("activationEvents", i)));
        } else if let Json::String(event) = event
            && !out.contains(event)
        {
            out.push(event.clone());
        }
    }
    Some(out)
}

/// `derivedActivationEvents` (`pluginActivation.ts:47`): each event once, in first-seen order.
pub fn derived_activation_events(
    k: &PluginContributions,
    main: bool,
    sidecar: bool,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |e: String| {
        if !out.contains(&e) {
            out.push(e);
        }
    };
    if !k.hooks.is_empty() || (sidecar && !main) {
        add("onStartupFinished".to_owned());
    }
    k.commands
        .iter()
        .for_each(|x| add(format!("onCommand:{}", x.name)));
    k.sidebar_views
        .iter()
        .for_each(|v| add(format!("onView:{}", v.id)));
    k.panels
        .iter()
        .filter(|p| p.view.is_some())
        .for_each(|p| add(format!("onPanel:{}", p.id)));
    k.toolbar
        .iter()
        .for_each(|t| add(format!("onToolbar:{}", t.id)));
    k.omnibox
        .iter()
        .for_each(|o| add(format!("onOmnibox:{}", o.id)));
    MENU_SLOTS
        .iter()
        .filter(|s| !k.menus.slot(s).is_empty())
        .for_each(|s| add(format!("onMenu:{s}")));
    out
}

/// `startsEagerly` (`pluginActivation.ts:59`).
pub fn starts_eagerly(events: &[String]) -> bool {
    events
        .iter()
        .any(|e| e == "*" || e == "onStartupFinished" || e.starts_with("onEvent:"))
}

/// `urlActivationHosts` (`pluginActivation.ts:61`).
pub fn url_activation_hosts(events: &[String]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| e.strip_prefix("onUrl:"))
        .map(str::to_owned)
        .collect()
}
