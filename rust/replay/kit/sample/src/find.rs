// Ported from ardia apps/browser/src/state/find/actions.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::BTreeMap;
use std::mem;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    Mount {
        tab: String,
        r#match: Matches,
    },
    Open {
        tab: String,
        now: i64,
        #[serde(default)]
        options: OpenOptions,
    },
    Close {
        tab: String,
    },
    Step {
        tab: String,
        backwards: bool,
    },
    TakeFocus {
        tab: String,
    },
    SetMatches {
        tab: String,
        count: Option<u32>,
        active: u32,
    },
    SubmitRepeatsKey {
        key_at: i64,
        now: i64,
    },
    TakeCounts,
}

#[derive(Default, Deserialize)]
pub struct OpenOptions {
    query: Option<String>,
    replace: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct Matches {
    count: u32,
    active: u32,
}

// Ardia's setFind merges patches into CLOSED_FIND, only for known tabs.
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct FindState<'a> {
    open: bool,
    query: &'a str,
    count: Option<u32>,
    active: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_request: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replace: Option<bool>,
}

#[derive(Default, Serialize)]
struct Counts {
    find_opens: u32,
    find_steps: u32,
}

#[derive(Default)]
struct State<'a> {
    find: BTreeMap<&'a str, FindState<'a>>,
    counts: Counts,
}

enum Effect<'a> {
    None,
    Close(&'a str),
    Find {
        tab: &'a str,
        query: &'a str,
        forwards: bool,
    },
}

impl<'a> State<'a> {
    fn entry(&mut self, tab: &'a str) -> Option<&mut FindState<'a>> {
        matches!(tab, "a" | "b").then(|| self.find.entry(tab).or_default())
    }

    fn open(&mut self, tab: &'a str) -> Option<&mut FindState<'a>> {
        if !self.find.get(tab).is_some_and(|state| state.open) {
            self.counts.find_opens += 1;
        }
        let state = self.entry(tab)?;
        state.open = true;
        Some(state)
    }

    fn apply(&mut self, action: &'a Action) -> (Effect<'a>, Option<Value>) {
        let mut effect = Effect::None;
        match action {
            Action::Mount { .. } => {}
            Action::Open { tab, now, options } => {
                if let Some(state) = self.open(tab) {
                    state.focus_request = Some(*now);
                    if let Some(query) = &options.query {
                        state.query = query;
                    }
                    if let Some(replace) = options.replace {
                        state.replace = Some(replace);
                    }
                }
            }
            Action::Close { tab } => {
                if let Some(state) = self.entry(tab) {
                    state.open = false;
                    state.count = None;
                    state.active = 0;
                }
                effect = Effect::Close(tab);
            }
            Action::Step { tab, backwards } => {
                if let Some(query) = self
                    .find
                    .get(tab.as_str())
                    .map(|s| s.query)
                    .filter(|s| !s.is_empty())
                {
                    self.counts.find_steps += 1;
                    effect = Effect::Find {
                        tab,
                        query,
                        forwards: !backwards,
                    };
                } else {
                    self.open(tab);
                }
            }
            Action::TakeFocus { tab } => {
                if let Some(state) = self.entry(tab) {
                    state.focus_request = None;
                }
            }
            Action::SetMatches { tab, count, active } => {
                if let Some(state) = self.entry(tab) {
                    state.count = *count;
                    state.active = *active;
                }
            }
            Action::SubmitRepeatsKey { key_at, now } => {
                return (
                    effect,
                    Some(json!(i128::from(*now) - i128::from(*key_at) < 250)),
                );
            }
            Action::TakeCounts => return (effect, Some(json!(mem::take(&mut self.counts)))),
        }
        (effect, None)
    }
}

pub fn replay(actions: &[Action]) -> Vec<Value> {
    let mut state = State::default();
    let mut webviews = BTreeMap::new();
    actions
        .iter()
        .map(|action| {
            if let Action::Mount { tab, r#match } = action {
                webviews.insert(tab.as_str(), r#match);
            }
            let (effect, result) = state.apply(action);
            let calls = match effect {
                Effect::Close(tab) if webviews.contains_key(tab) => json!([
                    {"name": "stopFinding", "args": [false]},
                    {"name": "focus", "args": []},
                ]),
                Effect::Find {
                    tab,
                    query,
                    forwards,
                } => match webviews.get(tab) {
                    Some(result) => {
                        json!([{"name": "find", "args": [query, forwards, true], "result": result}])
                    }
                    None => json!([]),
                },
                _ => json!([]),
            };
            match result {
                Some(result) => json!({"state": state.find, "calls": calls, "result": result}),
                None => json!({"state": state.find, "calls": calls}),
            }
        })
        .collect()
}
