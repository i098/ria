// Ported from ardia packages/ardiacore/ios/ArdiaTabStrip.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::{BTreeMap, HashMap};

use objc2::rc::{Retained, Weak};
use ria_engine::types::{
    Number, StripGroup, StripState, StripTab, TabStripCommand, TabStripTransaction,
};
use serde_json::{Value, json};

use super::ardia_tab_registry::TabRegistry;
use crate::core::bindings::{Profile, Tab, Window};

pub struct StripWindow<'a> {
    pub window: &'a Window,
    pub profile: &'a Profile,
    pub profile_name: &'a str,
    pub app_window: Option<&'a str>,
    pub host_changes: bool,
    pub standalone: bool,
}

#[derive(Default, PartialEq)]
struct Groups {
    groups: Vec<StripGroup>,
    tabs: HashMap<i32, String>,
    active_picked_on_close: bool,
}

#[derive(Default)]
pub struct TabStrips {
    revision: u64,
    sent: HashMap<i32, StripState>,
    known: HashMap<isize, BTreeMap<i32, String>>,
    groups: HashMap<i32, Groups>,
    active: HashMap<i32, Weak<Tab>>,
    pinned: HashMap<*const Tab, Weak<Tab>>,
}

pub enum StripCommandResult {
    Done(TabStripTransaction),
    Group { strip: i32, args: Value },
}

impl TabStrips {
    /// Apply the engine's `tabs.strip` event. Return the strip ID if the public group state changed.
    pub fn observe_engine_strip(&mut self, event: &Value) -> Option<i32> {
        let id = i32::try_from(event.get("window")?.as_i64()?).ok()?;
        if event["closed"].as_bool() == Some(true) {
            self.groups.remove(&id);
            return None;
        }
        let tabs = event["tabs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|tab| {
                Some((
                    i32::try_from(tab["tab"].as_i64()?).ok()?,
                    tab["group"].as_str()?.to_owned(),
                ))
            })
            .collect();
        let groups = event["groups"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|group| serde_json::from_value(group.clone()).ok())
            .collect();
        let next = Groups {
            groups,
            tabs,
            active_picked_on_close: event["activePickedOnClose"].as_bool().unwrap_or(false),
        };
        if self.groups.get(&id) == Some(&next) {
            return None;
        }
        self.groups.insert(id, next);
        Some(id)
    }

    pub fn activated(&mut self, strip: i32, tab: &Tab) {
        self.active.insert(strip, Weak::new(tab));
    }

    pub fn active(&self, strip: i32) -> Option<Retained<Tab>> {
        self.active.get(&strip)?.load()
    }

    pub fn removed(&mut self, strip: i32, tab: &Tab) {
        if self
            .active(strip)
            .is_some_and(|active| std::ptr::eq(&*active, tab))
        {
            self.active.remove(&strip);
        }
        self.pinned.remove(&std::ptr::from_ref(tab));
    }

    pub fn set_pinned(&mut self, tab: &Tab, pinned: bool) {
        let key = std::ptr::from_ref(tab);
        if pinned {
            self.pinned.retain(|_, tab| tab.load().is_some());
            self.pinned.insert(key, Weak::new(tab));
        } else {
            self.pinned.remove(&key);
        }
    }

    fn snapshot(&mut self, scope: &StripWindow<'_>, registry: &mut TabRegistry) -> StripState {
        let window = scope.window.window().windowNumber();
        let id = scope.window.chrome_window_id_for_profile(scope.profile);
        let active = self.active(id);
        let groups = self.groups.get(&id);
        let rows = scope.window.tabs_for_profile(scope.profile);
        let tabs = rows
            .iter()
            .enumerate()
            .map(|(index, tab)| StripTab {
                key: registry.app_tab_id(&tab).map(str::to_owned),
                browser: registry.browser_id(&tab).into(),
                index: Number(index as f64),
                active: active
                    .as_ref()
                    .map_or(index == 0, |active| std::ptr::eq(&**active, &*tab)),
                pinned: self
                    .pinned
                    .get(&std::ptr::from_ref(&*tab))
                    .and_then(Weak::load)
                    .is_some(),
                group: groups.and_then(|group| group.tabs.get(&tab.tab_id()).cloned()),
            })
            .collect();
        if id >= 0 {
            self.known
                .entry(window)
                .or_default()
                .insert(id, scope.profile_name.into());
        }
        StripState {
            strip: id.into(),
            window: Number(window as f64),
            app_window: scope.app_window.map(str::to_owned),
            profile: scope.profile_name.into(),
            tabs,
            active_picked_on_close: groups
                .and_then(|group| group.active_picked_on_close.then_some(true)),
            groups: Some(groups.map_or_else(Vec::new, |group| group.groups.clone())),
            closed: None,
        }
    }

    fn send(
        &mut self,
        command: Option<Number>,
        strips: Vec<StripState>,
        rejected: bool,
    ) -> TabStripTransaction {
        self.revision += 1;
        for strip in &strips {
            let id = strip.strip.0 as i32;
            if strip.closed == Some(true) {
                self.sent.remove(&id);
            } else {
                self.sent.insert(id, strip.clone());
            }
        }
        TabStripTransaction {
            rev: Number(self.revision as f64),
            cmd: command,
            strips,
            rejected: rejected.then_some(true),
        }
    }

    pub fn changed(
        &mut self,
        scope: &StripWindow<'_>,
        registry: &mut TabRegistry,
        cause: Option<Number>,
    ) -> Option<TabStripTransaction> {
        if scope.standalone {
            return None;
        }
        let cause = cause.or_else(|| scope.host_changes.then_some(Number(-1.0)));
        let strip = self.snapshot(scope, registry);
        if cause.is_none_or(|cause| cause == Number(-1.0))
            && self.sent.get(&(strip.strip.0 as i32)) == Some(&strip)
        {
            return None;
        }
        Some(self.send(cause, vec![strip], false))
    }

    pub fn window_closed(&mut self, window: isize) -> Option<TabStripTransaction> {
        let strips = self
            .known
            .remove(&window)?
            .into_iter()
            .map(|(id, profile)| {
                self.active.remove(&id);
                StripState {
                    strip: id.into(),
                    window: Number(window as f64),
                    profile,
                    tabs: Vec::new(),
                    app_window: None,
                    active_picked_on_close: None,
                    groups: None,
                    closed: Some(true),
                }
            })
            .collect();
        Some(self.send(None, strips, false))
    }

    pub fn all(
        &mut self,
        scopes: &[StripWindow<'_>],
        registry: &mut TabRegistry,
    ) -> TabStripTransaction {
        let mut strips = Vec::new();
        for scope in scopes.iter().filter(|scope| !scope.standalone) {
            let strip = self.snapshot(scope, registry);
            self.sent.insert(strip.strip.0 as i32, strip.clone());
            strips.push(strip);
        }
        TabStripTransaction {
            rev: Number(self.revision as f64),
            cmd: None,
            strips,
            rejected: None,
        }
    }

    pub fn command(
        &mut self,
        number: Number,
        command: TabStripCommand,
        scopes: &[StripWindow<'_>],
        registry: &mut TabRegistry,
    ) -> StripCommandResult {
        let strip = match &command {
            TabStripCommand::Activate { strip, .. }
            | TabStripCommand::Arrange { strip, .. }
            | TabStripCommand::Group { strip, .. } => strip.0 as i32,
        };
        let Some(scope) = scopes
            .iter()
            .rfind(|scope| scope.window.chrome_window_id_for_profile(scope.profile) == strip)
        else {
            return StripCommandResult::Done(self.send(Some(number), vec![], true));
        };
        let tab_list = scope.window.tabs_for_profile(scope.profile);
        let by_key: HashMap<_, _> = tab_list
            .iter()
            .filter_map(|tab| Some((registry.app_tab_id(&tab)?, tab)))
            .collect();
        let rejected = match command {
            TabStripCommand::Activate { key, .. } => {
                if let Some(tab) = by_key.get(key.as_str()) {
                    scope.window.activate_tab(tab);
                    false
                } else {
                    true
                }
            }
            TabStripCommand::Arrange { keys, pinned, .. } => {
                let mut placed = 0;
                for (index, key) in keys.iter().enumerate() {
                    if let Some(tab) = by_key.get(key.as_str()) {
                        let pinned = (index as f64) < pinned.0;
                        self.set_pinned(tab, pinned);
                        scope.window.place_tab(tab, placed, pinned);
                        placed += 1;
                    }
                }
                !keys.is_empty() && placed == 0
            }
            TabStripCommand::Group {
                keys,
                group,
                title,
                color,
                collapsed,
                ..
            } => {
                let ids: Vec<_> = keys
                    .iter()
                    .filter_map(|key| by_key.get(key.as_str()).map(|tab| tab.tab_id()))
                    .collect();
                if !ids.is_empty() {
                    let mut args =
                        json!({ "window": strip, "tabs": ids, "group": group.unwrap_or_default() });
                    if let Some(title) = title {
                        args["title"] = title.into();
                    }
                    if let Some(color) = color.filter(Value::is_string) {
                        args["color"] = color;
                    }
                    if let Some(collapsed) = collapsed {
                        args["collapsed"] = collapsed.into();
                    }
                    return StripCommandResult::Group { strip, args };
                }
                true
            }
        };
        StripCommandResult::Done(self.group_finished(number, scope, registry, rejected))
    }

    pub fn group_finished(
        &mut self,
        number: Number,
        scope: &StripWindow<'_>,
        registry: &mut TabRegistry,
        rejected: bool,
    ) -> TabStripTransaction {
        let snapshot = self.snapshot(scope, registry);
        self.send(Some(number), vec![snapshot], rejected)
    }
}
