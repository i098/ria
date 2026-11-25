// Ria original, no Ardia source.

//! The Rust side of `companion-cycle.harness.ts`: the same scripted host and steps, driven through
//! `ria_sync::logic::companion`.

#![forbid(unsafe_code)]

use indexmap::IndexMap;
use ria_sync::logic::companion::adapters::{day_of, history_expired, is_web_url, last_visit_of};
use ria_sync::logic::companion::cycle::{
    AcknowledgeInput, AddBookmarkInput, CycleInput, RemovalsDoneInput, RemoveBookmarkInput,
    SendInput, companion_acknowledge, companion_add_bookmark, companion_removals_done,
    companion_remove_bookmark, companion_send, companion_view, run_companion_cycle,
};
use ria_sync::logic::companion::state::{
    Host, LoadInput, companion_load_at, companion_state_unsaved, companion_unload, register_host,
    save, view_is_new,
};
use ria_sync::logic::hlc::Clock;
use ria_sync::logic::scope_state::SyncResult;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The files of each scope: id to payload and whether the file is partial.
type Folder = IndexMap<String, IndexMap<String, (String, bool)>>;

/// The scripted folder, journal and saved state; `fail` lists the host calls that fail next.
#[derive(Default)]
struct World {
    folder: RefCell<Folder>,
    counter: Cell<u32>,
    chain: RefCell<String>,
    fail: RefCell<Vec<String>>,
    journal: RefCell<IndexMap<String, Vec<Value>>>,
    state_json: RefCell<Option<String>>,
    log: RefCell<Vec<Value>>,
    /// The ids that each read of a scope lists as pending (not yet downloaded).
    pending: RefCell<IndexMap<String, Vec<String>>>,
}

impl World {
    fn take(&self, what: &str) -> bool {
        let mut fail = self.fail.borrow_mut();
        let at = fail.iter().position(|f| f == what);
        at.map(|at| fail.remove(at)).is_some()
    }

    fn add(&self, scope: &str, payload: String, partial: bool) -> String {
        self.counter.set(self.counter.get() + 1);
        let id = format!("f{:04}", self.counter.get());
        self.folder
            .borrow_mut()
            .entry(scope.to_string())
            .or_default()
            .insert(id.clone(), (payload, partial));
        id
    }

    fn log(&self, entry: Value) {
        self.log.borrow_mut().push(entry);
    }
}

struct WorldHost(Rc<World>);

impl Host for WorldHost {
    fn chain_status(&self) -> SyncResult<String> {
        let chain = self.0.chain.borrow().clone();
        self.0.log(json!(["chain", chain]));
        if self.0.take("chain") {
            return Err("chain failed".into());
        }
        Ok(chain)
    }

    fn read(&self, scope: &str, known_json: &str) -> SyncResult<String> {
        let w = &self.0;
        w.log(json!(["read", scope, known_json]));
        if w.take("read") {
            return Err("read failed".into());
        }
        let ids: Vec<String> = serde_json::from_str(known_json).map_err(|e| e.to_string())?;
        let (mut files, mut damaged, mut present) = (Vec::new(), Vec::new(), Vec::new());
        let mut folder = w.folder.borrow_mut();
        for (id, (payload, partial)) in folder.entry(scope.to_string()).or_default().iter() {
            present.push(json!(id));
            if ids.contains(id) {
                continue;
            }
            if *partial {
                damaged.push(json!({"id": id, "age": 0}));
            } else {
                files.push(json!({"id": id, "payload": payload}));
            }
        }
        let pending = w.pending.borrow().get(scope).cloned().unwrap_or_default();
        Ok(
            json!({"files": files, "pending": pending, "damaged": damaged, "present": present})
                .to_string(),
        )
    }

    fn write(&self, scope: &str, payload: &str) -> SyncResult<String> {
        let w = &self.0;
        if w.take("write") {
            w.log(json!(["write", scope, null]));
            return Err("disk full".into());
        }
        let id = w.add(scope, payload.to_string(), false);
        w.log(json!(["write", scope, id]));
        if w.take("writeAfter") {
            return Err("write timed out".into());
        }
        Ok(id)
    }

    fn remove(&self, scope: &str, ids_json: &str) -> SyncResult<String> {
        let w = &self.0;
        w.log(json!(["remove", scope, ids_json]));
        if w.take("remove") {
            return Err("remove failed".into());
        }
        let ids: Vec<String> = serde_json::from_str(ids_json).map_err(|e| e.to_string())?;
        let mut folder = w.folder.borrow_mut();
        let files = folder.entry(scope.to_string()).or_default();
        for id in ids {
            files.shift_remove(&id);
        }
        Ok(String::new())
    }

    fn persist_journal(&self, scope: &str, batches_json: &str) -> SyncResult<String> {
        let w = &self.0;
        w.log(json!(["persist", scope, batches_json]));
        if w.take("persist") {
            return Err("journal disk full".into());
        }
        if w.take("persistNo") {
            return Ok("no".into());
        }
        let raw: Vec<String> = serde_json::from_str(batches_json).map_err(|e| e.to_string())?;
        let mut journal = w.journal.borrow_mut();
        let list = journal.entry(scope.to_string()).or_default();
        for text in raw {
            let f: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            if !list.iter().any(|x| x["seq"] == f["seq"]) {
                list.push(f);
            }
        }
        Ok("ok".into())
    }

    fn save_state(&self, state_json: &str, seqs_json: &str) -> SyncResult<String> {
        let w = &self.0;
        w.log(json!(["save", seqs_json]));
        if w.take("save") {
            return Err("state disk full".into());
        }
        if w.take("saveNo") {
            return Ok("no".into());
        }
        *w.state_json.borrow_mut() = Some(state_json.to_string());
        let saved: IndexMap<String, f64> =
            serde_json::from_str(seqs_json).map_err(|e| e.to_string())?;
        w.journal.borrow_mut().retain(|scope, list| {
            let floor = saved.get(scope).copied().unwrap_or(0.0);
            list.retain(|f| f["seq"].as_f64().is_some_and(|s| s > floor));
            !list.is_empty()
        });
        Ok("ok".into())
    }
}

/// `Date.now()` of the original, which `clock-freeze.mjs` fixes to `RIA_NOW_MS`.
fn now_ms() -> f64 {
    if let Some(fixed) = std::env::var("RIA_NOW_MS")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        return fixed;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64)
}

/// A JS number as JSON: integral numbers print without a fraction, as `JSON.stringify` does.
fn number(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

fn arg<T: DeserializeOwned>(v: &Value) -> SyncResult<T> {
    T::deserialize(v).map_err(|e| e.to_string())
}

fn out(v: &impl Serialize) -> SyncResult<Value> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// `peerLog`: a log file of another device, each op stamped at its own wall time.
fn peer_log(device: &str, seq: &Value, at: &Value, ops: &Value) -> Value {
    let ops: Vec<Value> = ops
        .as_array()
        .into_iter()
        .flatten()
        .map(|op| {
            let wall = op[2].as_f64().unwrap_or_default();
            let h = Clock::new(device, None, Some(Rc::new(move || wall))).tick();
            json!({"k": op[0], "h": h, "v": op[1]})
        })
        .collect();
    json!({"v": 1, "kind": "log", "device": device, "seq": seq, "at": at, "ops": ops})
}

fn load_input(w: &World, a: &Value) -> SyncResult<LoadInput> {
    let mut a = a.clone();
    if a["state"] == "$saved" {
        a["state"] = match w.state_json.borrow().as_deref() {
            Some(s) => serde_json::from_str(s).map_err(|e| e.to_string())?,
            None => Value::Null,
        };
    }
    if a["journal"] == "$journal" {
        a["journal"] = out(&*w.journal.borrow())?;
    }
    arg(&a)
}

fn show(w: &World) -> Value {
    let folder: Vec<Value> = w
        .folder
        .borrow()
        .iter()
        .map(|(scope, files)| {
            json!([
                scope,
                files
                    .iter()
                    .map(|(id, (p, partial))| json!([id, p, partial]))
                    .collect::<Vec<_>>()
            ])
        })
        .collect();
    let log = std::mem::take(&mut *w.log.borrow_mut());
    json!({"folder": folder, "journal": *w.journal.borrow(), "state": *w.state_json.borrow(), "log": log})
}

async fn run(w: &Rc<World>, step: &[Value]) -> SyncResult<Value> {
    let none = Value::Null;
    let at = |i: usize| step.get(i).unwrap_or(&none);
    let (a, b, c) = (at(1), at(2), at(3));
    Ok(match text(&step[0]) {
        "host" => {
            register_host(if *a == false {
                None
            } else {
                Some(Rc::new(WorldHost(w.clone())))
            });
            Value::Null
        }
        "chain" => {
            *w.chain.borrow_mut() = text(a).to_string();
            Value::Null
        }
        "fail" => {
            w.fail.borrow_mut().push(text(a).to_string());
            Value::Null
        }
        "pending" => {
            let ids = b
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            w.pending.borrow_mut().insert(text(a).to_string(), ids);
            Value::Null
        }
        "file" => {
            let payload = b.as_str().map_or_else(|| b.to_string(), str::to_string);
            json!(w.add(text(a), payload, *c == true))
        }
        "peer" => json!(w.add(
            text(a),
            peer_log(text(b), c, at(4), at(5)).to_string(),
            false
        )),
        "unlink" => {
            // `scopeOf` makes the scope, as the original does.
            w.folder
                .borrow_mut()
                .entry(text(a).to_string())
                .or_default()
                .shift_remove(text(b));
            Value::Null
        }
        "load" => out(&companion_load_at(&load_input(w, a)?, now_ms())?)?,
        "cycle" => {
            let mut input: CycleInput = arg(a)?;
            // The original reads `Date.now()` without `now`; the hook fixes it to `RIA_NOW_MS`.
            input.now.get_or_insert_with(now_ms);
            out(&run_companion_cycle(&input).await?)?
        }
        "send" => out(&companion_send(&arg::<SendInput>(a)?)?)?,
        "ack" => out(&companion_acknowledge(&arg::<AcknowledgeInput>(a)?)?)?,
        "removalsDone" => out(&companion_removals_done(&arg::<RemovalsDoneInput>(a)?)?)?,
        "addBookmark" => out(&companion_add_bookmark(&arg::<AddBookmarkInput>(a)?)?)?,
        "removeBookmark" => out(&companion_remove_bookmark(&arg::<RemoveBookmarkInput>(a)?)?)?,
        "view" => out(&companion_view()?)?,
        "unload" => json!(companion_unload()),
        "unsaved" => json!(companion_state_unsaved()),
        "save" => json!(save(*a == true)?),
        "viewIsNew" => json!(view_is_new(text(a))),
        "dayOf" => json!(day_of(a.as_f64().unwrap_or(f64::NAN))?),
        "lastVisitOf" => number(last_visit_of(step.get(1))),
        "historyExpired" => json!(history_expired(text(a), b, c.as_f64().unwrap_or(f64::NAN))),
        "isWebUrl" => json!(is_web_url(text(a))),
        "show" => show(w),
        _ => json!({"error": "bad step"}),
    })
}

async fn replay(steps: Vec<Value>) -> Vec<Value> {
    let w = Rc::new(World {
        chain: RefCell::new("ok".into()),
        ..Default::default()
    });
    companion_unload();
    register_host(Some(Rc::new(WorldHost(w.clone()))));
    let mut results = Vec::with_capacity(steps.len());
    for step in steps {
        let result = match step.as_array() {
            Some(step) if step.first().is_some_and(Value::is_string) => {
                run(&w, step).await.unwrap_or_else(|e| json!({"error": e}))
            }
            _ => json!({"error": "bad step"}),
        };
        results.push(result);
    }
    results
}

/// Port of `replayCompanion` (`packages/sync/src/companion-cycle.harness.ts:194`).
pub async fn replay_companion(input: String) -> Result<Vec<Value>, String> {
    let Ok(Value::Array(steps)) = serde_json::from_str::<Value>(&input) else {
        return Ok(vec![json!({"error": "bad script"})]);
    };
    // The companion is not `Send`; the binding's future must be.
    Ok(futures::executor::block_on(replay(steps)))
}
