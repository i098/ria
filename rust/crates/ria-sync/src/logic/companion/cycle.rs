// Ported from ardia packages/sync/src/companion/cycle.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! One sync cycle of the phone companion over the host's folder, and the changes that the phone
//! app asks for between cycles (send a tab, acknowledge, add or remove a bookmark).
//!
//! Changes from Ardia:
//! - The cycle takes the loaded state out of the thread local while it runs and puts it back,
//!   also when it fails; the adapters reach it through an `Rc<RefCell<..>>`. A scope state is
//!   out of `scopes` while its `syncScope` runs.
//! - The floors of a profile scope are out of `floors` while its cycle runs (the history
//!   adapter shares them), and go back, also when the cycle fails.
//! - A visit that is not an object, or whose title is not a string, reads as a visit without a
//!   profile or a title; visit times that are not numbers are dropped at once (Ardia drops them
//!   where it reads them).
//! - `url in visited[scope]` and `key in sent` ask for own keys only; Ardia's `in` also finds the
//!   names of `Object.prototype`, which no address or send key is.
//! - A host reply that is not JSON, or not a listing, returns the `serde_json` error text.
//! - The inputs are typed: a missing or wrongly typed field returns the `serde_json` error text.

use super::adapters::{
    Ack, BookmarkContext, BookmarkDraft, BookmarkOp, BookmarkOpKind, DAY_MS, DropKeys,
    HISTORY_WINDOW_MS, HistoryContext, LocalTab, LocalVisit, PendingSend, SEND_LIFETIME_MS,
    SendContext, bookmarks_adapter, device_adapter, device_tabs_adapter, history_adapter,
    history_expired, is_web_url, sends_adapter, to_js_string, to_number, truthy,
};
use super::legacy::from_legacy_url;
use super::state::{
    CompanionState, host, json_of, note_late_keys, put_loaded, save_of, take_loaded, view_is_new,
    with_loaded,
};
use super::view::{View, build_view, data_owner, default_profile_id, live_profiles};
use crate::logic::hlc::Clock;
use crate::logic::js::{cmp_utf16, js_max, now_ms};
use crate::logic::records::SendRecord;
use crate::logic::scope::{SyncScopeArgs, sync_scope};
use crate::logic::scope_keys::visible;
use crate::logic::scope_state::{
    Adapter, Listing, LocalBoxFuture, LogFile, ScopeOptions, SyncResult, Transport, empty_scope,
};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Port of `CycleInput` (`packages/sync/src/companion/cycle.ts:26`).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct CycleInput {
    pub name: String,
    #[serde(default)]
    pub now: Option<f64>,
    #[serde(default)]
    pub retiring: bool,
    #[serde(default)]
    pub tabs: Option<Vec<LocalTab>>,
    /// `LocalVisit[]`, read as Ardia reads it: a visit without a string `u` or an array `vt` is skipped.
    #[serde(default)]
    pub visits: Option<Vec<Value>>,
    #[serde(default, rename = "fetchedDue")]
    pub fetched_due: Option<Vec<String>>,
}

/// Port of `Received` (`packages/sync/src/companion/cycle.ts:35`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Received {
    pub profile: String,
    pub key: String,
    pub u: String,
    pub t: String,
    pub from: String,
    #[serde(rename = "fromName")]
    pub from_name: Option<String>,
    pub at: f64,
}

/// Port of `Removal` (`packages/sync/src/companion/cycle.ts:36`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Removal {
    pub profile: String,
    pub urls: Vec<String>,
    pub unassigned: bool,
}

/// Port of `CycleOutput` (`packages/sync/src/companion/cycle.ts:38`); `view` comes last, as
/// Ardia adds it last.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CycleOutput {
    pub status: String,
    pub received: Vec<Received>,
    #[serde(rename = "historyRemoved")]
    pub history_removed: Vec<Removal>,
    #[serde(rename = "visitsDue")]
    pub visits_due: Vec<String>,
    pub published: usize,
    pub changed: usize,
    pub pending: u64,
    pub saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<View>,
}

/// What a change between cycles returns: `{ saved }`, `{ saved, existing }` or `refused`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Saved {
    pub saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing: Option<bool>,
}

const APP: &str = "app";
const GHOST_MS: f64 = 30.0 * 86_400_000.0;

/// Port of `removalsOf` (`packages/sync/src/companion/cycle.ts:53`).
fn removals_of(c: &CompanionState) -> Vec<Removal> {
    let fallback = default_profile_id(&live_profiles(c.scopes.get(APP)));
    let mut lists: Vec<(&String, &Vec<String>)> = c
        .removals
        .iter()
        .filter(|(_, urls)| !urls.is_empty())
        .collect();
    lists.sort_by(|a, z| cmp_utf16(a.0, z.0));
    lists
        .into_iter()
        .map(|(profile, urls)| {
            let mut urls = urls.clone();
            urls.sort_by(|a, z| cmp_utf16(a, z));
            Removal {
                profile: profile.clone(),
                urls,
                unassigned: fallback.as_ref() == Some(profile),
            }
        })
        .collect()
}

/// Port of `dueOf` (`packages/sync/src/companion/cycle.ts:61`).
fn due_of(c: &CompanionState) -> Vec<String> {
    let all: IndexSet<&String> = c.due.values().flat_map(IndexMap::keys).collect();
    let mut out: Vec<String> = all.into_iter().cloned().collect();
    out.sort_by(|a, z| cmp_utf16(a, z));
    out
}

/// Port of `profileScope` (`packages/sync/src/companion/cycle.ts:62`).
fn profile_scope(sync_id: &str) -> String {
    format!("p:{sync_id}")
}

/// Port of `transport` (`packages/sync/src/companion/cycle.ts:64`): the folder through the host.
struct HostTransport;

fn ready<'a, T: 'a>(value: T) -> LocalBoxFuture<'a, T> {
    Box::pin(std::future::ready(value))
}

impl Transport for HostTransport {
    fn write<'a>(
        &'a self,
        scope: &'a str,
        payload: &'a str,
    ) -> LocalBoxFuture<'a, SyncResult<String>> {
        ready(host().and_then(|h| h.write(scope, payload)))
    }

    fn read<'a>(
        &'a self,
        scope: &'a str,
        known: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<Listing>> {
        let listing = host()
            .and_then(|h| h.read(scope, &json_of(&known)?))
            .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()));
        ready(listing)
    }

    fn remove<'a>(
        &'a self,
        scope: &'a str,
        ids: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<()>> {
        ready(
            host()
                .and_then(|h| h.remove(scope, &json_of(&ids)?))
                .map(|_| ()),
        )
    }
}

type Persist = Rc<dyn for<'a> Fn(&'a [LogFile]) -> LocalBoxFuture<'a, SyncResult<()>>>;

fn persist_hook<F: for<'a> Fn(&'a [LogFile]) -> LocalBoxFuture<'a, SyncResult<()>> + 'static>(
    f: F,
) -> Persist {
    Rc::new(f)
}

/// Port of `persistJournal` (`packages/sync/src/companion/cycle.ts:72`).
fn persist_journal(scope: &str) -> Persist {
    let scope = scope.to_string();
    persist_hook(move |batches| {
        let persist = || {
            let host = host()?;
            let raw = batches
                .iter()
                .map(json_of)
                .collect::<SyncResult<Vec<String>>>()?;
            if host.persist_journal(&scope, &json_of(&raw)?)? != "ok" {
                return Err("couldn't save the sync journal".to_string());
            }
            Ok(())
        };
        ready(persist())
    })
}

/// Port of `scopeOptions` (`packages/sync/src/companion/cycle.ts:78`).
fn scope_options(scope: &str, profile: bool) -> ScopeOptions {
    ScopeOptions {
        persist: Some(persist_journal(scope)),
        compact_after_files: Some(f64::INFINITY),
        expired: if profile {
            Some(Rc::new(history_expired))
        } else {
            None
        },
        ..Default::default()
    }
}

/// An entry of a list that `dropFrom` filters.
trait Keyed {
    fn scope(&self) -> &str;
    fn key(&self) -> &str;
}

impl Keyed for PendingSend {
    fn scope(&self) -> &str {
        &self.scope
    }
    fn key(&self) -> &str {
        &self.key
    }
}

impl Keyed for Ack {
    fn scope(&self) -> &str {
        &self.scope
    }
    fn key(&self) -> &str {
        &self.key
    }
}

impl Keyed for BookmarkOp {
    fn scope(&self) -> &str {
        &self.scope
    }
    fn key(&self) -> &str {
        &self.key
    }
}

/// Port of `dropFrom` (`packages/sync/src/companion/cycle.ts:84`), in place.
fn drop_from<T: Keyed>(list: &mut Vec<T>, scope: &str, keys: &IndexSet<String>) {
    if !keys.is_empty() {
        list.retain(|x| x.scope() != scope || !keys.contains(x.key()));
    }
}

/// Port of `profileOf` (`packages/sync/src/companion/cycle.ts:88`).
fn profile_of(
    profiles: &IndexMap<String, Value>,
    fallback: Option<&String>,
    p: Option<&str>,
) -> Option<String> {
    match p {
        Some(p) if !p.is_empty() && profiles.contains_key(p) => Some(p.to_string()),
        _ => fallback.cloned(),
    }
}

/// The `run` closure of `runCompanionCycle` with the totals that it adds up.
struct Runner<'a> {
    cell: &'a Rc<RefCell<CompanionState>>,
    clock: &'a mut Clock,
    device: &'a str,
    now: &'a Rc<dyn Fn() -> f64>,
    published: usize,
    changed: usize,
    pending: u64,
}

impl Runner<'_> {
    /// `run` (`packages/sync/src/companion/cycle.ts:105`).
    async fn run(
        &mut self,
        scope: &str,
        adapters: Vec<Box<dyn Adapter>>,
        profile: bool,
    ) -> SyncResult<()> {
        let mut state = {
            let mut c = self.cell.borrow_mut();
            std::mem::replace(
                c.scopes
                    .entry(scope.to_string())
                    .or_insert_with(empty_scope),
                empty_scope(),
            )
        };
        let old_last = state.journal.as_ref().and_then(|j| j.last()).map(|f| f.seq);
        let result = sync_scope(SyncScopeArgs {
            scope,
            state: &mut state,
            transport: &HostTransport,
            adapters: &adapters,
            clock: &mut *self.clock,
            device: self.device,
            now: Some(self.now.clone()),
            options: scope_options(scope, profile),
        })
        .await;
        let rebuilt = old_last.is_some_and(|l| {
            state
                .journal
                .as_ref()
                .and_then(|j| j.first())
                .is_some_and(|f| f.seq > l)
        });
        let mut c = self.cell.borrow_mut();
        let CompanionState {
            scopes, late_keys, ..
        } = &mut *c;
        note_late_keys(
            late_keys.entry(scope.to_string()).or_default(),
            &state,
            rebuilt,
        );
        scopes.insert(scope.to_string(), state);
        let result = result?;
        if let Some(state) = c.scopes.get_mut(scope) {
            state
                .unapplied
                .retain(|k| adapters.iter().any(|a| k.starts_with(a.prefix())));
        }
        c.pulled.insert(scope.to_string(), true);
        self.published += result.published;
        self.changed += result.changed;
        self.pending += result.pending;
        Ok(())
    }
}

/// Port of `runCompanionCycle` (`packages/sync/src/companion/cycle.ts:93`).
pub async fn run_companion_cycle(input: &CycleInput) -> SyncResult<CycleOutput> {
    let cell = Rc::new(RefCell::new(take_loaded()?));
    let out = cycle(&cell, input).await;
    put_loaded(cell.take());
    out
}

async fn cycle(cell: &Rc<RefCell<CompanionState>>, input: &CycleInput) -> SyncResult<CycleOutput> {
    let fixed = input.now;
    let now: Rc<dyn Fn() -> f64> = Rc::new(move || fixed.unwrap_or_else(now_ms));
    let status = host()?.chain_status()?;
    if status != "ok" {
        let c = cell.borrow();
        let (history_removed, visits_due) = (removals_of(&c), due_of(&c));
        let saved = save_of(&c, false)?;
        return Ok(CycleOutput {
            status,
            received: Vec::new(),
            history_removed,
            visits_due,
            published: 0,
            changed: 0,
            pending: 0,
            saved,
            view: None,
        });
    }
    let (device, last) = {
        let c = cell.borrow();
        (c.device.clone(), c.clock.clone())
    };
    let mut clock = Clock::new(&device, last.as_deref(), Some(now.clone()));
    let retiring = input.retiring;
    let mut runner = Runner {
        cell,
        clock: &mut clock,
        device: &device,
        now: &now,
        published: 0,
        changed: 0,
        pending: 0,
    };

    let ghosts: Vec<String> = {
        let mut c = cell.borrow_mut();
        let at = now();
        c.ghosts.retain(|g| at - g.since < GHOST_MS);
        c.ghosts.iter().map(|g| g.id.clone()).collect()
    };
    runner
        .run(
            APP,
            vec![device_adapter(&device, &input.name, retiring, &ghosts)],
            false,
        )
        .await?;
    if retiring {
        let scopes: Vec<String> = cell
            .borrow()
            .scopes
            .keys()
            .filter(|s| s.starts_with("p:"))
            .cloned()
            .collect();
        for scope in scopes {
            runner
                .run(
                    &scope,
                    vec![device_tabs_adapter(
                        &device,
                        &input.name,
                        Some(Vec::new()),
                        true,
                        &ghosts,
                    )],
                    true,
                )
                .await?;
        }
    } else {
        profile_cycles(&mut runner, input, &ghosts).await?;
    }
    let (published, changed, pending) = (runner.published, runner.changed, runner.pending);
    {
        let mut c = cell.borrow_mut();
        c.clock = Some(clock.last());
        forget_old_sends(&mut c, now());
    }

    let mut received = Vec::new();
    if !retiring {
        let mut c = cell.borrow_mut();
        let c = &mut *c;
        let acked: HashSet<String> = c
            .acks
            .iter()
            .map(|a| format!("{}\n{}", a.scope, a.key))
            .collect();
        let app = c.scopes.get(APP);
        let names: HashMap<String, Option<String>> = app
            .map(|app| visible(app, "dev:"))
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| {
                (
                    k.get(4..).unwrap_or_default().to_string(),
                    v.get("n").and_then(Value::as_str).map(str::to_string),
                )
            })
            .collect();
        for id in live_profiles(app).keys() {
            let scope = profile_scope(id);
            let Some(state) = c.scopes.get(&scope) else {
                continue;
            };
            for (k, s) in visible(state, "send:") {
                if !truthy(&s)
                    || s.get("to").and_then(Value::as_str) != Some(device.as_str())
                    || s.get("from").and_then(Value::as_str) == Some(device.as_str())
                    || acked.contains(&format!("{scope}\n{k}"))
                {
                    continue;
                }
                let Some(u) = s.get("u").and_then(Value::as_str).filter(|u| is_web_url(u)) else {
                    c.acks.push(Ack {
                        scope: scope.clone(),
                        key: k,
                    });
                    continue;
                };
                let from = s
                    .get("from")
                    .filter(|f| !f.is_null())
                    .map_or_else(String::new, to_js_string);
                // `String(s.from)`: `undefined` prints as "undefined".
                let from_key = s
                    .get("from")
                    .map_or_else(|| "undefined".to_string(), to_js_string);
                let at = s.get("at").map_or(f64::NAN, to_number);
                received.push(Received {
                    profile: id.clone(),
                    key: k,
                    u: u.to_string(),
                    t: s.get("t")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    from,
                    from_name: names.get(&from_key).cloned().flatten(),
                    at: if at == 0.0 || at.is_nan() { 0.0 } else { at },
                });
            }
        }
        received.sort_by(|a, z| {
            let d = a.at - z.at;
            if d != 0.0 && !d.is_nan() {
                d.partial_cmp(&0.0).unwrap_or(Ordering::Equal)
            } else if cmp_utf16(&a.key, &z.key) == Ordering::Less {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });
    }

    let c = cell.borrow();
    let mut out = CycleOutput {
        status,
        received,
        history_removed: removals_of(&c),
        visits_due: due_of(&c),
        published,
        changed,
        pending,
        saved: save_of(&c, published > 0 || changed > 0)?,
        view: None,
    };
    if !retiring {
        let view = build_view(&c.scopes, &device);
        if view_is_new(&json_of(&view)?) {
            out.view = Some(view);
        }
    }
    Ok(out)
}

/// The profile part of `runCompanionCycle` (`packages/sync/src/companion/cycle.ts:121`).
async fn profile_cycles(
    runner: &mut Runner<'_>,
    input: &CycleInput,
    ghosts: &[String],
) -> SyncResult<()> {
    let cell = runner.cell;
    let now = runner.now.clone();
    let device = runner.device.to_string();
    let (profiles, fallback, owner) = {
        let c = cell.borrow();
        let app = c.scopes.get(APP);
        let profiles = live_profiles(app);
        let fallback = default_profile_id(&profiles);
        let owner: IndexMap<String, String> = profiles
            .keys()
            .map(|id| (id.clone(), data_owner(app, id)))
            .collect();
        (profiles, fallback, owner)
    };
    let mut tabs_of: IndexMap<String, Vec<LocalTab>> = IndexMap::new();
    for t in input.tabs.iter().flatten() {
        let Some(p) = profile_of(&profiles, fallback.as_ref(), t.p.as_deref()) else {
            continue;
        };
        tabs_of.entry(p).or_default().push(t.clone());
    }
    let mut visits_of: IndexMap<String, Vec<LocalVisit>> = IndexMap::new();
    let removing: Rc<RefCell<HashMap<String, HashSet<String>>>> = Rc::default();
    for v in input.visits.iter().flatten() {
        let Some(p) = profile_of(
            &profiles,
            fallback.as_ref(),
            v.get("p").and_then(Value::as_str),
        ) else {
            continue;
        };
        let (Some(u), Some(vt)) = (
            v.get("u").and_then(Value::as_str),
            v.get("vt").and_then(Value::as_array),
        ) else {
            continue;
        };
        let o = owner.get(&p).cloned().unwrap_or_else(|| p.clone());
        let times: Vec<f64> = vt.iter().filter_map(Value::as_f64).collect();
        let newest = times.iter().fold(f64::NEG_INFINITY, |m, &t| js_max(m, t));
        let t = v
            .get("t")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        visits_of.entry(o.clone()).or_default().push(LocalVisit {
            u: u.to_string(),
            t,
            vt: times,
            p: Some(p),
        });
        let mut c = cell.borrow_mut();
        let mine = c.visited.entry(profile_scope(&o)).or_default();
        if newest.is_finite() && newest > mine.get(u).copied().unwrap_or(f64::NEG_INFINITY) {
            mine.insert(u.to_string(), newest);
        }
    }
    let mut ids: Vec<&String> = profiles.keys().collect();
    ids.sort_by(|a, z| cmp_utf16(a, z));
    for id in ids {
        let scope = profile_scope(id);
        let owns_data = owner.get(id) == Some(id);
        let (floors, mine) = {
            let mut c = cell.borrow_mut();
            let floors = c.floors.entry(scope.clone()).or_default();
            floors.retain(|_, at| *at >= now() - HISTORY_WINDOW_MS);
            let floors = std::mem::take(floors);
            let mine = c.visited.entry(scope.clone()).or_default();
            mine.retain(|_, at| *at >= now() - HISTORY_WINDOW_MS);
            let mine = mine.clone();
            c.scopes.entry(scope.clone()).or_insert_with(empty_scope);
            (Rc::new(RefCell::new(floors)), mine)
        };
        let drop_sends: DropKeys = {
            let c = cell.clone();
            Rc::new(move |s: &str, keys: &IndexSet<String>| {
                let mut c = c.borrow_mut();
                drop_from(&mut c.sends, s, keys);
                drop_from(&mut c.acks, s, keys);
            })
        };
        let (sends, acks) = (cell.clone(), cell.clone());
        let mut adapters: Vec<Box<dyn Adapter>> = vec![
            device_tabs_adapter(
                &device,
                &input.name,
                input
                    .tabs
                    .as_ref()
                    .map(|_| tabs_of.get(id).cloned().unwrap_or_default()),
                false,
                ghosts,
            ),
            sends_adapter(SendContext {
                scope: scope.clone(),
                sends: Box::new(move || sends.borrow().sends.clone()),
                acks: Box::new(move || acks.borrow().acks.clone()),
                drop: drop_sends,
            }),
        ];
        if owns_data {
            let mut owned: Vec<String> = owner
                .iter()
                .filter(|(_, o)| *o == id)
                .map(|(p, _)| p.clone())
                .collect();
            owned.sort_by(|a, z| cmp_utf16(a, z));
            let (due, removed, pulled) = (cell.clone(), cell.clone(), cell.clone());
            let (due_scope, ready_scope, removing) =
                (scope.clone(), scope.clone(), removing.clone());
            let has_visits = input.visits.is_some();
            let extracts = Cell::new(0u32);
            adapters.push(history_adapter(HistoryContext {
                visits: visits_of.get(id).cloned().unwrap_or_default(),
                profiles: owned,
                floors: floors.clone(),
                visited: mine,
                due: Box::new(move |url: &str| {
                    due.borrow_mut()
                        .due
                        .entry(due_scope.clone())
                        .or_default()
                        .insert(url.to_string(), true);
                }),
                removed: Box::new(move |p: &str, url: &str| {
                    let mut c = removed.borrow_mut();
                    let list = c.removals.entry(p.to_string()).or_default();
                    let mut removing = removing.borrow_mut();
                    let listed = removing
                        .entry(p.to_string())
                        .or_insert_with(|| list.iter().cloned().collect());
                    if listed.insert(url.to_string()) {
                        list.push(url.to_string());
                    }
                }),
                ready: Box::new(move || {
                    has_visits
                        && (pulled
                            .borrow()
                            .pulled
                            .get(&ready_scope)
                            .copied()
                            .unwrap_or(false)
                            || {
                                extracts.set(extracts.get() + 1);
                                extracts.get() > 1
                            })
                }),
            }));
            let (ops, dropped) = (cell.clone(), cell.clone());
            adapters.push(bookmarks_adapter(BookmarkContext {
                scope: scope.clone(),
                ops: Box::new(move |f: &mut dyn FnMut(&mut Vec<BookmarkOp>)| {
                    f(&mut ops.borrow_mut().bookmarks)
                }),
                drop: Rc::new(move |s: &str, keys: &IndexSet<String>| {
                    drop_from(&mut dropped.borrow_mut().bookmarks, s, keys)
                }),
            }));
        }
        let result = runner.run(&scope, adapters, true).await;
        let mut c = cell.borrow_mut();
        c.floors.insert(scope.clone(), floors.take());
        result?;
        if c.floors.get(&scope).is_some_and(IndexMap::is_empty) {
            c.floors.shift_remove(&scope);
        }
        if c.visited.get(&scope).is_some_and(IndexMap::is_empty) {
            c.visited.shift_remove(&scope);
        }
    }
    let fetched: HashSet<&str> = input
        .fetched_due
        .iter()
        .flatten()
        .map(String::as_str)
        .collect();
    let mut c = cell.borrow_mut();
    let c = &mut *c;
    let visited = &c.visited;
    c.due.retain(|scope, urls| {
        urls.retain(|url, _| {
            !fetched.contains(url.as_str())
                && visited.get(scope).is_some_and(|m| m.contains_key(url))
        });
        !urls.is_empty()
    });
    Ok(())
}

/// Port of `companionView` (`packages/sync/src/companion/cycle.ts:251`).
pub fn companion_view() -> SyncResult<View> {
    with_loaded(|c| {
        let view = build_view(&c.scopes, &c.device);
        view_is_new(&json_of(&view)?);
        Ok(view)
    })
}

/// Port of `scopeFor` (`packages/sync/src/companion/cycle.ts:258`).
fn scope_for(c: &CompanionState, profile: &str) -> String {
    profile_scope(&data_owner(c.scopes.get(APP), profile))
}

/// Port of `isLive` (`packages/sync/src/companion/cycle.ts:259`).
fn is_live(c: &CompanionState, profile: &str) -> bool {
    live_profiles(c.scopes.get(APP)).contains_key(profile)
}

/// Port of `refused` (`packages/sync/src/companion/cycle.ts:260`).
const REFUSED: Saved = Saved {
    saved: false,
    refused: Some(true),
    existing: None,
};
const SEND_KEY_KEPT_MS: f64 = SEND_LIFETIME_MS + DAY_MS;

/// Port of `forgetOldSends` (`packages/sync/src/companion/cycle.ts:263`).
fn forget_old_sends(c: &mut CompanionState, now: f64) -> &mut IndexMap<String, f64> {
    c.sent.retain(|_, at| now - *at <= SEND_KEY_KEPT_MS);
    &mut c.sent
}

/// The input of `companionSend`.
#[derive(Clone, Debug, Deserialize)]
pub struct SendInput {
    pub profile: String,
    pub key: String,
    pub u: String,
    pub t: String,
    pub to: String,
    pub at: f64,
}

/// Port of `companionSend` (`packages/sync/src/companion/cycle.ts:269`).
pub fn companion_send(input: &SendInput) -> SyncResult<Saved> {
    with_loaded(|c| {
        if !is_live(c, &input.profile) {
            return Ok(REFUSED);
        }
        let key = format!("send:{}", input.key);
        let device = c.device.clone();
        let known = forget_old_sends(c, input.at).contains_key(&key)
            || c.sends.iter().any(|s| s.key == key);
        c.sent.entry(key.clone()).or_insert(input.at);
        if !known {
            let value = SendRecord {
                u: input.u.clone(),
                t: input.t.clone(),
                to: input.to.clone(),
                from: device,
                at: input.at,
            };
            c.sends.push(PendingSend {
                scope: profile_scope(&input.profile),
                key,
                value,
            });
        }
        Ok(Saved {
            saved: save_of(c, true)?,
            refused: None,
            existing: None,
        })
    })
}

/// One profile of `companionRemovalsDone`.
#[derive(Clone, Debug, Deserialize)]
pub struct RemovalsDoneItem {
    pub profile: String,
    pub urls: Vec<String>,
}

/// The input of `companionRemovalsDone`.
#[derive(Clone, Debug, Deserialize)]
pub struct RemovalsDoneInput {
    pub items: Vec<RemovalsDoneItem>,
}

/// Port of `companionRemovalsDone` (`packages/sync/src/companion/cycle.ts:282`).
pub fn companion_removals_done(input: &RemovalsDoneInput) -> SyncResult<Saved> {
    with_loaded(|c| {
        for item in &input.items {
            let done: HashSet<&String> = item.urls.iter().collect();
            if let Some(list) = c.removals.get_mut(&item.profile) {
                list.retain(|url| !done.contains(url));
                if list.is_empty() {
                    c.removals.shift_remove(&item.profile);
                }
            }
        }
        Ok(Saved {
            saved: save_of(c, true)?,
            refused: None,
            existing: None,
        })
    })
}

/// One received send of `companionAcknowledge`.
#[derive(Clone, Debug, Deserialize)]
pub struct AcknowledgeItem {
    pub profile: String,
    pub key: String,
}

/// The input of `companionAcknowledge`.
#[derive(Clone, Debug, Deserialize)]
pub struct AcknowledgeInput {
    pub items: Vec<AcknowledgeItem>,
}

/// Port of `companionAcknowledge` (`packages/sync/src/companion/cycle.ts:293`).
pub fn companion_acknowledge(input: &AcknowledgeInput) -> SyncResult<Saved> {
    with_loaded(|c| {
        for item in &input.items {
            let scope = profile_scope(&item.profile);
            if !c.acks.iter().any(|a| a.scope == scope && a.key == item.key) {
                c.acks.push(Ack {
                    scope,
                    key: item.key.clone(),
                });
            }
        }
        Ok(Saved {
            saved: save_of(c, true)?,
            refused: None,
            existing: None,
        })
    })
}

/// Port of `hasBookmarkFor` (`packages/sync/src/companion/cycle.ts:302`).
fn has_bookmark_for(c: &CompanionState, scope: &str, url: &str) -> SyncResult<bool> {
    let removing: HashSet<String> = c
        .bookmarks
        .iter()
        .filter(|o| o.scope == scope && o.op == BookmarkOpKind::Remove)
        .map(|o| format!("bm:{}", o.key))
        .collect();
    for o in c
        .bookmarks
        .iter()
        .filter(|o| o.scope == scope && o.op == BookmarkOpKind::Add)
    {
        let value = o
            .value
            .as_ref()
            .ok_or("Cannot read properties of undefined (reading 'u')")?;
        if value.u.as_deref() == Some(url) {
            return Ok(true);
        }
    }
    let Some(state) = c.scopes.get(scope) else {
        return Ok(false);
    };
    Ok(visible(state, "bm:").iter().any(|(key, bookmark)| {
        !removing.contains(key)
            && bookmark.get("k").and_then(Value::as_str) == Some("u")
            && bookmark
                .get("u")
                .and_then(Value::as_str)
                .is_some_and(|u| from_legacy_url(u) == url)
    }))
}

/// The input of `companionAddBookmark`.
#[derive(Clone, Debug, Deserialize)]
pub struct AddBookmarkInput {
    pub profile: String,
    pub key: String,
    pub u: String,
    pub t: String,
    pub at: f64,
}

/// Port of `companionAddBookmark` (`packages/sync/src/companion/cycle.ts:314`).
pub fn companion_add_bookmark(input: &AddBookmarkInput) -> SyncResult<Saved> {
    with_loaded(|c| {
        if !is_live(c, &input.profile) {
            return Ok(REFUSED);
        }
        let scope = scope_for(c, &input.profile);
        if has_bookmark_for(c, &scope, &input.u)? {
            return Ok(Saved {
                saved: true,
                refused: None,
                existing: Some(true),
            });
        }
        if !c
            .bookmarks
            .iter()
            .any(|o| o.scope == scope && o.key == input.key)
        {
            let value = BookmarkDraft {
                k: "u".into(),
                p: "other".into(),
                t: input.t.clone(),
                u: Some(input.u.clone()),
                a: input.at,
                pos: None,
            };
            c.bookmarks.push(BookmarkOp {
                scope,
                op: BookmarkOpKind::Add,
                key: input.key.clone(),
                value: Some(value),
            });
        }
        Ok(Saved {
            saved: save_of(c, true)?,
            refused: None,
            existing: Some(false),
        })
    })
}

/// The input of `companionRemoveBookmark`.
#[derive(Clone, Debug, Deserialize)]
pub struct RemoveBookmarkInput {
    pub profile: String,
    pub key: String,
}

/// Port of `companionRemoveBookmark` (`packages/sync/src/companion/cycle.ts:325`).
pub fn companion_remove_bookmark(input: &RemoveBookmarkInput) -> SyncResult<Saved> {
    with_loaded(|c| {
        if !is_live(c, &input.profile) {
            return Ok(REFUSED);
        }
        let scope = scope_for(c, &input.profile);
        c.bookmarks
            .retain(|o| !(o.scope == scope && o.key == input.key));
        c.bookmarks.push(BookmarkOp {
            scope,
            op: BookmarkOpKind::Remove,
            key: input.key.clone(),
            value: None,
        });
        Ok(Saved {
            saved: save_of(c, true)?,
            refused: None,
            existing: None,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_from_keeps_other_scopes_and_keys() {
        let ack = |scope: &str, key: &str| Ack {
            scope: scope.into(),
            key: key.into(),
        };
        let mut list = vec![ack("a", "1"), ack("b", "1"), ack("a", "2")];
        drop_from(&mut list, "a", &IndexSet::from(["1".to_string()]));
        assert_eq!(list, vec![ack("b", "1"), ack("a", "2")]);
    }
}
