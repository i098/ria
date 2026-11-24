// Ported from ardia packages/ardiacore/ios/ArdiaTabRegistry.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use objc2::rc::{Retained, Weak};
use objc2_app_kit::NSView;
use serde_json::Value;

use crate::core::bindings::Tab;

const OFFER_LIFETIME: Duration = Duration::from_secs(30);

struct Entry {
    tab: Weak<Tab>,
    view: Option<Weak<NSView>>,
    app_id: String,
    browser_id: i32,
    first_show_focus: bool,
    held: Vec<(String, Value)>,
}

struct Offer {
    tab: Retained<Tab>,
    expires: Instant,
}

#[derive(Default)]
pub struct TabRegistry {
    entries: HashMap<*const Tab, Entry>,
    offers: HashMap<String, Offer>,
    next_browser_id: i32,
}

impl TabRegistry {
    fn entry(&mut self, tab: &Tab) -> &mut Entry {
        let key = std::ptr::from_ref(tab);
        if !self
            .entries
            .get(&key)
            .is_some_and(|entry| entry.tab.load().is_some())
        {
            self.entries.retain(|_, entry| entry.tab.load().is_some());
            self.next_browser_id += 1;
            self.entries.insert(
                key,
                Entry {
                    tab: Weak::new(tab),
                    view: None,
                    app_id: String::new(),
                    browser_id: self.next_browser_id,
                    first_show_focus: false,
                    held: Vec::new(),
                },
            );
        }
        self.entries.get_mut(&key).expect("tab was registered")
    }

    pub fn browser_id(&mut self, tab: &Tab) -> i32 {
        self.entry(tab).browser_id
    }

    pub fn tab_with_browser_id(&self, id: i32) -> Option<Retained<Tab>> {
        self.entries
            .values()
            .find(|entry| entry.browser_id == id)?
            .tab
            .load()
    }

    pub fn set_view(&mut self, tab: &Tab, view: Option<(&NSView, String)>) {
        let entry = self.entry(tab);
        if let Some((view, id)) = view {
            entry.view = Some(Weak::new(view));
            entry.app_id = id;
        } else {
            entry.view = None;
            entry.app_id.clear();
        }
    }

    pub fn view_for_tab(&self, tab: &Tab) -> Option<Retained<NSView>> {
        let entry = self.entries.get(&std::ptr::from_ref(tab))?;
        entry.tab.load()?;
        entry.view.as_ref()?.load()
    }

    pub fn app_tab_id(&self, tab: &Tab) -> Option<&str> {
        self.view_for_tab(tab)?;
        let id = &self.entries.get(&std::ptr::from_ref(tab))?.app_id;
        (!id.is_empty()).then_some(id)
    }

    pub fn tab_with_app_id(&self, id: &str) -> Option<Retained<Tab>> {
        if id.is_empty() {
            return None;
        }
        self.entries.values().find_map(|entry| {
            if entry.app_id != id {
                return None;
            }
            entry.view.as_ref()?.load()?;
            entry.tab.load()
        })
    }

    pub fn app_tab_id_containing(&self, view: &NSView) -> Option<&str> {
        self.entries.values().find_map(|entry| {
            if entry.app_id.is_empty() {
                return None;
            }
            entry.view.as_ref()?.load()?;
            let page = entry.tab.load()?.view()?;
            view.isDescendantOf(&page).then_some(entry.app_id.as_str())
        })
    }

    pub fn offer(&mut self, tab: Retained<Tab>, prefix: &str, now: Instant) -> String {
        let key = format!("{prefix}:{}", self.browser_id(&tab));
        self.offers.insert(
            key.clone(),
            Offer {
                tab,
                expires: now + OFFER_LIFETIME,
            },
        );
        key
    }

    pub fn offered(&self, id: &str) -> Option<&Tab> {
        self.offers.get(id).map(|offer| &*offer.tab)
    }

    pub fn take_offered(&mut self, id: &str) -> Option<Retained<Tab>> {
        self.offers.remove(id).map(|offer| offer.tab)
    }

    /// Return tabs to close after the caller releases the registry borrow.
    pub fn expire_offers(&mut self, now: Instant) -> Vec<Retained<Tab>> {
        let entries = &self.entries;
        self.offers
            .extract_if(|_, offer| offer.expires <= now)
            .filter_map(|(_, offer)| {
                let key = std::ptr::from_ref(&*offer.tab);
                let hosted = entries
                    .get(&key)
                    .and_then(|entry| entry.view.as_ref())
                    .and_then(Weak::load)
                    .is_some();
                (!hosted).then_some(offer.tab)
            })
            .collect()
    }

    pub fn expect_first_show_focus(&mut self, tab: &Tab, expects: bool) {
        self.entry(tab).first_show_focus = expects;
    }

    pub fn take_first_show_focus(&mut self, tab: &Tab) -> bool {
        std::mem::take(&mut self.entry(tab).first_show_focus)
    }

    pub fn forget(&mut self, tab: &Tab) {
        if let Some(entry) = self.entries.get_mut(&std::ptr::from_ref(tab)) {
            entry.view = None;
            entry.app_id.clear();
            entry.first_show_focus = false;
        }
        self.offers
            .retain(|_, offer| !std::ptr::eq(&*offer.tab, tab));
    }

    pub fn hold_until_hosted(&mut self, tab: &Tab, topic: String, payload: Value) {
        self.entry(tab).held.push((topic, payload));
    }

    pub fn drop_held_event(&mut self, topic: &str, id: &str) {
        for entry in self.entries.values_mut() {
            entry.held.retain(|(event, payload)| {
                event != topic || payload.get("id").and_then(Value::as_str) != Some(id)
            });
        }
    }

    pub fn release_held_events(&mut self, tab: &Tab) -> Vec<(String, Value)> {
        if self.app_tab_id(tab).is_none() {
            return Vec::new();
        }
        let entry = self.entry(tab);
        let mut held = std::mem::take(&mut entry.held);
        for (_, payload) in &mut held {
            if let Some(payload) = payload.as_object_mut() {
                payload.insert("tabId".into(), Value::String(entry.app_id.clone()));
            }
        }
        held
    }
}
