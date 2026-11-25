// Ported from ardia packages/sync/src/companion/state.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The saved state of the phone companion, the host that stores it, and the module globals that
//! keep the loaded state and the last saved and shown JSON.
//!
//! Changes from Ardia:
//! - The module globals and `globalThis.__host` are thread locals; `register_host` sets the host.
//! - A host method returns `SyncResult`, so a host can fail as a JS host throws.
//! - `loaded()` cannot lend the state out of the thread local: `with_loaded` runs a function on
//!   it. The cycle takes the state out while it runs (`take_loaded`, `put_loaded`).
//! - `companion_load_at` takes the time that Ardia reads from `Date.now()`; `companion_load`
//!   gives it the system time.
//! - `CompanionState` is typed. A saved state that lacks an optional field gets it at its place in
//!   the type, not at the end; unknown fields are dropped; a malformed state or journal returns
//!   the `serde_json` error text. A falsy `state` counts as no state.
//! - JS writes the optional keys of a scope state (`pendingSince`, `applyAs`, `journal`,
//!   `awaitRemote`) in the order the code added them. `late_keys` keeps that order for the saved
//!   JSON, from the loaded JSON and from each sync of a scope (`note_late_keys`).

use super::adapters::{Ack, BookmarkOp, PendingSend, truthy};
use crate::logic::hlc::Clock;
use crate::logic::js::{cmp_utf16, now_ms, number_to_string, stringify};
use crate::logic::scope::restore_journal;
use crate::logic::scope_state::{LogFile, ScopeState, SyncResult, empty_scope};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// The native side: the chain status, the files of the folder, the journal and the saved state.
pub trait Host {
    fn chain_status(&self) -> SyncResult<String>;
    fn read(&self, scope: &str, known_json: &str) -> SyncResult<String>;
    fn write(&self, scope: &str, payload: &str) -> SyncResult<String>;
    fn remove(&self, scope: &str, ids_json: &str) -> SyncResult<String>;
    fn persist_journal(&self, scope: &str, batches_json: &str) -> SyncResult<String>;
    fn save_state(&self, state_json: &str, seqs_json: &str) -> SyncResult<String>;
}

/// A device id that this phone had before, with the time when it was seen as a ghost.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ghost {
    pub id: String,
    pub since: f64,
}

/// `null` reads as the default, as `??=` does.
fn or_default<'de, D: Deserializer<'de>, T: Default + Deserialize<'de>>(
    d: D,
) -> Result<T, D::Error> {
    Option::<T>::deserialize(d).map(Option::unwrap_or_default)
}

/// Port of `CompanionState` (`packages/sync/src/companion/state.ts:15`); the fields that
/// `companionLoad` fills are always present.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompanionState {
    pub v: u8,
    pub device: String,
    #[serde(default)]
    pub clock: Option<String>,
    #[serde(default, deserialize_with = "or_default")]
    pub scopes: IndexMap<String, ScopeState>,
    #[serde(default, deserialize_with = "or_default")]
    pub sends: Vec<PendingSend>,
    #[serde(default, deserialize_with = "or_default")]
    pub acks: Vec<Ack>,
    #[serde(default, deserialize_with = "or_default")]
    pub bookmarks: Vec<BookmarkOp>,
    #[serde(default, deserialize_with = "or_default")]
    pub floors: IndexMap<String, IndexMap<String, f64>>,
    #[serde(default, deserialize_with = "or_default")]
    pub pulled: IndexMap<String, bool>,
    #[serde(default, deserialize_with = "or_default")]
    pub visited: IndexMap<String, IndexMap<String, f64>>,
    #[serde(default, deserialize_with = "or_default")]
    pub due: IndexMap<String, IndexMap<String, bool>>,
    #[serde(default, deserialize_with = "or_default")]
    pub removals: IndexMap<String, Vec<String>>,
    #[serde(default, deserialize_with = "or_default")]
    pub ghosts: Vec<Ghost>,
    #[serde(default, deserialize_with = "or_default")]
    pub sent: IndexMap<String, f64>,
    /// For each scope, its optional keys in the order JS added them.
    #[serde(skip)]
    pub(crate) late_keys: IndexMap<String, Vec<&'static str>>,
}

/// The optional keys of a scope state, in the order one `syncScope` can add them.
const LATE_KEYS: [&str; 4] = ["pendingSince", "applyAs", "journal", "awaitRemote"];

/// Keeps `order` as JS keeps the keys of `s`: a lost key goes, a new key goes to the end.
/// `journal_rebuilt` tells that the journal was deleted and made again.
pub(crate) fn note_late_keys(order: &mut Vec<&'static str>, s: &ScopeState, journal_rebuilt: bool) {
    let has = |k: &str| match k {
        "pendingSince" => s.pending_since.is_some(),
        "applyAs" => s.apply_as.is_some(),
        "journal" => s.journal.is_some(),
        _ => s.await_remote.is_some(),
    };
    order.retain(|k| has(k) && !(journal_rebuilt && *k == "journal"));
    for k in LATE_KEYS {
        if has(k) && !order.contains(&k) {
            order.push(k);
        }
    }
}

thread_local! {
    static COMPANION: RefCell<Option<CompanionState>> = const { RefCell::new(None) };
    /// `savedJSON` and `savedPrint`.
    static SAVED: RefCell<(String, String)> = const { RefCell::new((String::new(), String::new())) };
    static VIEW_JSON: RefCell<String> = const { RefCell::new(String::new()) };
    static HOST: RefCell<Option<Rc<dyn Host>>> = const { RefCell::new(None) };
}

/// Sets (or, with `None`, clears) the host: `globalThis.__host = host` in Ardia.
pub fn register_host(host: Option<Rc<dyn Host>>) {
    HOST.with(|h| *h.borrow_mut() = host);
}

/// Port of `host` (`packages/sync/src/companion/state.ts:37`).
pub fn host() -> SyncResult<Rc<dyn Host>> {
    HOST.with(|h| h.borrow().clone())
        .ok_or_else(|| "the sync host isn't registered".to_string())
}

const NOT_LOADED: &str = "the companion isn't loaded";

/// Port of `loaded` (`packages/sync/src/companion/state.ts:43`): runs `f` on the loaded state.
pub fn with_loaded<T>(f: impl FnOnce(&mut CompanionState) -> SyncResult<T>) -> SyncResult<T> {
    COMPANION.with(|c| match c.borrow_mut().as_mut() {
        Some(state) => f(state),
        None => Err(NOT_LOADED.to_string()),
    })
}

/// Takes the loaded state out, for a cycle that awaits while it changes the state.
pub(crate) fn take_loaded() -> SyncResult<CompanionState> {
    COMPANION
        .with(|c| c.borrow_mut().take())
        .ok_or_else(|| NOT_LOADED.to_string())
}

/// Puts back the state that `take_loaded` took.
pub(crate) fn put_loaded(state: CompanionState) {
    COMPANION.with(|c| *c.borrow_mut() = Some(state));
}

/// Port of `freshState` (`packages/sync/src/companion/state.ts:48`).
fn fresh_state(device: &str) -> CompanionState {
    CompanionState {
        v: 1,
        device: device.to_string(),
        ..Default::default()
    }
}

/// The input of `companionLoad`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct LoadInput {
    pub device: String,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub journal: Option<IndexMap<String, Value>>,
    #[serde(default)]
    pub fresh: Option<Value>,
    #[serde(default)]
    pub ghosts: Option<Vec<String>>,
}

/// What `companionLoad` returns.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Loaded {
    pub device: String,
    pub scopes: usize,
}

/// Port of `companionLoad` (`packages/sync/src/companion/state.ts:50`) at the system time.
pub fn companion_load(input: &LoadInput) -> SyncResult<Loaded> {
    companion_load_at(input, now_ms())
}

/// Port of `companionLoad` (`packages/sync/src/companion/state.ts:50`); `now` is `Date.now()`.
pub fn companion_load_at(input: &LoadInput, now: f64) -> SyncResult<Loaded> {
    let given = input.state.as_ref().filter(|s| truthy(s));
    if given.is_none() && !input.fresh.as_ref().is_some_and(truthy) {
        return Err("no saved state for this device".into());
    }
    let mut state = match given {
        Some(s) => {
            let scopes = s.get("scopes");
            if s.get("v").and_then(Value::as_f64) != Some(1.0)
                || s.get("device").and_then(Value::as_str) != Some(input.device.as_str())
                || !scopes.is_some_and(|v| truthy(v) && (v.is_object() || v.is_array()))
            {
                return Err("the saved state belongs to another device or version".into());
            }
            let mut state = CompanionState::deserialize(s).map_err(|e| e.to_string())?;
            for (name, scope) in scopes.and_then(Value::as_object).into_iter().flatten() {
                let keys = scope.as_object().into_iter().flat_map(|m| m.keys());
                state.late_keys.insert(
                    name.clone(),
                    keys.filter_map(|k| LATE_KEYS.into_iter().find(|l| l == k))
                        .collect(),
                );
            }
            state
        }
        None => fresh_state(&input.device),
    };
    for id in input.ghosts.iter().flatten() {
        if *id != input.device && !state.ghosts.iter().any(|g| g.id == *id) {
            state.ghosts.push(Ghost {
                id: id.clone(),
                since: now,
            });
        }
    }
    let mut clock = Clock::new(
        &input.device,
        state.clock.as_deref(),
        Some(Rc::new(move || now)),
    );
    let mut restored = false;
    for (scope, batches) in input.journal.iter().flatten() {
        let mut mine = Vec::new();
        for f in batches.as_array().into_iter().flatten() {
            if truthy(f) && f.get("device").and_then(Value::as_str) == Some(input.device.as_str()) {
                mine.push(LogFile::deserialize(f).map_err(|e| e.to_string())?);
            }
        }
        if mine.is_empty() {
            continue;
        }
        restore_journal(
            state
                .scopes
                .entry(scope.clone())
                .or_insert_with(empty_scope),
            &mine,
            &mut clock,
            0,
        );
        restored = true;
    }
    if restored {
        state.clock = Some(clock.last());
    }
    let CompanionState {
        scopes, late_keys, ..
    } = &mut state;
    for (name, s) in scopes.iter() {
        note_late_keys(late_keys.entry(name.clone()).or_default(), s, false);
    }
    let out = Loaded {
        device: state.device.clone(),
        scopes: state.scopes.len(),
    };
    put_loaded(state);
    SAVED.with(|s| *s.borrow_mut() = (String::new(), String::new()));
    VIEW_JSON.with(|v| v.borrow_mut().clear());
    Ok(out)
}

/// Port of `companionUnload` (`packages/sync/src/companion/state.ts:84`).
pub fn companion_unload() -> bool {
    COMPANION.with(|c| *c.borrow_mut() = None);
    SAVED.with(|s| *s.borrow_mut() = (String::new(), String::new()));
    VIEW_JSON.with(|v| v.borrow_mut().clear());
    true
}

/// Port of `companionStateUnsaved` (`packages/sync/src/companion/state.ts:92`).
pub fn companion_state_unsaved() -> bool {
    SAVED.with(|s| *s.borrow_mut() = (String::new(), String::new()));
    true
}

/// `JSON.stringify` of a serialisable value.
pub(crate) fn json_of(value: &impl Serialize) -> SyncResult<String> {
    serde_json::to_value(value)
        .map(|v| stringify(&v))
        .map_err(|e| e.to_string())
}

/// Port of `fingerprint` (`packages/sync/src/companion/state.ts:98`).
fn fingerprint(c: &CompanionState) -> SyncResult<String> {
    let mut names: Vec<&String> = c.scopes.keys().collect();
    names.sort_by(|a, z| cmp_utf16(a, z));
    let mut scopes = Vec::with_capacity(names.len());
    for name in names {
        let s = &c.scopes[name];
        let count = |n: usize| number_to_string(n as f64);
        scopes.push(
            [
                name.clone(),
                number_to_string(s.seq as f64),
                count(s.journal.as_ref().map_or(0, Vec::len)),
                count(s.outbox.len()),
                count(s.files.len()),
                number_to_string(s.pending as f64),
                count(s.damaged.len()),
                count(s.unapplied.len()),
                s.joined.to_string(),
                json_of(&s.vv)?,
                json_of(&s.extra)?,
                json_of(&s.seen)?,
            ]
            .join("|"),
        );
    }
    json_of(&(
        &c.clock,
        scopes,
        &c.sends,
        &c.acks,
        &c.bookmarks,
        &c.floors,
        &c.pulled,
        &c.visited,
        &c.due,
        &c.removals,
        &c.ghosts,
        &c.sent,
    ))
}

/// `JSON.stringify(c)`, with the optional keys of each scope state in the order JS added them.
fn state_json(c: &CompanionState) -> SyncResult<String> {
    let mut v = serde_json::to_value(c).map_err(|e| e.to_string())?;
    for (name, s) in v
        .get_mut("scopes")
        .and_then(Value::as_object_mut)
        .into_iter()
        .flatten()
    {
        let (Some(map), Some(order)) = (s.as_object_mut(), c.late_keys.get(name)) else {
            continue;
        };
        for key in order {
            if let Some(x) = map.shift_remove(*key) {
                map.insert((*key).to_string(), x);
            }
        }
    }
    Ok(stringify(&v))
}

/// Port of `save` (`packages/sync/src/companion/state.ts:108`) for the loaded state `c`.
pub(crate) fn save_of(c: &CompanionState, force: bool) -> SyncResult<bool> {
    let print = fingerprint(c)?;
    let (saved_json, saved_print) = SAVED.with(|s| s.borrow().clone());
    if !force && !saved_json.is_empty() && print == saved_print {
        return Ok(true);
    }
    let json = state_json(c)?;
    if json == saved_json {
        SAVED.with(|s| s.borrow_mut().1 = print);
        return Ok(true);
    }
    let seqs: IndexMap<&String, u64> = c.scopes.iter().map(|(scope, s)| (scope, s.seq)).collect();
    if host()?.save_state(&json, &json_of(&seqs)?)? != "ok" {
        return Ok(false);
    }
    SAVED.with(|s| *s.borrow_mut() = (json, print));
    Ok(true)
}

/// Port of `save` (`packages/sync/src/companion/state.ts:108`).
pub fn save(force: bool) -> SyncResult<bool> {
    with_loaded(|c| save_of(c, force))
}

/// Port of `viewIsNew` (`packages/sync/src/companion/state.ts:124`).
pub fn view_is_new(json: &str) -> bool {
    VIEW_JSON.with(|v| {
        let mut v = v.borrow_mut();
        if *v == json {
            return false;
        }
        json.clone_into(&mut v);
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_keys_follow_js_insertion_order() {
        let mut s = empty_scope();
        let mut order = Vec::new();
        s.apply_as = Some(IndexMap::new());
        s.journal = Some(Vec::new());
        note_late_keys(&mut order, &s, false);
        assert_eq!(order, ["applyAs", "journal"]);
        // A new key goes to the end.
        s.pending_since = Some(IndexMap::new());
        note_late_keys(&mut order, &s, false);
        assert_eq!(order, ["applyAs", "journal", "pendingSince"]);
        // A journal deleted and made again within one sync goes to the end.
        note_late_keys(&mut order, &s, true);
        assert_eq!(order, ["applyAs", "pendingSince", "journal"]);
        s.apply_as = None;
        note_late_keys(&mut order, &s, false);
        assert_eq!(order, ["pendingSince", "journal"]);
    }
}
