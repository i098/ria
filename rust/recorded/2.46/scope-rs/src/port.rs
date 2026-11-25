// Ria original, no Ardia source.

//! The Rust side of `scope.harness.ts`: the same scripted folder, adapters and options, driven
//! through `ria_sync::logic::scope`.

#![forbid(unsafe_code)]

use indexmap::{IndexMap, IndexSet};
use ria_sync::logic::canon::canonical;
use ria_sync::logic::hlc::Clock;
use ria_sync::logic::scope::{
    SyncScopeArgs, compact, compaction_due, flush, ingest_snapshot, parse_sync_file, pull,
    restore_journal, sync_scope, written_seq,
};
use ria_sync::logic::scope_state::{
    Adapter, ApplyRecords, DamagedFile, ExtractCtx, Extraction, ListedFile, Listing,
    LocalBoxFuture, LogFile, ScopeOptions, ScopeState, SnapshotFile, SyncFile, SyncResult,
    Transport, empty_scope,
};
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

const NUMBERS: [&str; 7] = [
    "tombstoneTTL",
    "compactAfterFiles",
    "compactAfterLogs",
    "compactLogShare",
    "compactInterval",
    "pruneGrace",
    "damagedGiveUp",
];

type Store = Rc<RefCell<IndexMap<String, Value>>>;
type Persist = Rc<dyn for<'a> Fn(&'a [LogFile]) -> LocalBoxFuture<'a, SyncResult<()>>>;

#[derive(Default)]
struct Entry {
    payload: String,
    hidden: Vec<String>,
    partial: Vec<String>,
}

/// The parts of the world that the transport, the adapters and the options reach.
#[derive(Default)]
struct Shared {
    now: Cell<f64>,
    counter: Cell<u64>,
    folder: RefCell<IndexMap<String, Entry>>,
    log: RefCell<Vec<Value>>,
}

#[derive(Clone)]
struct AdapterCfg {
    config: Value,
    store: Store,
}

struct Device {
    id: String,
    state: ScopeState,
    clock: Clock,
    offset: Rc<Cell<f64>>,
    adapters: Vec<AdapterCfg>,
    options: Value,
    holds: Rc<RefCell<VecDeque<bool>>>,
    journal: Rc<RefCell<IndexMap<u64, LogFile>>>,
    fail: Rc<RefCell<IndexSet<String>>>,
    on_read: Rc<RefCell<Option<Vec<Value>>>>,
    saved: String,
    saved_clock: Option<String>,
}

struct World {
    w: Rc<Shared>,
    devices: IndexMap<String, Device>,
    current: String,
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

fn u(v: Option<Value>) -> Value {
    v.unwrap_or_else(|| "<undefined>".into())
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn list(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|x| text(x).to_string())
        .collect()
}

fn time(w: &Rc<Shared>, offset: &Rc<Cell<f64>>) -> Rc<dyn Fn() -> f64> {
    let (w, offset) = (Rc::clone(w), Rc::clone(offset));
    Rc::new(move || w.now.get() + offset.get())
}

fn edit(adapters: &[AdapterCfg], key: &str, value: Option<Value>) {
    let Some(adapter) = adapters
        .iter()
        .find(|a| key.starts_with(text(&a.config["prefix"])))
    else {
        return;
    };
    let mut store = adapter.store.borrow_mut();
    match value {
        Some(value) => {
            store.insert(key.to_string(), value);
        }
        None => {
            store.shift_remove(key);
        }
    }
}

fn configure(d: &mut Device, config: &Value) {
    if let Some(adapters) = config["adapters"].as_array() {
        d.adapters = adapters
            .iter()
            .map(|c| AdapterCfg {
                config: c.clone(),
                store: Rc::new(RefCell::new(
                    c["store"]
                        .as_object()
                        .map(|s| s.clone().into_iter().collect())
                        .unwrap_or_default(),
                )),
            })
            .collect();
    }
    if config["options"].is_object() {
        d.options = config["options"].clone();
        *d.holds.borrow_mut() = d.options["holdApply"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|h| h.as_bool().unwrap_or(false))
            .collect();
    }
    if let Some(offset) = config["offset"].as_f64() {
        d.offset.set(offset);
    }
}

fn device(w: &Rc<Shared>, id: &str) -> Device {
    let offset = Rc::new(Cell::new(0.0));
    Device {
        id: id.to_string(),
        state: empty_scope(),
        clock: Clock::new(id, None, Some(time(w, &offset))),
        offset,
        adapters: Vec::new(),
        options: json!({}),
        holds: Rc::default(),
        journal: Rc::default(),
        fail: Rc::default(),
        on_read: Rc::default(),
        saved: serde_json::to_string(&empty_scope()).unwrap_or_default(),
        saved_clock: None,
    }
}

struct ScriptTransport {
    w: Rc<Shared>,
    id: String,
    fail: Rc<RefCell<IndexSet<String>>>,
    on_read: Rc<RefCell<Option<Vec<Value>>>>,
    adapters: Vec<AdapterCfg>,
}

impl ScriptTransport {
    fn failed(&self, op: &str) -> bool {
        self.fail.borrow_mut().shift_remove(op)
    }
}

impl Transport for ScriptTransport {
    fn write<'a>(
        &'a self,
        scope: &'a str,
        payload: &'a str,
    ) -> LocalBoxFuture<'a, SyncResult<String>> {
        Box::pin(async move {
            self.w
                .log
                .borrow_mut()
                .push(json!(["write", self.id, scope]));
            if self.failed("write") {
                return Err("write failed".into());
            }
            self.w.counter.set(self.w.counter.get() + 1);
            let id = format!("f{}", self.w.counter.get());
            self.w.folder.borrow_mut().insert(
                id.clone(),
                Entry {
                    payload: payload.to_string(),
                    ..Entry::default()
                },
            );
            if self.failed("writeAfter") {
                return Err("write timed out".into());
            }
            Ok(id)
        })
    }

    fn read<'a>(
        &'a self,
        scope: &'a str,
        known: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<Listing>> {
        Box::pin(async move {
            self.w
                .log
                .borrow_mut()
                .push(json!(["read", self.id, scope, known]));
            if self.failed("read") {
                return Err("read failed".into());
            }
            let mut listing = Listing::default();
            for (id, e) in self.w.folder.borrow().iter() {
                listing.present.push(id.clone());
                if known.contains(id) {
                    continue;
                }
                if e.hidden.contains(&self.id) {
                    listing.pending.push(id.clone());
                } else if e.partial.contains(&self.id) {
                    listing.damaged.push(DamagedFile {
                        id: id.clone(),
                        age: 0.0,
                    });
                } else {
                    listing.files.push(ListedFile {
                        id: id.clone(),
                        payload: e.payload.clone(),
                    });
                }
            }
            for change in self.on_read.borrow_mut().take().unwrap_or_default() {
                let change = change.as_array().cloned().unwrap_or_default();
                edit(
                    &self.adapters,
                    change.first().map(text).unwrap_or_default(),
                    change.get(1).cloned(),
                );
            }
            Ok(listing)
        })
    }

    fn remove<'a>(
        &'a self,
        scope: &'a str,
        ids: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<()>> {
        Box::pin(async move {
            self.w
                .log
                .borrow_mut()
                .push(json!(["remove", self.id, scope, ids]));
            if self.failed("remove") {
                return Err("remove failed".into());
            }
            for id in ids {
                self.w.folder.borrow_mut().shift_remove(id);
            }
            Ok(())
        })
    }
}

struct ScriptAdapter {
    w: Rc<Shared>,
    c: Value,
    store: Store,
}

impl Adapter for ScriptAdapter {
    fn prefix(&self) -> &str {
        text(&self.c["prefix"])
    }

    fn extract<'a>(
        &'a self,
        ctx: &'a ExtractCtx<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Extraction>>> {
        Box::pin(async move {
            let prefix = self.prefix().to_string();
            let mut log = self.w.log.borrow_mut();
            log.push(json!(["extract", prefix, ctx.now]));
            for k in list(&self.c["probe"]) {
                let (remote, stamp, base_json) = (
                    ctx.remote(&k).cloned(),
                    ctx.stamp(&k).map(Value::from),
                    ctx.base_json(&k).map(Value::from),
                );
                log.push(json!([
                    "probe",
                    k,
                    u(ctx.base(&k)),
                    ctx.queued(&k),
                    ctx.known(&k),
                    u(remote),
                    u(stamp),
                    u(base_json)
                ]));
            }
            match self.c["extract"].as_str() {
                Some("null") => return Ok(None),
                Some("throw") => return Err("extract failed".into()),
                _ => {}
            }
            let (republish, ignore, known) = (
                self.c["republish"]
                    .as_array()
                    .map(|_| list(&self.c["republish"])),
                self.c["ignore"].as_array().map(|_| list(&self.c["ignore"])),
                self.c["json"].as_object().cloned(),
            );
            let w = Rc::clone(&self.w);
            Ok(Some(Extraction {
                values: self.store.borrow().clone(),
                republish: republish.map(|keys| {
                    Box::new(move |k: &str| keys.iter().any(|x| x == k))
                        as Box<dyn Fn(&str) -> bool>
                }),
                ignore: ignore.map(|keys| {
                    Box::new(move |k: &str| keys.iter().any(|x| x == k))
                        as Box<dyn Fn(&str) -> bool>
                }),
                json: known.map(|json| {
                    Box::new(move |k: &str, _: &Value| {
                        json.get(k).and_then(Value::as_str).map(str::to_string)
                    }) as Box<dyn Fn(&str, &Value) -> Option<String>>
                }),
                published: (self.c["published"] == true).then(|| {
                    Box::new(move |keys: &IndexSet<String>| {
                        w.log.borrow_mut().push(json!(["published", prefix, keys]))
                    }) as Box<dyn FnOnce(&IndexSet<String>)>
                }),
            }))
        })
    }

    fn apply<'a>(
        &'a self,
        visible: &'a IndexMap<String, Value>,
        changed: &'a IndexSet<String>,
        records: &'a ApplyRecords<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<bool>> {
        Box::pin(async move {
            let base = |k: &str| {
                records
                    .state
                    .base
                    .get(k)
                    .and_then(|json| serde_json::from_str::<Value>(json).ok())
            };
            let known: Vec<Value> = changed
                .iter()
                .map(|k| json!([records.known(k), u(base(k))]))
                .collect();
            let mut entry = vec![
                json!("apply"),
                json!(self.prefix()),
                json!(changed),
                json!(records.joined),
                json!(records.now),
                json!(records.horizon),
                json!(known),
            ];
            if self.c["checkHeld"] == true {
                entry.push(json!((records.held)()));
            }
            self.w.log.borrow_mut().push(Value::Array(entry));
            let mode = self.c["apply"].as_str();
            if mode == Some("throw") {
                return Err("apply failed".into());
            }
            for k in changed {
                match visible.get(k) {
                    Some(v) => {
                        self.store.borrow_mut().insert(k.clone(), v.clone());
                    }
                    None if mode != Some("sets") => {
                        self.store.borrow_mut().shift_remove(k);
                    }
                    None => {}
                }
                if self.c["land"] == true {
                    (records.landed)(k);
                }
            }
            for k in list(&self.c["landed"]) {
                (records.landed)(&k);
            }
            Ok(mode != Some("refuse"))
        })
    }

    fn edited_at(&self, value: &Value) -> Option<f64> {
        value
            .as_object()?
            .get(self.c["editedAt"].as_str()?)?
            .as_f64()
    }

    fn adopt_remote_on_join(&self) -> bool {
        self.c["adoptRemoteOnJoin"] == true
    }

    fn publish_after_pull(&self) -> bool {
        self.c["publishAfterPull"] == true
    }
}

fn persist_hook<F: for<'a> Fn(&'a [LogFile]) -> LocalBoxFuture<'a, SyncResult<()>> + 'static>(
    f: F,
) -> Persist {
    Rc::new(f)
}

fn options(w: &Rc<Shared>, d: &Device) -> ScopeOptions {
    let o = &d.options;
    let number = |k: &str| o[k].as_f64();
    let mut out = ScopeOptions {
        tombstone_ttl: number(NUMBERS[0]),
        compact_after_files: number(NUMBERS[1]),
        compact_after_logs: number(NUMBERS[2]),
        compact_log_share: number(NUMBERS[3]),
        compact_interval: number(NUMBERS[4]),
        prune_grace: number(NUMBERS[5]),
        damaged_give_up: number(NUMBERS[6]),
        ..ScopeOptions::default()
    };
    if o["expired"].is_array() {
        let expired = list(&o["expired"]);
        out.expired = Some(Rc::new(move |key: &str, _: &Value, _: f64| {
            expired.iter().any(|x| x == key)
        }));
    }
    if o["persist"] == true {
        let (w, id, fail, journal) = (
            Rc::clone(w),
            d.id.clone(),
            Rc::clone(&d.fail),
            Rc::clone(&d.journal),
        );
        out.persist = Some(persist_hook(move |batches| {
            w.log.borrow_mut().push(json!([
                "persist",
                id,
                batches.iter().map(|f| f.seq).collect::<Vec<_>>()
            ]));
            let result = if fail.borrow_mut().shift_remove("persist") {
                Err("persist failed".to_string())
            } else {
                let mut journal = journal.borrow_mut();
                for f in batches {
                    journal.insert(f.seq, f.clone());
                }
                Ok(())
            };
            Box::pin(std::future::ready(result))
        }));
    }
    if o["holdApply"].is_array() {
        let (w, holds) = (Rc::clone(w), Rc::clone(&d.holds));
        out.hold_apply = Some(Rc::new(move || {
            let held = holds.borrow_mut().pop_front().unwrap_or(false);
            w.log.borrow_mut().push(json!(["hold", held]));
            held
        }));
    }
    out
}

fn transport(w: &Rc<Shared>, d: &Device) -> ScriptTransport {
    ScriptTransport {
        w: Rc::clone(w),
        id: d.id.clone(),
        fail: Rc::clone(&d.fail),
        on_read: Rc::clone(&d.on_read),
        adapters: d.adapters.clone(),
    }
}

fn show(w: &Rc<Shared>, d: &Device) -> Result<Value, String> {
    let out = json!({
        "device": d.id,
        "state": serde_json::to_value(&d.state).map_err(err)?,
        "clock": d.clock.last(),
        "stores": d.adapters.iter().map(|a| json!(*a.store.borrow())).collect::<Vec<_>>(),
        "journal": d.journal.borrow().keys().collect::<Vec<_>>(),
        "folder": w.folder.borrow().iter().map(|(id, e)| json!([id, e.payload])).collect::<Vec<_>>(),
        "log": std::mem::take(&mut *w.log.borrow_mut()),
    });
    Ok(json!(canonical(&out)))
}

/// `{ ...emptyScope(), ...partial }`.
fn state_from(partial: &Value) -> Result<ScopeState, String> {
    let mut state = serde_json::to_value(empty_scope()).map_err(err)?;
    if let (Some(state), Some(partial)) = (state.as_object_mut(), partial.as_object()) {
        state.extend(partial.clone());
    }
    serde_json::from_value(state).map_err(err)
}

fn entry<'a>(folder: &'a mut IndexMap<String, Entry>, id: &str) -> &'a mut Entry {
    folder.entry(id.to_string()).or_default()
}

async fn run(world: &mut World, step: &[Value]) -> Result<Value, String> {
    let (a, b) = (
        step.get(1).unwrap_or(&Value::Null),
        step.get(2).unwrap_or(&Value::Null),
    );
    let name = step.first().and_then(Value::as_str).unwrap_or_default();
    let w = Rc::clone(&world.w);
    if name == "device" {
        let id = text(a).to_string();
        let next = world
            .devices
            .entry(id.clone())
            .or_insert_with(|| device(&w, &id));
        configure(next, b);
        world.current = id;
        return Ok(Value::Null);
    }
    let Some(d) = world.devices.get_mut(&world.current) else {
        return Err("no device".into());
    };
    let now = time(&w, &d.offset);
    let (t, o) = (transport(&w, d), options(&w, d));
    Ok(match name {
        "now" => {
            w.now.set(a.as_f64().unwrap_or_default());
            Value::Null
        }
        "tick" => {
            w.now.set(w.now.get() + a.as_f64().unwrap_or_default());
            Value::Null
        }
        "state" => {
            d.state = state_from(a)?;
            Value::Null
        }
        "relaunch" => {
            d.state = d.state.clone();
            d.clock = Clock::new(&d.id, Some(&d.clock.last()), Some(now));
            Value::Null
        }
        "clock" => {
            d.clock = Clock::new(&d.id, a.as_str(), Some(now));
            json!(d.clock.last())
        }
        "set" => {
            edit(&d.adapters, text(a), Some(b.clone()));
            Value::Null
        }
        "del" => {
            edit(&d.adapters, text(a), None);
            Value::Null
        }
        "onRead" => {
            *d.on_read.borrow_mut() = a.as_array().cloned();
            Value::Null
        }
        "fail" => {
            d.fail.borrow_mut().insert(text(a).to_string());
            Value::Null
        }
        "file" => {
            entry(&mut w.folder.borrow_mut(), text(a)).payload = text(b).to_string();
            Value::Null
        }
        "drop" => {
            w.folder.borrow_mut().shift_remove(text(a));
            Value::Null
        }
        "hide" | "partial" => {
            let mut folder = w.folder.borrow_mut();
            let e = entry(&mut folder, text(a));
            let list = if name == "hide" {
                &mut e.hidden
            } else {
                &mut e.partial
            };
            if b.is_null() {
                list.clear();
            } else {
                list.push(text(b).to_string());
            }
            Value::Null
        }
        "sync" => {
            let adapters: Vec<Box<dyn Adapter>> = d
                .adapters
                .iter()
                .map(|x| {
                    Box::new(ScriptAdapter {
                        w: Rc::clone(&w),
                        c: x.config.clone(),
                        store: Rc::clone(&x.store),
                    }) as Box<dyn Adapter>
                })
                .collect();
            let r = sync_scope(SyncScopeArgs {
                scope: "s",
                state: &mut d.state,
                transport: &t,
                adapters: &adapters,
                clock: &mut d.clock,
                device: &d.id,
                now: Some(now),
                options: o,
            })
            .await?;
            json!({"published": r.published, "changed": r.changed, "pending": r.pending, "compacted": r.compacted, "snapshotBytes": r.snapshot_bytes})
        }
        "flush" => {
            flush(&mut d.state, &t, "s", &d.id, now(), &o).await?;
            Value::Null
        }
        "pull" => json!(pull(&mut d.state, &t, "s", &mut d.clock, now(), &o).await?),
        "compact" => json!(compact(&mut d.state, &t, "s", &d.id, now(), &o, a == true).await?),
        "due" => json!(compaction_due(&d.state, now(), &o)),
        "restore" => {
            let batches: Vec<LogFile> = if a.is_null() {
                d.journal.borrow().values().cloned().collect()
            } else {
                serde_json::from_value(a.clone()).map_err(err)?
            };
            restore_journal(
                &mut d.state,
                &batches,
                &mut d.clock,
                b.as_u64().unwrap_or(0),
            );
            Value::Null
        }
        "save" => {
            d.saved = serde_json::to_string(&d.state).map_err(err)?;
            d.saved_clock = Some(d.clock.last());
            let seq = d.state.seq;
            d.journal.borrow_mut().retain(|&s, _| s > seq);
            Value::Null
        }
        "crash" => {
            d.state = serde_json::from_str(&d.saved).map_err(err)?;
            d.clock = Clock::new(&d.id, d.saved_clock.as_deref(), Some(now));
            let batches: Vec<LogFile> = d.journal.borrow().values().cloned().collect();
            restore_journal(&mut d.state, &batches, &mut d.clock, 0);
            Value::Null
        }
        "parse" => {
            let file = match parse_sync_file(text(a)) {
                Some(SyncFile::Log(f)) => serde_json::to_value(f).map_err(err)?,
                Some(SyncFile::Snapshot(f)) => serde_json::to_value(f).map_err(err)?,
                None => Value::Null,
            };
            json!(canonical(&file))
        }
        "ingest" => {
            let file: SnapshotFile = serde_json::from_value(a.clone()).map_err(err)?;
            let mut changed = IndexSet::new();
            ingest_snapshot(&mut d.state, &file, &mut d.clock, &mut changed);
            json!(changed)
        }
        "written" => json!(written_seq(&d.state)),
        "show" => show(&w, d)?,
        _ => json!({"error": "bad step"}),
    })
}

async fn replay(steps: Vec<Value>) -> Vec<Value> {
    let w = Rc::new(Shared {
        now: Cell::new(1000.0),
        ..Shared::default()
    });
    let first = device(&w, "devA");
    let mut world = World {
        w,
        devices: IndexMap::from([("devA".to_string(), first)]),
        current: "devA".into(),
    };
    let mut out = Vec::new();
    for step in steps {
        out.push(match step.as_array() {
            Some(step) => run(&mut world, step)
                .await
                .unwrap_or_else(|error| json!({"error": error})),
            None => json!({"error": "bad step"}),
        });
    }
    out
}

/// Port of `replayScope` (`packages/sync/src/scope.harness.ts:279`).
pub async fn replay_scope(input: String) -> Result<Vec<Value>, String> {
    let Ok(Value::Array(steps)) = serde_json::from_str::<Value>(&input) else {
        return Ok(vec![json!({"error": "bad script"})]);
    };
    Ok(futures::executor::block_on(replay(steps)))
}
