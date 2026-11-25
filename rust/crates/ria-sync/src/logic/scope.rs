// Ported from ardia packages/sync/src/scope.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! One sync cycle of one scope: publish local changes, write them as log files, read the files of
//! other devices, merge them by timestamp, apply them, and compact the folder into a snapshot.
//!
//! Changes from Ardia:
//! - `parseSyncFile` builds typed files, so a parsed file keeps only the fields of its type; a
//!   record in a snapshot with more fields loses them. Ria writes no such fields.
//! - `JSON.parse` takes a lone surrogate escape and turns an overflowing number into `Infinity`;
//!   `serde_json` refuses both, so `parseSyncFile` gives `null` for such a file. Ria writes neither.
//! - `JSON.parse` accepts deeper values; `serde_json` refuses nesting past 127 levels, so
//!   `parseSyncFile` gives `null` for such a file. Ria writes no such nesting.
//! - `ScopeOptions` fields are `None` for "use the default"; JS can also pass `undefined`
//!   explicitly, which the spread of Ardia turns into an `undefined` option.
//! - Keys keep insertion order; a JS object puts integer-like keys first. Keys named like
//!   `Object.prototype` members (`__proto__`, `constructor`) are plain keys here. Adapter prefixes,
//!   device ids and file ids of Ria are neither.
//! - The publish stamps are kept by adapter position, not by adapter object: the same adapter
//!   object twice in one list is not possible with boxed adapters.
//! - The settle queue of a state is `state.runtime.settling`.
//! - A failed `apply` writes its error to stderr (`eprintln!`), as `console.warn` does.
//! - `writtenSeq` stops at 0 when the journal is longer than `seq`, where JS goes below 0.

use super::applied::{covered, is_applied, mark_applied, raise_applied};
use super::canon::canonical;
use super::file_health::{PulledFiles, holds_back, note_file_health};
use super::hlc::{Clock, Hlc, hlc_wall};
use super::js::{cmp_utf16, js_max, now_ms, stringify, utf16_len};
use super::scope_keys::{
    base_keys_with, drop_base, drop_record, put_base, put_record, visible, with_base_index,
};
use super::scope_state::{
    Adapter, ApplyAs, ApplyRecords, ExtractCtx, FileKind, FileMeta, LogFile, LogKind, Op,
    ScopeOptions, ScopeState, SnapshotFile, SnapshotKind, SyncFile, SyncRecord, SyncResult,
    Transport,
};
use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// The options with the defaults of `DEFAULTS` (`packages/sync/src/scope.ts:9`) filled in.
struct Opts {
    expired: Option<Rc<dyn Fn(&str, &Value, f64) -> bool>>,
    tombstone_ttl: f64,
    compact_after_files: f64,
    compact_after_logs: f64,
    compact_log_share: f64,
    compact_interval: f64,
    prune_grace: f64,
}

const TOMBSTONE_TTL: f64 = 45.0 * 86_400_000.0;
const DAMAGED_GIVE_UP: f64 = 86_400_000.0;

fn opts(o: &ScopeOptions) -> Opts {
    Opts {
        expired: o.expired.clone(),
        tombstone_ttl: o.tombstone_ttl.unwrap_or(TOMBSTONE_TTL),
        compact_after_files: o.compact_after_files.unwrap_or(40.0),
        compact_after_logs: o.compact_after_logs.unwrap_or(400.0),
        compact_log_share: o.compact_log_share.unwrap_or(0.25),
        compact_interval: o.compact_interval.unwrap_or(10.0 * 60_000.0),
        prune_grace: o.prune_grace.unwrap_or(60.0 * 60_000.0),
    }
}

/// `isCount`: a safe integer of at least 0.
fn count(v: &Value) -> Option<u64> {
    let x = v.as_f64()?;
    (x.fract() == 0.0 && (0.0..=9_007_199_254_740_991.0).contains(&x)).then_some(x as u64)
}

/// `isTime`: a finite number.
fn time(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite())
}

/// `isMapOf`: an object whose values all pass `item`.
fn map_of<T>(v: &Value, item: impl Fn(&Value) -> Option<T>) -> Option<IndexMap<String, T>> {
    v.as_object()?
        .iter()
        .map(|(k, v)| Some((k.clone(), item(v)?)))
        .collect()
}

/// `isOrigin`.
fn origin(v: &Value) -> Option<(String, u64)> {
    match v.as_array()?.as_slice() {
        [device, seq] => Some((device.as_str()?.to_string(), count(seq)?)),
        _ => None,
    }
}

/// `isOp`.
fn op(v: &Value) -> Option<Op> {
    let o = v.as_object()?;
    Some(Op {
        k: o.get("k")?.as_str()?.to_string(),
        h: o.get("h")?.as_str()?.to_string(),
        v: o.get("v")?.clone(),
    })
}

/// `isRecord`.
fn record(v: &Value) -> Option<SyncRecord> {
    let o = v.as_object()?;
    Some(SyncRecord {
        h: o.get("h")?.as_str()?.to_string(),
        o: origin(o.get("o")?)?,
        v: o.get("v")?.clone(),
    })
}

/// Port of `parseSyncFile` (`packages/sync/src/scope.ts:29`).
pub fn parse_sync_file(payload: &str) -> Option<SyncFile> {
    let file: Value = serde_json::from_str(payload).ok()?;
    let file: &Map<String, Value> = file.as_object()?;
    if file.get("v").and_then(Value::as_f64) != Some(1.0) {
        return None;
    }
    let device = file.get("device")?.as_str()?.to_string();
    let at = time(file.get("at")?)?;
    match file.get("kind").and_then(Value::as_str) {
        Some("log") => {
            let seq = count(file.get("seq")?)?;
            let ops = file
                .get("ops")?
                .as_array()?
                .iter()
                .map(op)
                .collect::<Option<Vec<_>>>()?;
            Some(SyncFile::Log(LogFile {
                v: 1,
                kind: LogKind::Log,
                device,
                seq,
                at,
                ops,
            }))
        }
        Some("snapshot") => Some(SyncFile::Snapshot(SnapshotFile {
            v: 1,
            kind: SnapshotKind::Snapshot,
            device,
            at,
            vv: map_of(file.get("vv")?, count)?,
            seen: map_of(file.get("seen")?, time)?,
            records: map_of(file.get("records")?, record)?,
        })),
        _ => None,
    }
}

/// `JSON.stringify` of a file.
fn to_json(file: &impl Serialize) -> String {
    // INFALLIBLE: the files hold only string keys and JSON values.
    serde_json::to_value(file)
        .map(|v| stringify(&v))
        .unwrap_or_default()
}

/// Port of `see` (`packages/sync/src/scope.ts:44`).
fn see(state: &mut ScopeState, device: &str, at: f64) {
    if state.seen.get(device).is_none_or(|&seen| at > seen) {
        state.seen.insert(device.to_string(), at);
    }
}

/// Port of `mergeRecord` (`packages/sync/src/scope.ts:48`).
fn merge_record(state: &mut ScopeState, key: &str, record: SyncRecord, clock: &mut Clock) -> bool {
    clock.observe(&record.h);
    if state
        .records
        .get(key)
        .is_some_and(|current| cmp_utf16(&current.h, &record.h).is_ge())
    {
        return false;
    }
    put_record(state, key, record);
    true
}

/// Port of `ingestLog` (`packages/sync/src/scope.ts:56`).
fn ingest_log(
    state: &mut ScopeState,
    file: &LogFile,
    clock: &mut Clock,
    changed: &mut IndexSet<String>,
) {
    see(state, &file.device, file.at);
    if is_applied(state, (&file.device, file.seq)) {
        return;
    }
    for op in &file.ops {
        if merge_record(
            state,
            &op.k,
            SyncRecord {
                h: op.h.clone(),
                v: op.v.clone(),
                o: (file.device.clone(), file.seq),
            },
            clock,
        ) {
            changed.insert(op.k.clone());
        }
    }
    mark_applied(state, &file.device, file.seq);
}

/// Port of `ingestSnapshot` (`packages/sync/src/scope.ts:65`).
pub fn ingest_snapshot(
    state: &mut ScopeState,
    file: &SnapshotFile,
    clock: &mut Clock,
    changed: &mut IndexSet<String>,
) {
    for (device, &at) in &file.seen {
        see(state, device, at);
    }
    see(state, &file.device, file.at);
    for (key, record) in &file.records {
        if !state.records.contains_key(key) && is_applied(state, (&record.o.0, record.o.1)) {
            continue;
        }
        if merge_record(state, key, record.clone(), clock) {
            changed.insert(key.clone());
        }
    }
    let gone: Vec<String> = state
        .records
        .iter()
        .filter(|(key, record)| {
            !file.records.contains_key(*key) && covered(&file.vv, (&record.o.0, record.o.1))
        })
        .map(|(key, _)| key.clone())
        .collect();
    for key in gone {
        drop_record(state, &key);
        changed.insert(key);
    }
    for (device, &seq) in &file.vv {
        raise_applied(state, device, seq);
    }
}

/// Port of `settleDurable` (`packages/sync/src/scope.ts:91`).
fn settle_durable(state: &mut ScopeState) {
    for settle in std::mem::take(&mut state.runtime.settling) {
        settle();
    }
}

/// `Change` (`packages/sync/src/scope.ts:98`): `value` is `null` for a delete.
struct Change {
    key: String,
    value: Value,
    json: String,
}

/// `Stamps` (`packages/sync/src/scope.ts:99`): key to `(json, h)`.
type Stamps = IndexMap<String, (String, Hlc)>;

type Settle = Box<dyn FnOnce(&IndexSet<String>)>;

/// Port of `changesOf` (`packages/sync/src/scope.ts:101`).
async fn changes_of(
    state: &mut ScopeState,
    adapter: &dyn Adapter,
    now: f64,
) -> SyncResult<Option<(Vec<Change>, Option<Settle>)>> {
    if adapter.adopt_remote_on_join() && !state.joined {
        return Ok(None);
    }
    let extraction = {
        let ctx = ExtractCtx::new(state, now);
        adapter.extract(&ctx).await?
    };
    let Some(extraction) = extraction else {
        return Ok(None);
    };
    let mut changes = Vec::new();
    for (key, value) in &extraction.values {
        if value.is_null() {
            return Err(format!("[sync] {key}: a live value can't be null"));
        }
        let json = extraction
            .json
            .as_ref()
            .and_then(|known| known(key, value))
            .unwrap_or_else(|| canonical(value));
        if state.base.get(key) == Some(&json)
            && !extraction
                .republish
                .as_ref()
                .is_some_and(|republish| republish(key))
        {
            continue;
        }
        changes.push(Change {
            key: key.clone(),
            value: value.clone(),
            json,
        });
    }
    for key in base_keys_with(state, adapter.prefix()) {
        if !state.base.contains_key(&key)
            || extraction.values.contains_key(&key)
            || extraction
                .ignore
                .as_ref()
                .is_some_and(|ignore| ignore(&key))
        {
            continue;
        }
        changes.push(Change {
            key,
            value: Value::Null,
            json: "null".into(),
        });
    }
    Ok(Some((changes, extraction.published)))
}

/// Port of `stampOf` (`packages/sync/src/scope.ts:124`).
fn stamp_of(
    state: &ScopeState,
    adapter: &dyn Adapter,
    clock: &mut Clock,
    key: &str,
    value: &Value,
) -> Hlc {
    let edited_at = if !state.records.contains_key(key) && !value.is_null() {
        adapter.edited_at(value)
    } else {
        None
    };
    match edited_at {
        Some(time) => clock.at(time),
        None => clock.tick(),
    }
}

/// Port of `candidateStamps` (`packages/sync/src/scope.ts:129`).
async fn candidate_stamps(
    state: &mut ScopeState,
    adapter: &dyn Adapter,
    clock: &mut Clock,
    now: f64,
) -> SyncResult<Stamps> {
    let changes = changes_of(state, adapter, now)
        .await?
        .map(|(changes, _)| changes)
        .unwrap_or_default();
    Ok(changes
        .into_iter()
        .map(|c| {
            (
                c.key.clone(),
                (c.json, stamp_of(state, adapter, clock, &c.key, &c.value)),
            )
        })
        .collect())
}

/// Port of `publishAdapter` (`packages/sync/src/scope.ts:134`).
async fn publish_adapter(
    state: &mut ScopeState,
    adapter: &dyn Adapter,
    clock: &mut Clock,
    device: &str,
    now: f64,
    stamps: Option<&Stamps>,
) -> SyncResult<IndexSet<String>> {
    let mut published = IndexSet::new();
    let Some((changes, settle)) = changes_of(state, adapter, now).await? else {
        return Ok(published);
    };
    for Change { key, value, json } in changes {
        if value.is_null() {
            drop_base(state, &key);
        } else {
            put_base(state, &key, json.clone());
        }
        let h = match stamps.and_then(|stamps| stamps.get(&key)) {
            Some((candidate, h)) if *candidate == json => h.clone(),
            _ => stamp_of(state, adapter, clock, &key, &value),
        };
        if state
            .records
            .get(&key)
            .is_some_and(|known| cmp_utf16(&known.h, &h).is_ge())
        {
            continue;
        }
        put_record(
            state,
            &key,
            SyncRecord {
                h: h.clone(),
                v: value.clone(),
                o: (device.to_string(), state.seq + 1),
            },
        );
        state.outbox.push(Op {
            k: key.clone(),
            h,
            v: value,
        });
        published.insert(key);
    }
    if let Some(settle) = settle
        && !published.is_empty()
    {
        let keys = published.clone();
        state.runtime.settling.push(Box::new(move || settle(&keys)));
    }
    Ok(published)
}

/// Port of `flush` (`packages/sync/src/scope.ts:155`).
pub async fn flush(
    state: &mut ScopeState,
    transport: &dyn Transport,
    scope: &str,
    device: &str,
    now: f64,
    options: &ScopeOptions,
) -> SyncResult<()> {
    if !state.outbox.is_empty() {
        state.seq += 1;
        let ops = std::mem::take(&mut state.outbox);
        state.journal.get_or_insert_default().push(LogFile {
            v: 1,
            kind: LogKind::Log,
            device: device.to_string(),
            seq: state.seq,
            at: now,
            ops,
        });
    }
    if state.journal.as_ref().is_none_or(Vec::is_empty) {
        return Ok(());
    }
    if let Some(persist) = &options.persist {
        persist(state.journal.as_deref().unwrap_or_default()).await?;
        settle_durable(state);
    }
    while let Some(first) = state.journal.as_ref().and_then(|journal| journal.first()) {
        let payload = to_json(first);
        let id = transport.write(scope, &payload).await?;
        let Some(file) = state.journal.as_mut().map(|journal| journal.remove(0)) else {
            break;
        };
        state.files.insert(
            id,
            FileMeta {
                d: file.device.clone(),
                k: FileKind::Log,
                s: Some(file.seq),
                vv: None,
                at: file.at,
                b: Some(utf16_len(&payload) as f64),
            },
        );
        mark_applied(state, &file.device, file.seq);
        see(state, &file.device, now);
    }
    state.journal = None;
    settle_durable(state);
    Ok(())
}

/// Port of `restoreJournal` (`packages/sync/src/scope.ts:179`).
pub fn restore_journal(
    state: &mut ScopeState,
    batches: &[LogFile],
    clock: &mut Clock,
    written: u64,
) {
    let saved = state.seq;
    let mut sorted: Vec<&LogFile> = batches.iter().collect();
    sorted.sort_by_key(|file| file.seq);
    for file in sorted {
        if file.seq <= state.seq {
            continue;
        }
        if file.seq == saved + 1 {
            state.outbox.clear();
        }
        for op in &file.ops {
            merge_record(
                state,
                &op.k,
                SyncRecord {
                    h: op.h.clone(),
                    v: op.v.clone(),
                    o: (file.device.clone(), file.seq),
                },
                clock,
            );
            if op.v.is_null() {
                drop_base(state, &op.k);
            } else {
                put_base(state, &op.k, canonical(&op.v));
            }
        }
        state.seq = file.seq;
        if file.seq <= written {
            mark_applied(state, &file.device, file.seq);
        } else {
            state.journal.get_or_insert_default().push(file.clone());
        }
    }
    state.seq = state.seq.max(written);
}

/// Port of `writtenSeq` (`packages/sync/src/scope.ts:196`).
pub fn written_seq(state: &ScopeState) -> u64 {
    state
        .seq
        .saturating_sub(state.journal.as_ref().map_or(0, Vec::len) as u64)
}

/// Port of `pull` (`packages/sync/src/scope.ts:198`).
pub async fn pull(
    state: &mut ScopeState,
    transport: &dyn Transport,
    scope: &str,
    clock: &mut Clock,
    now: f64,
    options: &ScopeOptions,
) -> SyncResult<IndexSet<String>> {
    let damaged_give_up = options.damaged_give_up.unwrap_or(DAMAGED_GIVE_UP);
    let known: Vec<String> = state
        .files
        .iter()
        .filter(|(_, f)| f.k != FileKind::Bad)
        .map(|(id, _)| id.clone())
        .collect();
    let listing = transport.read(scope, &known).await?;
    let mut changed = IndexSet::new();
    let mut parsed = Vec::new();
    let mut unreadable: Vec<String> = listing.damaged.iter().map(|d| d.id.clone()).collect();
    for listed in &listing.files {
        match parse_sync_file(&listed.payload) {
            Some(file) => parsed.push((listed.id.clone(), file, utf16_len(&listed.payload) as f64)),
            None => unreadable.push(listed.id.clone()),
        }
    }
    parsed.sort_by_key(|(_, file, _)| !matches!(file, SyncFile::Snapshot(_)));
    for (id, file, b) in &parsed {
        let meta = match file {
            SyncFile::Snapshot(file) => {
                ingest_snapshot(state, file, clock, &mut changed);
                FileMeta {
                    d: file.device.clone(),
                    k: FileKind::Snapshot,
                    s: None,
                    vv: Some(file.vv.clone()),
                    at: file.at,
                    b: Some(*b),
                }
            }
            SyncFile::Log(file) => {
                ingest_log(state, file, clock, &mut changed);
                FileMeta {
                    d: file.device.clone(),
                    k: FileKind::Log,
                    s: Some(file.seq),
                    vv: None,
                    at: file.at,
                    b: Some(*b),
                }
            }
        };
        state.files.insert(id.clone(), meta);
    }
    let present: IndexSet<String> = listing.present.iter().cloned().collect();
    state.files.retain(|id, _| present.contains(id));
    let files = PulledFiles {
        read: parsed.into_iter().map(|(id, _, _)| id).collect(),
        unreadable,
        pending: listing.pending,
        present,
    };
    note_file_health(state, &files, now, damaged_give_up);
    Ok(changed)
}

/// `settle` of `applyAdapter` (`packages/sync/src/scope.ts:231`).
fn settle_base(state: &mut ScopeState, view: &IndexMap<String, Value>, key: &str) {
    match view.get(key) {
        Some(value) => put_base(state, key, canonical(value)),
        None => drop_base(state, key),
    }
}

/// Port of `applyAdapter` (`packages/sync/src/scope.ts:225`).
async fn apply_adapter(
    state: &mut ScopeState,
    adapter: &dyn Adapter,
    keys: &mut IndexSet<String>,
    now: f64,
    horizon: f64,
    held: &dyn Fn() -> bool,
) -> bool {
    let prefix = adapter.prefix();
    let mine: IndexSet<String> = keys
        .iter()
        .filter(|k| k.starts_with(prefix))
        .cloned()
        .collect();
    if mine.is_empty() {
        return true;
    }
    let view = visible(state, prefix);
    let joined = if state.joined {
        ApplyAs::Joined
    } else {
        ApplyAs::Joining
    };
    let apply_as = *state
        .apply_as
        .get_or_insert_default()
        .entry(prefix.to_string())
        .or_insert(joined);
    let landed = RefCell::new(IndexSet::<String>::new());
    let ok = {
        let land = |key: &str| {
            landed.borrow_mut().insert(key.to_string());
        };
        let records = ApplyRecords {
            state: &*state,
            now,
            horizon,
            joined: apply_as == ApplyAs::Joined,
            held,
            landed: &land,
        };
        match adapter.apply(&view, &mine, &records).await {
            Ok(ok) => ok,
            Err(error) => {
                eprintln!("[sync] applying {prefix} failed {error}");
                false
            }
        }
    };
    if !ok || held() {
        for key in landed.into_inner() {
            if !mine.contains(&key) {
                continue;
            }
            settle_base(state, &view, &key);
            keys.shift_remove(&key);
        }
        return false;
    }
    state
        .apply_as
        .get_or_insert_default()
        .insert(prefix.to_string(), ApplyAs::Joined);
    for key in &mine {
        settle_base(state, &view, key);
    }
    true
}

/// Port of `dominates` (`packages/sync/src/scope.ts:258`).
fn dominates(vv: &IndexMap<String, u64>, other: &IndexMap<String, u64>) -> bool {
    other
        .iter()
        .all(|(d, &s)| vv.get(d).copied().unwrap_or(0) >= s)
}

/// Port of `newestSnapshot` (`packages/sync/src/scope.ts:260`).
fn newest_snapshot<'a>(
    files: &'a IndexMap<String, FileMeta>,
    device: Option<&str>,
) -> Option<(&'a String, &'a FileMeta)> {
    let device = device.filter(|d| !d.is_empty());
    let mut newest: Option<(&String, &FileMeta)> = None;
    for (id, f) in files {
        if f.k == FileKind::Snapshot
            && device.is_none_or(|d| f.d == d)
            && newest.is_none_or(|(_, n)| f.at > n.at)
        {
            newest = Some((id, f));
        }
    }
    newest
}

/// The seq of a log meta, `Infinity` when it has none.
fn seq_of(f: &FileMeta) -> u64 {
    f.s.unwrap_or(u64::MAX)
}

/// Port of `retiredBy` (`packages/sync/src/scope.ts:266`); `snapshots` is `kinds === "logsAndSnapshots"`.
fn retired_by(
    files: &IndexMap<String, FileMeta>,
    snapshot_id: &str,
    vv: &IndexMap<String, u64>,
    now: f64,
    prune_grace: f64,
    snapshots: bool,
) -> Vec<String> {
    let empty = IndexMap::new();
    files
        .iter()
        .filter(|(id, f)| {
            if *id == snapshot_id || now - f.at < prune_grace {
                return false;
            }
            match f.k {
                FileKind::Log => covered(vv, (&f.d, seq_of(f))),
                FileKind::Snapshot => snapshots && dominates(vv, f.vv.as_ref().unwrap_or(&empty)),
                FileKind::Bad => false,
            }
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Port of `prune` (`packages/sync/src/scope.ts:277`).
async fn prune(
    state: &mut ScopeState,
    transport: &dyn Transport,
    scope: &str,
    ids: Vec<String>,
) -> SyncResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    transport.remove(scope, &ids).await?;
    for id in &ids {
        state.files.shift_remove(id);
    }
    Ok(())
}

/// Port of `compactionDue` (`packages/sync/src/scope.ts:283`).
pub fn compaction_due(state: &ScopeState, now: f64, options: &ScopeOptions) -> bool {
    due(state, now, &opts(options))
}

fn due(state: &ScopeState, now: f64, o: &Opts) -> bool {
    let latest = newest_snapshot(&state.files, None).map(|(_, f)| f);
    if now - js_max(state.snapshot_at, latest.map_or(0.0, |f| f.at)) < o.compact_interval {
        return false;
    }
    let empty = IndexMap::new();
    let base = latest.and_then(|f| f.vv.as_ref()).unwrap_or(&empty);
    let mut logs = 0.0;
    let mut log_bytes = 0.0;
    for f in state.files.values() {
        if f.k != FileKind::Log
            || covered(base, (&f.d, seq_of(f)))
            || !covered(&state.vv, (&f.d, seq_of(f)))
        {
            continue;
        }
        logs += 1.0;
        log_bytes += f.b.unwrap_or(0.0);
    }
    if logs <= o.compact_after_files {
        return false;
    }
    logs >= o.compact_after_logs
        || log_bytes >= o.compact_log_share * latest.and_then(|f| f.b).unwrap_or(0.0)
}

/// Port of `compact` (`packages/sync/src/scope.ts:300`).
pub async fn compact(
    state: &mut ScopeState,
    transport: &dyn Transport,
    scope: &str,
    device: &str,
    now: f64,
    options: &ScopeOptions,
    force: bool,
) -> SyncResult<bool> {
    let o = opts(options);
    if !force && !due(state, now, &o) {
        let own = newest_snapshot(&state.files, Some(device));
        if let Some((id, f)) = own {
            let ids = retired_by(
                &state.files,
                id,
                f.vv.as_ref().unwrap_or(&IndexMap::new()),
                now,
                o.prune_grace,
                false,
            );
            if let Err(error) = prune(state, transport, scope, ids).await {
                eprintln!("[sync] removing covered files failed {error}");
            }
        }
        return Ok(false);
    }
    flush(state, transport, scope, device, now, options).await?;
    let tombstone_ttl = if state.extra.is_empty() {
        o.tombstone_ttl
    } else {
        2.0 * o.tombstone_ttl
    };
    let mut records = IndexMap::new();
    let mut dropped = Vec::new();
    for (key, record) in &state.records {
        let drop = if record.v.is_null() {
            now - hlc_wall(&record.h) > tombstone_ttl
        } else {
            o.expired
                .as_ref()
                .is_some_and(|expired| expired(key, &record.v, now))
        };
        if drop {
            dropped.push(key.clone());
        } else {
            records.insert(key.clone(), record.clone());
        }
    }
    for key in &dropped {
        drop_record(state, key);
    }
    let vv = state.vv.clone();
    let file = SnapshotFile {
        v: 1,
        kind: SnapshotKind::Snapshot,
        device: device.to_string(),
        at: now,
        vv,
        seen: state.seen.clone(),
        records,
    };
    let payload = to_json(&file);
    let id = transport.write(scope, &payload).await?;
    let vv = file.vv;
    state.files.insert(
        id.clone(),
        FileMeta {
            d: device.to_string(),
            k: FileKind::Snapshot,
            s: None,
            vv: Some(vv.clone()),
            at: now,
            b: Some(utf16_len(&payload) as f64),
        },
    );
    state.snapshot_at = now;
    let ids = retired_by(&state.files, &id, &vv, now, o.prune_grace, true);
    prune(state, transport, scope, ids).await?;
    Ok(true)
}

/// `CycleResult` (`packages/sync/src/scope.ts:325`).
#[derive(Clone, Debug, PartialEq)]
pub struct CycleResult {
    pub published: usize,
    pub changed: usize,
    pub pending: u64,
    pub compacted: bool,
    pub snapshot_bytes: f64,
}

/// The arguments of `syncScope` (`packages/sync/src/scope.ts:327`).
pub struct SyncScopeArgs<'a> {
    pub scope: &'a str,
    pub state: &'a mut ScopeState,
    pub transport: &'a dyn Transport,
    pub adapters: &'a [Box<dyn Adapter + 'a>],
    pub clock: &'a mut Clock,
    pub device: &'a str,
    pub now: Option<Rc<dyn Fn() -> f64>>,
    pub options: ScopeOptions,
}

/// `args` of `syncScopeIndexed` without the state, which `withBaseIndex` passes.
struct Cycle<'a> {
    scope: &'a str,
    transport: &'a dyn Transport,
    adapters: &'a [Box<dyn Adapter + 'a>],
    clock: &'a mut Clock,
    device: &'a str,
    now: Rc<dyn Fn() -> f64>,
    options: ScopeOptions,
}

/// Port of `syncScope` (`packages/sync/src/scope.ts:327`).
pub async fn sync_scope(args: SyncScopeArgs<'_>) -> SyncResult<CycleResult> {
    let SyncScopeArgs {
        scope,
        state,
        transport,
        adapters,
        clock,
        device,
        now,
        options,
    } = args;
    let cycle = Cycle {
        scope,
        transport,
        adapters,
        clock,
        device,
        now: now.unwrap_or_else(|| Rc::new(now_ms)),
        options,
    };
    with_base_index(state, async move |state: &mut ScopeState| {
        sync_scope_indexed(state, cycle).await
    })
    .await
}

/// Port of `syncScopeIndexed` (`packages/sync/src/scope.ts:340`).
async fn sync_scope_indexed(state: &mut ScopeState, args: Cycle<'_>) -> SyncResult<CycleResult> {
    let Cycle {
        scope,
        transport,
        adapters,
        clock,
        device,
        now,
        options,
    } = args;
    let mut published = 0;
    let mut stamps: Vec<Option<Stamps>> = Vec::with_capacity(adapters.len());
    for adapter in adapters {
        let adapter = adapter.as_ref();
        if adapter.publish_after_pull() {
            stamps.push(Some(candidate_stamps(state, adapter, clock, now()).await?));
        } else {
            stamps.push(None);
            published += publish_adapter(state, adapter, clock, device, now(), None)
                .await?
                .len();
        }
    }
    flush(state, transport, scope, device, now(), &options).await?;
    let changed = pull(state, transport, scope, clock, now(), &options).await?;
    let held = holds_back(
        state,
        now(),
        options.damaged_give_up.unwrap_or(DAMAGED_GIVE_UP),
    );
    let mut keys: IndexSet<String> = state
        .unapplied
        .iter()
        .cloned()
        .chain(changed.iter().cloned())
        .collect();
    let mut unapplied = Vec::new();
    let mut held_apply = false;
    let apply_held = || options.hold_apply.as_ref().is_some_and(|hold| hold());
    for (adapter, stamps) in adapters.iter().zip(&stamps) {
        let adapter = adapter.as_ref();
        let late = if adapter.publish_after_pull() && held {
            IndexSet::new()
        } else {
            publish_adapter(state, adapter, clock, device, now(), stamps.as_ref()).await?
        };
        published += late.len();
        for key in &late {
            keys.shift_remove(key);
        }
        held_apply = held_apply || apply_held();
        let applied = !held_apply
            && apply_adapter(
                state,
                adapter,
                &mut keys,
                now(),
                2.0 * options.tombstone_ttl.unwrap_or(TOMBSTONE_TTL),
                &apply_held,
            )
            .await;
        held_apply = held_apply || apply_held();
        if !applied {
            unapplied.extend(
                keys.iter()
                    .filter(|k| k.starts_with(adapter.prefix()))
                    .cloned(),
            );
        }
    }
    unapplied.extend(
        keys.into_iter()
            .filter(|k| !adapters.iter().any(|a| k.starts_with(a.prefix()))),
    );
    state.unapplied = unapplied;
    if !held_apply
        && (!state.await_remote.unwrap_or(false) || state.seen.keys().any(|d| d != device))
    {
        state.joined = true;
        state.await_remote = None;
    }
    flush(state, transport, scope, device, now(), &options).await?;
    let compacted = compact(state, transport, scope, device, now(), &options, false).await?;
    let snapshot_bytes = if compacted {
        newest_snapshot(&state.files, Some(device))
            .and_then(|(_, f)| f.b)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    Ok(CycleResult {
        published,
        changed: changed.len(),
        pending: state.pending,
        compacted,
        snapshot_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::scope_state::empty_scope;

    #[test]
    fn parse_takes_js_numbers_and_refuses_bad_shapes() {
        let log = parse_sync_file(
            r#"{"v":1.0,"kind":"log","device":"d","seq":2.0,"at":5,"ops":[{"k":"x","h":"h","v":null}]}"#,
        );
        assert!(matches!(log, Some(SyncFile::Log(LogFile { seq: 2, .. }))));
        for bad in [
            r#"{"v":1,"kind":"log","device":"d","seq":-1,"at":5,"ops":[]}"#,
            r#"{"v":1,"kind":"log","device":"d","seq":1,"at":5,"ops":[{"k":"x","h":"h"}]}"#,
            "[1]",
            "{",
        ] {
            assert_eq!(parse_sync_file(bad), None, "{bad}");
        }
    }

    #[test]
    fn parse_refuses_values_nested_past_the_json_depth_limit() {
        let file = |depth: usize| {
            format!(
                r#"{{"v":1,"kind":"log","device":"d","seq":1,"at":5,"ops":[{{"k":"x","h":"h","v":{}0{}}}]}}"#,
                "[".repeat(depth),
                "]".repeat(depth)
            )
        };
        assert!(parse_sync_file(&file(100)).is_some());
        assert_eq!(parse_sync_file(&file(200)), None);
    }

    #[test]
    fn restore_journal_sorts_and_keeps_unwritten_files() {
        let mut state = empty_scope();
        let mut clock = Clock::new("d", None, Some(Rc::new(|| 10.0)));
        let file = |seq| LogFile {
            v: 1,
            kind: LogKind::Log,
            device: "d".into(),
            seq,
            at: 1.0,
            ops: vec![Op {
                k: "x".into(),
                h: format!("00000000{seq}.0000.d"),
                v: Value::from(seq),
            }],
        };
        restore_journal(&mut state, &[file(2), file(1)], &mut clock, 1);
        assert_eq!(state.seq, 2);
        assert_eq!(state.vv.get("d"), Some(&1));
        assert_eq!(state.journal.as_ref().map(|j| j[0].seq), Some(2));
        assert_eq!(state.base.get("x").map(String::as_str), Some("2"));
        assert_eq!(written_seq(&state), 1);
    }
}
