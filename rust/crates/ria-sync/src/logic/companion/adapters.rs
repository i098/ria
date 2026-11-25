// Ported from ardia packages/sync/src/companion/adapters.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The adapters of the phone companion: its device record, its open tabs, its history, the tabs
//! that it sends and the bookmarks that it adds or removes.
//!
//! Changes from Ardia:
//! - An adapter reads its scope through the `state` of `ExtractCtx` and `ApplyRecords`, so
//!   `HistoryContext` and `BookmarkContext` have no `state` field.
//! - `HistoryContext.floors` is shared (`Rc<RefCell<..>>`), because the adapter writes the floors
//!   that the cycle keeps; `HistoryContext.visited` is a copy, because the adapter only reads it.
//! - `BookmarkContext.ops` lends the list of ops to a callback, because the adapter sets the
//!   `pos` of an op in place, as Ardia does through the shared object.
//! - `LocalTab` is typed: a tab without a string `u` and `t` and a number `at` is refused when
//!   it is read. `LocalVisit` is the visit that the cycle has checked (string `u`, number times).
//! - `dayOf` returns `Err("Invalid time value")` where `toISOString` throws a `RangeError`.
//! - A bookmark op `add` without a `value` returns an error, where Ardia throws a `TypeError`
//!   on `o.value.pos`.
//! - `to_number`, `to_js_string` and `truthy` are Ria additions: the JS coercions that the port needs.

use crate::logic::hlc::hlc_wall;
use crate::logic::js::{cmp_utf16, js_max, js_min, number, number_to_string, stringify};
use crate::logic::order::assign_positions;
use crate::logic::records::SendRecord;
use crate::logic::scope_keys::visible;
use crate::logic::scope_state::{
    Adapter, ApplyRecords, ExtractCtx, Extraction, LocalBoxFuture, SyncResult,
};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub const DAY_MS: f64 = 86_400_000.0;
pub const HISTORY_WINDOW_MS: f64 = 90.0 * DAY_MS;
pub const SEND_LIFETIME_MS: f64 = 7.0 * DAY_MS;
pub const MAX_DEVICE_TABS: usize = 30;
pub const MAX_VISIT_TIMES: usize = 50;
const SAME_VISIT_MS: f64 = 1.0;

/// An open tab of the phone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalTab {
    pub u: String,
    pub t: String,
    pub at: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<String>,
}

/// The visits of one address on the phone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalVisit {
    pub u: String,
    pub t: String,
    pub vt: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<String>,
}

/// A tab that the phone sends and that is not yet published.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingSend {
    pub scope: String,
    pub key: String,
    pub value: SendRecord,
}

/// A received send that the phone has opened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ack {
    pub scope: String,
    pub key: String,
}

/// `"add"` or `"remove"`: the `op` of a `BookmarkOp`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BookmarkOpKind {
    Add,
    Remove,
}

/// The value of a bookmark that the phone adds: `pos` is set when the adapter places it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BookmarkDraft {
    pub k: String,
    pub p: String,
    pub t: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u: Option<String>,
    pub a: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<String>,
}

/// A bookmark that the phone adds or removes; `value` is set for `add` only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BookmarkOp {
    pub scope: String,
    pub op: BookmarkOpKind,
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<BookmarkDraft>,
}

/// JS truthiness of a JSON value.
pub(crate) fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `String(v)` for a JSON value.
pub(crate) fn to_js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() {
                    String::new()
                } else {
                    to_js_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

/// `Number(text)`: StringToNumber of ECMA-262.
fn string_to_number(text: &str) -> f64 {
    let s = text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if s.is_empty() {
        return 0.0;
    }
    match s {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(digits) = s.strip_prefix(prefix) {
            if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            return digits.chars().fold(0.0, |n, c| {
                n * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
            });
        }
    }
    // Rust also reads "inf" and "nan", which JS does not.
    if !s
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E'))
    {
        return f64::NAN;
    }
    s.parse().unwrap_or(f64::NAN)
}

/// `Number(v)` (ToNumber) for a JSON value.
pub(crate) fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => string_to_number(s),
        Value::Array(_) => string_to_number(&to_js_string(v)),
        Value::Object(_) => f64::NAN,
    }
}

/// Port of `isWebUrl` (`packages/sync/src/companion/adapters.ts:27`).
pub fn is_web_url(url: &str) -> bool {
    let b = url.as_bytes();
    let starts = |p: &str| b.len() >= p.len() && b[..p.len()].eq_ignore_ascii_case(p.as_bytes());
    starts("http://") || starts("https://")
}

/// Port of `historyExpired` (`packages/sync/src/companion/adapters.ts:29`).
pub fn history_expired(key: &str, value: &Value, now: f64) -> bool {
    key.starts_with("h:") && last_visit_of(Some(value)) < now - HISTORY_WINDOW_MS
}

/// Port of `lastVisitOf` (`packages/sync/src/companion/adapters.ts:32`).
pub fn last_visit_of(v: Option<&Value>) -> f64 {
    match v
        .filter(|v| truthy(v))
        .and_then(|v| v.get("vt"))
        .and_then(Value::as_array)
    {
        Some(times) => times.iter().fold(0.0, |m, t| js_max(m, to_number(t))),
        None => 0.0,
    }
}

/// `[year, month, day]` of a count of days since 1970-01-01 (proleptic Gregorian).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Port of `dayOf` (`packages/sync/src/companion/adapters.ts:34`): `YYYYMMDD` of the UTC day.
pub fn day_of(at: f64) -> SyncResult<String> {
    if !at.is_finite() || at.abs() > 8.64e15 {
        return Err("Invalid time value".into());
    }
    let (y, m, d) = civil_from_days((at.trunc() / DAY_MS).floor() as i64);
    Ok(if (0..=9999).contains(&y) {
        format!("{y:04}{m:02}{d:02}")
    } else {
        // `±YYYYYY-MM-DD` cut to 10 characters, then every `-` removed, the sign of a year too.
        format!("{}{:06}{m:02}", if y < 0 { "" } else { "+" }, y.abs())
    })
}

fn ready<'a, T: 'a>(value: T) -> LocalBoxFuture<'a, T> {
    Box::pin(std::future::ready(value))
}

fn ghost_keys(prefix: &str, ghosts: &[String]) -> Rc<HashSet<String>> {
    Rc::new(ghosts.iter().map(|g| format!("{prefix}{g}")).collect())
}

/// `ignore` of the device and tabs adapters: keys of other devices that are not ghosts.
fn not_mine(key: &str, old: &Rc<HashSet<String>>) -> Box<dyn Fn(&str) -> bool> {
    let (key, old) = (key.to_string(), old.clone());
    Box::new(move |k: &str| k != key && !old.contains(k))
}

struct DeviceAdapter {
    key: String,
    name: String,
    retiring: bool,
    old: Rc<HashSet<String>>,
}

/// Port of `deviceAdapter` (`packages/sync/src/companion/adapters.ts:36`).
pub fn device_adapter(
    device: &str,
    name: &str,
    retiring: bool,
    ghosts: &[String],
) -> Box<dyn Adapter> {
    Box::new(DeviceAdapter {
        key: format!("dev:{device}"),
        name: name.to_string(),
        retiring,
        old: ghost_keys("dev:", ghosts),
    })
}

impl DeviceAdapter {
    fn extraction(&self, now: f64) -> SyncResult<Option<Extraction>> {
        let mut values = IndexMap::new();
        if !self.retiring {
            values.insert(
                self.key.clone(),
                json!({ "n": self.name, "k": "phone", "f": ["send"], "s": day_of(now)? }),
            );
        }
        Ok(Some(Extraction {
            values,
            ignore: Some(not_mine(&self.key, &self.old)),
            ..Default::default()
        }))
    }
}

impl Adapter for DeviceAdapter {
    fn prefix(&self) -> &str {
        "dev:"
    }

    fn extract<'a>(
        &'a self,
        ctx: &'a ExtractCtx<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Extraction>>> {
        ready(self.extraction(ctx.now))
    }

    fn apply<'a>(
        &'a self,
        _: &'a IndexMap<String, Value>,
        _: &'a IndexSet<String>,
        _: &'a ApplyRecords<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<bool>> {
        ready(Ok(true))
    }
}

struct DeviceTabsAdapter {
    key: String,
    name: String,
    tabs: Option<Vec<LocalTab>>,
    retiring: bool,
    old: Rc<HashSet<String>>,
}

/// Port of `deviceTabsAdapter` (`packages/sync/src/companion/adapters.ts:49`).
pub fn device_tabs_adapter(
    device: &str,
    name: &str,
    tabs: Option<Vec<LocalTab>>,
    retiring: bool,
    ghosts: &[String],
) -> Box<dyn Adapter> {
    Box::new(DeviceTabsAdapter {
        key: format!("tabs:{device}"),
        name: name.to_string(),
        tabs,
        retiring,
        old: ghost_keys("tabs:", ghosts),
    })
}

impl DeviceTabsAdapter {
    fn extraction(&self) -> Option<Extraction> {
        let ignore = Some(not_mine(&self.key, &self.old));
        if self.retiring {
            return Some(Extraction {
                ignore,
                ..Default::default()
            });
        }
        let Some(tabs) = &self.tabs else {
            if self.old.is_empty() {
                return None;
            }
            let old = self.old.clone();
            return Some(Extraction {
                ignore: Some(Box::new(move |k: &str| !old.contains(k))),
                ..Default::default()
            });
        };
        let mut recent: Vec<&LocalTab> = tabs.iter().filter(|t| is_web_url(&t.u)).collect();
        recent.sort_by(|a, z| z.at.partial_cmp(&a.at).unwrap_or(std::cmp::Ordering::Equal));
        recent.truncate(MAX_DEVICE_TABS);
        let recent: Vec<Value> = recent
            .iter()
            .map(|t| json!({ "u": t.u, "t": t.t }))
            .collect();
        let mut values = IndexMap::new();
        values.insert(
            self.key.clone(),
            json!({ "n": self.name, "k": "phone", "tabs": recent }),
        );
        Some(Extraction {
            values,
            ignore,
            ..Default::default()
        })
    }
}

impl Adapter for DeviceTabsAdapter {
    fn prefix(&self) -> &str {
        "tabs:"
    }

    fn extract<'a>(
        &'a self,
        _: &'a ExtractCtx<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Extraction>>> {
        ready(Ok(self.extraction()))
    }

    fn apply<'a>(
        &'a self,
        _: &'a IndexMap<String, Value>,
        _: &'a IndexSet<String>,
        _: &'a ApplyRecords<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<bool>> {
        ready(Ok(true))
    }
}

/// Port of `near` (`packages/sync/src/companion/adapters.ts:69`).
fn near(times: &[f64], t: f64) -> bool {
    times.iter().any(|k| (k - t).abs() <= SAME_VISIT_MS)
}

/// Port of `unionTimes` (`packages/sync/src/companion/adapters.ts:71`).
fn union_times(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut all: Vec<f64> = a.iter().chain(b).copied().collect();
    all.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<f64> = Vec::new();
    for t in all {
        if out.last().is_none_or(|last| t - last > SAME_VISIT_MS) {
            out.push(t);
        }
    }
    let cut = out.len().saturating_sub(MAX_VISIT_TIMES);
    out.split_off(cut)
}

/// Port of `keptTimes` (`packages/sync/src/companion/adapters.ts:77`) for times that are numbers.
fn kept_times(times: Vec<f64>) -> Vec<f64> {
    if times.len() > MAX_VISIT_TIMES {
        union_times(
            &times
                .into_iter()
                .filter(|t| t.is_finite())
                .collect::<Vec<_>>(),
            &[],
        )
    } else {
        times
    }
}

fn oldest_kept(times: &[f64]) -> f64 {
    if times.len() >= MAX_VISIT_TIMES {
        times.iter().fold(f64::INFINITY, |m, &t| js_min(m, t))
    } else {
        f64::NEG_INFINITY
    }
}

fn numbers(times: &[f64]) -> Value {
    Value::Array(times.iter().map(|&t| number(t)).collect())
}

/// `removed(profile, url)` of `HistoryContext`.
pub type Removed = Box<dyn Fn(&str, &str)>;

/// The context of `historyAdapter`: the visits of the phone and the callbacks of the cycle.
pub struct HistoryContext {
    /// The visits whose data this scope holds; `p` is the profile of each.
    pub visits: Vec<LocalVisit>,
    pub profiles: Vec<String>,
    pub floors: Rc<RefCell<IndexMap<String, f64>>>,
    pub visited: IndexMap<String, f64>,
    pub due: Box<dyn Fn(&str)>,
    pub removed: Removed,
    pub ready: Box<dyn Fn() -> bool>,
}

struct HistoryAdapter {
    ctx: HistoryContext,
}

/// Port of `historyAdapter` (`packages/sync/src/companion/adapters.ts:90`).
pub fn history_adapter(ctx: HistoryContext) -> Box<dyn Adapter> {
    Box::new(HistoryAdapter { ctx })
}

struct UrlEntry {
    title: String,
    times: Vec<f64>,
    profiles: IndexSet<String>,
    newest: f64,
}

impl HistoryAdapter {
    fn floor(&self, url: &str) -> f64 {
        self.ctx
            .floors
            .borrow()
            .get(url)
            .copied()
            .unwrap_or(f64::NEG_INFINITY)
    }

    fn raise_floor(&self, url: &str, deleted_at: f64) {
        if deleted_at > self.floor(url) {
            self.ctx
                .floors
                .borrow_mut()
                .insert(url.to_string(), deleted_at);
        }
    }

    fn extraction(&self, x: &ExtractCtx<'_>) -> Option<Extraction> {
        if !(self.ctx.ready)() {
            return None;
        }
        let now = x.now;
        let since = now - HISTORY_WINDOW_MS;
        let mut by_url: IndexMap<&str, UrlEntry> = IndexMap::new();
        for v in &self.ctx.visits {
            if !is_web_url(&v.u) {
                continue;
            }
            let times: Vec<f64> =
                v.vt.iter()
                    .copied()
                    .filter(|t| t.is_finite() && *t >= since)
                    .collect();
            if times.is_empty() {
                continue;
            }
            let entry = by_url.entry(&v.u).or_insert_with(|| UrlEntry {
                title: String::new(),
                times: Vec::new(),
                profiles: IndexSet::new(),
                newest: 0.0,
            });
            entry.times.extend(&times);
            entry.profiles.insert(v.p.clone().unwrap_or_default());
            let newest = times.iter().fold(f64::NEG_INFINITY, |m, &t| js_max(m, t));
            if newest >= entry.newest {
                entry.newest = newest;
                if !v.t.is_empty() {
                    entry.title = v.t.clone();
                }
            }
        }
        let mut values = IndexMap::new();
        for (url, entry) in by_url {
            let key = format!("h:{url}");
            let known = x.remote(&key);
            if known.is_some_and(Value::is_null) {
                let wall = x
                    .stamp(&key)
                    .filter(|h| !h.is_empty())
                    .map_or(0.0, hlc_wall);
                self.raise_floor(url, js_min(wall, now));
            }
            let floor = self.floor(url);
            let later: Vec<f64> = entry.times.iter().copied().filter(|&t| t > floor).collect();
            let local = union_times(&[], &later);
            if local.is_empty() {
                for p in &entry.profiles {
                    (self.ctx.removed)(p, url);
                }
                continue;
            }
            let Some(known) = known.filter(|k| truthy(k)) else {
                values.insert(
                    key,
                    json!({ "t": entry.title, "n": local.len(), "vt": numbers(&local) }),
                );
                continue;
            };
            let known_times = kept_times(
                known
                    .get("vt")
                    .and_then(Value::as_array)
                    .map_or_else(Vec::new, |vt| vt.iter().filter_map(Value::as_f64).collect()),
            );
            let oldest = oldest_kept(&known_times);
            if !local.iter().any(|&t| t >= oldest && !near(&known_times, t)) {
                continue;
            }
            let vt = union_times(&known_times, &local);
            let title = if entry.title.is_empty() {
                known
                    .get("t")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            } else {
                entry.title
            };
            let known_n = known.get("n").and_then(Value::as_f64).unwrap_or(0.0);
            let n = js_max(js_max(known_n, local.len() as f64), vt.len() as f64);
            values.insert(
                key,
                json!({ "t": title, "n": number(n), "vt": numbers(&vt) }),
            );
        }
        let keys: HashSet<String> = values.keys().cloned().collect();
        Some(Extraction {
            values,
            ignore: Some(Box::new(move |k: &str| !keys.contains(k))),
            ..Default::default()
        })
    }

    fn apply_now(
        &self,
        visible: &IndexMap<String, Value>,
        changed: &IndexSet<String>,
        records: &ApplyRecords<'_>,
    ) -> bool {
        for key in changed {
            let url = key.get(2..).unwrap_or_default();
            if let Some(value) = visible.get(key).filter(|v| truthy(v)) {
                let Some(&mine) = self.ctx.visited.get(url) else {
                    continue;
                };
                let times = match value.get("vt").and_then(Value::as_array) {
                    Some(vt) if vt.len() > MAX_VISIT_TIMES => {
                        kept_times(vt.iter().filter_map(Value::as_f64).collect())
                    }
                    Some(vt) => vt.iter().map(to_number).collect(),
                    None => Vec::new(),
                };
                if mine >= oldest_kept(&times) && mine > self.floor(url) && !near(&times, mine) {
                    (self.ctx.due)(url);
                }
                continue;
            }
            let tombstone = records.state.records.get(key);
            let before: Option<Value> = records
                .state
                .base
                .get(key)
                .and_then(|json| serde_json::from_str(json).ok());
            if tombstone.is_none()
                && before.as_ref().is_some_and(truthy)
                && last_visit_of(before.as_ref()) < records.now - HISTORY_WINDOW_MS
            {
                continue;
            }
            self.raise_floor(
                url,
                js_min(
                    tombstone.map_or(records.now, |t| hlc_wall(&t.h)),
                    records.now,
                ),
            );
            for p in &self.ctx.profiles {
                (self.ctx.removed)(p, url);
            }
        }
        true
    }
}

impl Adapter for HistoryAdapter {
    fn prefix(&self) -> &str {
        "h:"
    }

    fn extract<'a>(
        &'a self,
        ctx: &'a ExtractCtx<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Extraction>>> {
        ready(Ok(self.extraction(ctx)))
    }

    fn apply<'a>(
        &'a self,
        visible: &'a IndexMap<String, Value>,
        changed: &'a IndexSet<String>,
        records: &'a ApplyRecords<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<bool>> {
        ready(Ok(self.apply_now(visible, changed, records)))
    }

    fn edited_at(&self, value: &Value) -> Option<f64> {
        Some(last_visit_of(Some(value)))
    }
}

/// The callback that drops the entries of a scope with the given keys from a list of the cycle.
pub type DropKeys = Rc<dyn Fn(&str, &IndexSet<String>)>;

/// The context of `sendsAdapter`.
pub struct SendContext {
    pub scope: String,
    pub sends: Box<dyn Fn() -> Vec<PendingSend>>,
    pub acks: Box<dyn Fn() -> Vec<Ack>>,
    pub drop: DropKeys,
}

struct SendsAdapter {
    ctx: SendContext,
}

/// Port of `sendsAdapter` (`packages/sync/src/companion/adapters.ts:174`).
pub fn sends_adapter(ctx: SendContext) -> Box<dyn Adapter> {
    Box::new(SendsAdapter { ctx })
}

fn send_value(s: &SendRecord) -> Value {
    json!({ "u": s.u, "t": s.t, "to": s.to, "from": s.from, "at": number(s.at) })
}

impl SendsAdapter {
    fn extraction(&self, x: &ExtractCtx<'_>) -> Extraction {
        let scope = &self.ctx.scope;
        let mut values = IndexMap::new();
        let mut tombstones = IndexSet::new();
        let mut gone = IndexSet::new();
        for s in (self.ctx.sends)() {
            if &s.scope != scope {
                continue;
            }
            match x.remote(&s.key) {
                Some(Value::Null) => {
                    gone.insert(s.key);
                }
                record if x.now - s.value.at >= SEND_LIFETIME_MS => {
                    if record.is_none() {
                        gone.insert(s.key);
                    } else {
                        tombstones.insert(s.key);
                    }
                }
                _ => {
                    values.insert(s.key, send_value(&s.value));
                }
            }
        }
        for a in (self.ctx.acks)() {
            if &a.scope != scope {
                continue;
            }
            match x.remote(&a.key) {
                None | Some(Value::Null) => gone.insert(a.key),
                Some(_) => tombstones.insert(a.key),
            };
        }
        if !gone.is_empty() {
            (self.ctx.drop)(scope, &gone);
        }
        let ignored = tombstones.clone();
        let (drop, scope) = (self.ctx.drop.clone(), scope.clone());
        Extraction {
            values,
            ignore: Some(Box::new(move |k: &str| !ignored.contains(k))),
            published: Some(Box::new(move |keys: &IndexSet<String>| {
                drop(
                    &scope,
                    &keys
                        .iter()
                        .filter(|k| tombstones.contains(*k))
                        .cloned()
                        .collect(),
                )
            })),
            ..Default::default()
        }
    }
}

impl Adapter for SendsAdapter {
    fn prefix(&self) -> &str {
        "send:"
    }

    fn extract<'a>(
        &'a self,
        ctx: &'a ExtractCtx<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Extraction>>> {
        ready(Ok(Some(self.extraction(ctx))))
    }

    fn apply<'a>(
        &'a self,
        _: &'a IndexMap<String, Value>,
        _: &'a IndexSet<String>,
        _: &'a ApplyRecords<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<bool>> {
        ready(Ok(true))
    }
}

/// `ops` of `BookmarkContext`: lends the list of ops to its argument.
pub type LendOps = Box<dyn Fn(&mut dyn FnMut(&mut Vec<BookmarkOp>))>;

/// The context of `bookmarksAdapter`. `ops` calls its argument with the list of ops of the cycle.
pub struct BookmarkContext {
    pub scope: String,
    pub ops: LendOps,
    pub drop: DropKeys,
}

struct BookmarksAdapter {
    ctx: BookmarkContext,
}

/// Port of `bookmarksAdapter` (`packages/sync/src/companion/adapters.ts:216`).
pub fn bookmarks_adapter(ctx: BookmarkContext) -> Box<dyn Adapter> {
    Box::new(BookmarksAdapter { ctx })
}

/// The JSON of a bookmark draft, with the keys in the order of Ardia.
fn draft_value(d: &BookmarkDraft) -> Value {
    let mut m = Map::new();
    m.insert("k".into(), json!(d.k));
    m.insert("p".into(), json!(d.p));
    m.insert("t".into(), json!(d.t));
    if let Some(u) = &d.u {
        m.insert("u".into(), json!(u));
    }
    m.insert("a".into(), number(d.a));
    if let Some(pos) = &d.pos {
        m.insert("pos".into(), json!(pos));
    }
    Value::Object(m)
}

impl BookmarksAdapter {
    fn extraction(&self, x: &ExtractCtx<'_>) -> SyncResult<Extraction> {
        let scope = &self.ctx.scope;
        let mut values: IndexMap<String, Value> = IndexMap::new();
        let mut deletes: IndexSet<String> = IndexSet::new();
        let mut settled: IndexSet<String> = IndexSet::new();
        let mut any = false;
        let mut failed: Option<String> = None;
        (self.ctx.ops)(&mut |all: &mut Vec<BookmarkOp>| {
            let ops: Vec<&mut BookmarkOp> = all.iter_mut().filter(|o| &o.scope == scope).collect();
            if ops.is_empty() {
                return;
            }
            any = true;
            let view = visible(x.state, "bm:");
            let mut children: IndexMap<&str, Vec<&str>> = IndexMap::new();
            for (k, v) in &view {
                if let Some(parent) = v.get("p").and_then(Value::as_str) {
                    children
                        .entry(parent)
                        .or_default()
                        .push(k.get(3..).unwrap_or_default());
                }
            }
            let mut others: Vec<(&str, &str)> = view
                .iter()
                .filter(|(_, v)| v.get("p").and_then(Value::as_str) == Some("other"))
                .filter_map(|(k, v)| Some((v.get("pos")?.as_str()?, k.as_str())))
                .collect();
            others.sort_by(|a, z| cmp_utf16(a.0, z.0).then_with(|| cmp_utf16(a.1, z.1)));
            let mut other_positions: Vec<Option<String>> = others
                .into_iter()
                .map(|(pos, _)| Some(pos.to_string()))
                .collect();
            for o in ops {
                let key = format!("bm:{}", o.key);
                if o.op == BookmarkOpKind::Add {
                    let Some(value) = o.value.as_mut() else {
                        failed = Some("Cannot read properties of undefined (reading 'pos')".into());
                        return;
                    };
                    if value.pos.is_none() {
                        let mut list = other_positions.clone();
                        list.push(None);
                        let pos = match assign_positions(&list) {
                            Ok(mut all) => all.pop().unwrap_or_default(),
                            Err(error) => {
                                failed = Some(error);
                                return;
                            }
                        };
                        value.pos = Some(pos.clone());
                        other_positions.push(Some(pos));
                    }
                    let value = draft_value(value);
                    if let Some(record) = x.remote(&key)
                        && (record.is_null() || stringify(record) == stringify(&value))
                    {
                        settled.insert(o.key.clone());
                        continue;
                    }
                    values.insert(key, value);
                } else {
                    if x.remote(&key).is_none_or(Value::is_null) || x.base_json(&key).is_none() {
                        settled.insert(o.key.clone());
                        continue;
                    }
                    let mut stack = vec![o.key.clone()];
                    while let Some(at) = stack.pop() {
                        if !deletes.insert(format!("bm:{at}")) {
                            continue;
                        }
                        for child in children.get(at.as_str()).into_iter().flatten() {
                            stack.push(child.to_string());
                        }
                    }
                }
            }
        });
        if let Some(error) = failed {
            return Err(error);
        }
        if !any {
            return Ok(Extraction {
                ignore: Some(Box::new(|_: &str| true)),
                ..Default::default()
            });
        }
        for k in &deletes {
            values.shift_remove(k);
        }
        if !settled.is_empty() {
            (self.ctx.drop)(scope, &settled);
        }
        let kept: HashSet<String> = values.keys().cloned().collect();
        let (drop, scope) = (self.ctx.drop.clone(), scope.clone());
        Ok(Extraction {
            values,
            ignore: Some(Box::new(move |k: &str| {
                !kept.contains(k) && !deletes.contains(k)
            })),
            published: Some(Box::new(move |keys: &IndexSet<String>| {
                drop(
                    &scope,
                    &keys
                        .iter()
                        .map(|k| k.get(3..).unwrap_or_default().to_string())
                        .collect(),
                )
            })),
            ..Default::default()
        })
    }
}

impl Adapter for BookmarksAdapter {
    fn prefix(&self) -> &str {
        "bm:"
    }

    fn extract<'a>(
        &'a self,
        ctx: &'a ExtractCtx<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Extraction>>> {
        ready(self.extraction(ctx).map(Some))
    }

    fn apply<'a>(
        &'a self,
        _: &'a IndexMap<String, Value>,
        _: &'a IndexSet<String>,
        _: &'a ApplyRecords<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<bool>> {
        ready(Ok(true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_of_matches_to_iso_string() {
        assert_eq!(day_of(0.0).as_deref(), Ok("19700101"));
        assert_eq!(day_of(-1.0).as_deref(), Ok("19691231"));
        assert_eq!(day_of(1_790_812_800_000.0).as_deref(), Ok("20261001"));
        assert_eq!(day_of(951_782_400_000.0).as_deref(), Ok("20000229"));
        assert_eq!(day_of(8.64e15).as_deref(), Ok("+27576009"));
        assert_eq!(day_of(-8.64e15).as_deref(), Ok("27182104"));
        assert_eq!(day_of(-62_198_755_200_000.0).as_deref(), Ok("00000101"));
        assert_eq!(day_of(8.64e15 + 1.0), Err("Invalid time value".to_string()));
    }

    #[test]
    fn union_times_merges_near_times_and_keeps_the_newest() {
        assert_eq!(
            union_times(&[5.0, 1.0], &[2.0, 9.0, 9.5]),
            vec![1.0, 5.0, 9.0]
        );
        let many: Vec<f64> = (0..60).map(|i| f64::from(i) * 10.0).collect();
        assert_eq!(union_times(&many, &[]).first(), Some(&100.0));
    }

    #[test]
    fn coercions_follow_js() {
        assert_eq!(to_number(&json!(" 12 ")), 12.0);
        assert_eq!(to_number(&json!("")), 0.0);
        assert!(to_number(&json!("inf")).is_nan());
        assert_eq!(to_number(&json!("0x1f")), 31.0);
        assert_eq!(to_number(&json!([7])), 7.0);
        assert_eq!(to_number(&json!(null)), 0.0);
        assert_eq!(to_js_string(&json!([1, null, "a"])), "1,,a");
        assert!(is_web_url("HTTPS://x") && !is_web_url("ftp://x"));
    }
}
