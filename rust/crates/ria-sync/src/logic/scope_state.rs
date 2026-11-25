// Ported from ardia packages/sync/src/scopeState.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The state of one sync scope and the contracts of its transport and adapters.
//!
//! Changes from Ardia:
//! - The `WeakMap`s of `scope.ts` and `scopeKeys.ts` (touched keys, base index, settle queue)
//!   are the `runtime` field of `ScopeState`, which JSON never carries.
//! - An adapter reads the scope through the `state` of its context, not through a state that
//!   it captured at construction; the scope is borrowed, not shared.
//! - Async methods return a boxed local future (`LocalBoxFuture`), so adapters stay trait objects.
//! - A JS `throw` is an `Err(String)`.

use super::hlc::Hlc;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// A future that need not be `Send`.
pub type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// What a sync call can fail with: the text of the JS `Error`.
pub type SyncResult<T> = Result<T, String>;

/// `[device, seq]`: the log file that a record came from.
pub type Origin = (String, u64);

/// A value of a key with its timestamp and origin; `v` is `null` for a tombstone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyncRecord {
    pub h: Hlc,
    pub v: Value,
    pub o: Origin,
}

/// One change in a log file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Op {
    pub k: String,
    pub h: Hlc,
    pub v: Value,
}

/// `"log"`: the `kind` of a `LogFile`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogKind {
    #[default]
    #[serde(rename = "log")]
    Log,
}

/// `"snapshot"`: the `kind` of a `SnapshotFile`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotKind {
    #[default]
    #[serde(rename = "snapshot")]
    Snapshot,
}

/// The changes of one device in one batch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogFile {
    pub v: u8,
    pub kind: LogKind,
    pub device: String,
    pub seq: u64,
    pub at: f64,
    pub ops: Vec<Op>,
}

/// All records of a scope at one time, with the versions that it covers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub v: u8,
    pub kind: SnapshotKind,
    pub device: String,
    pub at: f64,
    pub vv: IndexMap<String, u64>,
    pub seen: IndexMap<String, f64>,
    pub records: IndexMap<String, SyncRecord>,
}

/// A file of a scope.
#[derive(Clone, Debug, PartialEq)]
pub enum SyncFile {
    Log(LogFile),
    Snapshot(SnapshotFile),
}

/// The kind of a `FileMeta`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileKind {
    #[serde(rename = "log")]
    Log,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "bad")]
    Bad,
}

/// What a scope knows of one file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileMeta {
    pub d: String,
    pub k: FileKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vv: Option<IndexMap<String, u64>>,
    pub at: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<f64>,
}

/// `"joined"` or `"joining"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplyAs {
    #[serde(rename = "joined")]
    Joined,
    #[serde(rename = "joining")]
    Joining,
}

/// The keys that a caller changed since it started to track (`trackTouchedKeys`).
#[derive(Clone, Debug, Default)]
pub struct TouchedKeys {
    pub records: IndexSet<String>,
    pub base: IndexSet<String>,
}

/// The keys of `state.base`, in order, and the same keys for each prefix asked so far.
#[derive(Clone, Debug, Default)]
pub struct BaseIndex {
    pub keys: Vec<String>,
    pub by_prefix: HashMap<String, Vec<String>>,
}

/// What the `WeakMap`s of Ardia keep for a state. It is never saved and a clone starts empty.
#[derive(Default)]
pub struct Runtime {
    /// `touching` of `scopeKeys.ts`: one set for each tracker that is running.
    pub touching: Vec<Rc<RefCell<TouchedKeys>>>,
    /// `baseIndexes` of `scopeKeys.ts`: set while `withBaseIndex` runs.
    pub base_index: Option<BaseIndex>,
    /// `settling` of `scope.ts`: callbacks that run when the journal is durable.
    pub settling: Vec<Box<dyn FnOnce()>>,
}

impl Clone for Runtime {
    fn clone(&self) -> Self {
        Runtime::default()
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime").finish_non_exhaustive()
    }
}

impl PartialEq for Runtime {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Everything that one device knows of one scope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScopeState {
    pub records: IndexMap<String, SyncRecord>,
    pub base: IndexMap<String, String>,
    pub unapplied: Vec<String>,
    pub vv: IndexMap<String, u64>,
    pub extra: IndexMap<String, Vec<u64>>,
    pub files: IndexMap<String, FileMeta>,
    pub damaged: IndexMap<String, f64>,
    #[serde(
        default,
        rename = "pendingSince",
        skip_serializing_if = "Option::is_none"
    )]
    pub pending_since: Option<IndexMap<String, f64>>,
    pub seq: u64,
    pub outbox: Vec<Op>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal: Option<Vec<LogFile>>,
    pub seen: IndexMap<String, f64>,
    pub joined: bool,
    #[serde(
        default,
        rename = "awaitRemote",
        skip_serializing_if = "Option::is_none"
    )]
    pub await_remote: Option<bool>,
    #[serde(default, rename = "applyAs", skip_serializing_if = "Option::is_none")]
    pub apply_as: Option<IndexMap<String, ApplyAs>>,
    #[serde(rename = "snapshotAt")]
    pub snapshot_at: f64,
    pub pending: u64,
    #[serde(skip)]
    pub runtime: Runtime,
}

/// Port of `emptyScope` (`packages/sync/src/scopeState.ts:41`).
pub fn empty_scope() -> ScopeState {
    ScopeState {
        records: IndexMap::new(),
        base: IndexMap::new(),
        unapplied: Vec::new(),
        vv: IndexMap::new(),
        extra: IndexMap::new(),
        files: IndexMap::new(),
        damaged: IndexMap::new(),
        pending_since: None,
        seq: 0,
        outbox: Vec::new(),
        journal: None,
        seen: IndexMap::new(),
        joined: false,
        await_remote: None,
        apply_as: None,
        snapshot_at: 0.0,
        pending: 0,
        runtime: Runtime::default(),
    }
}

/// A file that a transport lists as readable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListedFile {
    pub id: String,
    pub payload: String,
}

/// A file that a transport lists as unreadable, with its age.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DamagedFile {
    pub id: String,
    pub age: f64,
}

/// What a transport finds in a scope.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Listing {
    pub files: Vec<ListedFile>,
    pub pending: Vec<String>,
    pub damaged: Vec<DamagedFile>,
    pub present: Vec<String>,
}

/// The place where the files of a scope live (a folder that the user chose, or a test double).
pub trait Transport {
    fn write<'a>(
        &'a self,
        scope: &'a str,
        payload: &'a str,
    ) -> LocalBoxFuture<'a, SyncResult<String>>;
    fn read<'a>(
        &'a self,
        scope: &'a str,
        known: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<Listing>>;
    fn remove<'a>(
        &'a self,
        scope: &'a str,
        ids: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<()>>;
}

/// What an adapter offers to publish.
#[derive(Default)]
pub struct Extraction {
    pub values: IndexMap<String, Value>,
    pub ignore: Option<Box<dyn Fn(&str) -> bool>>,
    pub republish: Option<Box<dyn Fn(&str) -> bool>>,
    /// Called once with the keys that were published, when the journal is durable.
    pub published: Option<Box<dyn FnOnce(&IndexSet<String>)>>,
    pub json: Option<Box<dyn Fn(&str, &Value) -> Option<String>>>,
}

/// The arguments of `Adapter::extract`: the callbacks of `Adapter.extract` in Ardia, as methods.
pub struct ExtractCtx<'a> {
    pub state: &'a ScopeState,
    pub now: f64,
    pending: OnceCell<IndexSet<String>>,
}

impl<'a> ExtractCtx<'a> {
    pub fn new(state: &'a ScopeState, now: f64) -> Self {
        ExtractCtx {
            state,
            now,
            pending: OnceCell::new(),
        }
    }

    /// `base(key)`: the parsed JSON of `state.base[key]`.
    pub fn base(&self, key: &str) -> Option<Value> {
        self.state
            .base
            .get(key)
            .and_then(|json| serde_json::from_str(json).ok())
    }

    /// `queued(key)`: is a change of `key` in the outbox or the journal?
    pub fn queued(&self, key: &str) -> bool {
        self.pending
            .get_or_init(|| {
                let journal = self
                    .state
                    .journal
                    .iter()
                    .flatten()
                    .flat_map(|f| f.ops.iter());
                self.state
                    .outbox
                    .iter()
                    .chain(journal)
                    .map(|op| op.k.clone())
                    .collect()
            })
            .contains(key)
    }

    /// `known(key)`: is there a record of `key`?
    pub fn known(&self, key: &str) -> bool {
        self.state.records.contains_key(key)
    }

    /// `remote(key)`: the value of the record (`Some(Null)` for a tombstone, `None` for no record).
    pub fn remote(&self, key: &str) -> Option<&Value> {
        self.state.records.get(key).map(|r| &r.v)
    }

    /// `stamp(key)`: the timestamp of the record.
    pub fn stamp(&self, key: &str) -> Option<&str> {
        self.state.records.get(key).map(|r| r.h.as_str())
    }

    /// `baseJson(key)`.
    pub fn base_json(&self, key: &str) -> Option<&str> {
        self.state.base.get(key).map(String::as_str)
    }
}

/// The arguments of `Adapter::apply` after `visible` and `changed`.
pub struct ApplyRecords<'a> {
    pub state: &'a ScopeState,
    pub now: f64,
    pub horizon: f64,
    pub joined: bool,
    pub held: &'a dyn Fn() -> bool,
    pub landed: &'a dyn Fn(&str),
}

impl ApplyRecords<'_> {
    /// `known(key)`: is there a record of `key`?
    pub fn known(&self, key: &str) -> bool {
        self.state.records.contains_key(key)
    }
}

/// A part of the app that syncs the keys below one prefix.
pub trait Adapter {
    fn prefix(&self) -> &str;

    fn extract<'a>(
        &'a self,
        ctx: &'a ExtractCtx<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Extraction>>>;

    /// `Ok(false)` asks for a later retry; `Ok(true)` covers a JS `true` and `undefined`.
    fn apply<'a>(
        &'a self,
        visible: &'a IndexMap<String, Value>,
        changed: &'a IndexSet<String>,
        records: &'a ApplyRecords<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<bool>>;

    fn edited_at(&self, _value: &Value) -> Option<f64> {
        None
    }

    fn adopt_remote_on_join(&self) -> bool {
        false
    }

    fn publish_after_pull(&self) -> bool {
        false
    }
}

/// The options of a scope; `None` takes the default of `scope.ts`.
#[derive(Clone, Default)]
pub struct ScopeOptions {
    pub expired: Option<Rc<dyn Fn(&str, &Value, f64) -> bool>>,
    pub tombstone_ttl: Option<f64>,
    pub compact_after_files: Option<f64>,
    pub compact_after_logs: Option<f64>,
    pub compact_log_share: Option<f64>,
    pub compact_interval: Option<f64>,
    pub prune_grace: Option<f64>,
    pub damaged_give_up: Option<f64>,
    pub persist: Option<Rc<dyn for<'a> Fn(&'a [LogFile]) -> LocalBoxFuture<'a, SyncResult<()>>>>,
    pub hold_apply: Option<Rc<dyn Fn() -> bool>>,
}
