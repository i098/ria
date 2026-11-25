// Ria original, no Ardia source.

//! Replays the scripts of `extensions.harness.ts` against `ria_sync::logic::extensions`, with
//! the same scripted host.

#![forbid(unsafe_code)]

use indexmap::{IndexMap, IndexSet};
use ria_sync::logic::extensions::{
    self as ext, ExtensionChange, ExtensionIntent, ExtensionsAdapter, ExtensionsHost,
    LocalExtensions,
};
use ria_sync::logic::scope_state::{
    Adapter, ApplyRecords, ExtractCtx, LocalBoxFuture, Op, SyncRecord, SyncResult, empty_scope,
};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

/// A number as `JSON.stringify` writes it: an integral value has no `.0`.
fn num(x: f64) -> Value {
    if x.fract() == 0.0 && x.abs() < 9e15 {
        json!(x as i64)
    } else {
        json!(x)
    }
}

fn strings(v: &Value) -> Vec<String> {
    let items = v.as_array().into_iter().flatten();
    items
        .filter_map(|s| s.as_str().map(str::to_owned))
        .collect()
}

/// `new Map(entries)`: a missing value is `undefined`, which the adapter treats as `null`.
fn entries(v: &Value) -> IndexMap<String, Value> {
    let items = v.as_array().into_iter().flatten();
    items
        .filter_map(|e| {
            Some((
                e.get(0)?.as_str()?.to_owned(),
                e.get(1).cloned().unwrap_or_default(),
            ))
        })
        .collect()
}

/// The scripted host of the harness: it answers from the script and logs each call.
#[derive(Default)]
struct Host {
    local: RefCell<Option<LocalExtensions>>,
    intents: RefCell<IndexMap<String, ExtensionIntent>>,
    answers: RefCell<VecDeque<bool>>,
    calls: RefCell<Vec<Value>>,
}

impl Host {
    fn log(&self, call: Value) {
        self.calls.borrow_mut().push(call);
    }
}

impl ExtensionsHost for Host {
    fn read(&self) -> LocalBoxFuture<'_, SyncResult<Option<LocalExtensions>>> {
        self.log(json!(["read"]));
        let local = self.local.borrow().clone();
        Box::pin(async move { Ok(local) })
    }

    fn change<'a>(&'a self, change: &'a ExtensionChange) -> LocalBoxFuture<'a, SyncResult<bool>> {
        self.log(json!(["change", change]));
        let ok = self.answers.borrow_mut().pop_front().unwrap_or(true);
        Box::pin(async move { Ok(ok) })
    }

    fn intents(&self) -> IndexMap<String, ExtensionIntent> {
        self.log(json!(["intents"]));
        self.intents.borrow().clone()
    }

    fn settle_intents(&self, published: &IndexMap<String, f64>) {
        let published: serde_json::Map<String, Value> = published
            .iter()
            .map(|(id, g)| (id.clone(), num(*g)))
            .collect();
        self.log(json!(["settle", published]));
    }

    fn keep_installed(&self, id: &str) {
        self.log(json!(["keep", id]));
    }
}

/// `heldBy`: answers from the list, then repeats its last answer (`false` for none).
fn held_by(host: &Rc<Host>, held: &Value) -> impl Fn() -> bool + use<> {
    let held: RefCell<VecDeque<bool>> = RefCell::new(
        held.as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_bool)
            .collect(),
    );
    let host = Rc::clone(host);
    move || {
        host.log(json!(["held"]));
        let mut held = held.borrow_mut();
        let next = if held.len() > 1 {
            held.pop_front()
        } else {
            held.front().copied()
        };
        next.unwrap_or(false)
    }
}

fn pure(name: &str, args: &[Value]) -> Option<Value> {
    let text = |i: usize| args.get(i).and_then(Value::as_str).unwrap_or_default();
    let value = args.first().unwrap_or(&Value::Null);
    Some(match name {
        "extensionKey" => json!(ext::extension_key(text(0))),
        "extensionApprovalKey" => json!(ext::extension_approval_key(text(0))),
        "extensionIdOf" => json!(ext::extension_id_of(text(0))),
        "compareVersions" => json!(ext::compare_versions(text(0), text(1))),
        "isRemoval" => json!(ext::is_removal(value)),
        "isExtensionState" => json!(ext::is_extension_state(value)),
        _ => return None,
    })
}

async fn extract(adapter: &ExtensionsAdapter, a: &Value) -> SyncResult<Value> {
    let mut state = empty_scope();
    for (key, value) in a["base"].as_object().into_iter().flatten() {
        state.base.insert(
            key.clone(),
            serde_json::to_string(value).map_err(|e| e.to_string())?,
        );
    }
    for k in strings(&a["known"]) {
        state.records.insert(
            k,
            SyncRecord {
                h: String::new(),
                v: Value::Null,
                o: (String::new(), 0),
            },
        );
    }
    for k in strings(&a["queued"]) {
        state.outbox.push(Op {
            k,
            h: String::new(),
            v: Value::Null,
        });
    }
    let ctx = ExtractCtx::new(&state, 0.0);
    let Some(x) = adapter.extract(&ctx).await? else {
        return Ok(Value::Null);
    };
    let probe = strings(&a["probe"]);
    let ask = |f: Option<&dyn Fn(&str) -> bool>| -> Vec<bool> {
        probe.iter().map(|k| f.is_some_and(|f| f(k))).collect()
    };
    let values: Vec<Value> = x.values.iter().map(|(k, v)| json!([k, v])).collect();
    let (ignore, republish) = (ask(x.ignore.as_deref()), ask(x.republish.as_deref()));
    let result = json!({ "values": values, "ignore": ignore, "republish": republish });
    if let (Some(keys), Some(published)) = (a.get("publish").filter(|k| k.is_array()), x.published)
    {
        published(&strings(keys).into_iter().collect());
    }
    Ok(result)
}

async fn replay(input: &str) -> SyncResult<Vec<Value>> {
    let steps = match serde_json::from_str::<Value>(input) {
        Ok(Value::Array(steps)) if steps.iter().all(|s| s.get(0).is_some_and(Value::is_string)) => {
            steps
        }
        _ => return Ok(vec![json!("not a script")]),
    };
    let host = Rc::new(Host::default());
    let mut adapter = ext::extensions_adapter(host.clone());
    let mut out = Vec::new();
    for step in &steps {
        let step = step.as_array().map(Vec::as_slice).unwrap_or_default();
        let name = step.first().and_then(Value::as_str).unwrap_or_default();
        if let Some(value) = pure(name, step.get(1..).unwrap_or_default()) {
            out.push(value);
            continue;
        }
        let a = step.get(1).unwrap_or(&Value::Null);
        let result = match name {
            "host" => {
                if let Some(local) = a.get("local") {
                    *host.local.borrow_mut() =
                        serde_json::from_value(local.clone()).map_err(|e| e.to_string())?;
                }
                if let Some(intents) = a.get("intents").filter(|i| i.is_object()) {
                    *host.intents.borrow_mut() =
                        serde_json::from_value(intents.clone()).map_err(|e| e.to_string())?;
                }
                if let Some(answers) = a.get("answers").filter(|v| v.is_array()) {
                    *host.answers.borrow_mut() = answers
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_bool)
                        .collect();
                }
                out.push(Value::Null);
                continue;
            }
            "adapter" => {
                adapter = ext::extensions_adapter(host.clone());
                out.push(Value::Null);
                continue;
            }
            "extract" => extract(&adapter, a).await?,
            "apply" => {
                let held = held_by(&host, &a["held"]);
                let state = empty_scope();
                let records = ApplyRecords {
                    state: &state,
                    now: 0.0,
                    horizon: 0.0,
                    joined: a["joined"].as_bool().unwrap_or(true),
                    held: &held,
                    landed: &|_: &str| {},
                };
                let changed: IndexSet<String> = strings(&a["changed"]).into_iter().collect();
                let ok = adapter
                    .apply(&entries(&a["visible"]), &changed, &records)
                    .await?;
                json!(ok)
            }
            "reconcile" => {
                let held = a
                    .get("held")
                    .filter(|h| h.is_array())
                    .map(|h| held_by(&host, h));
                let held = held.as_ref().map(|h| h as &dyn Fn() -> bool);
                adapter
                    .reconcile(
                        &entries(&a["visible"]),
                        a["now"].as_f64().unwrap_or(0.0),
                        held,
                    )
                    .await?;
                Value::Null
            }
            _ => {
                out.push(json!(["unknown step", name]));
                continue;
            }
        };
        out.push(json!({ "result": result, "calls": host.calls.take() }));
    }
    Ok(out)
}

/// Port of `replayExtensions` (`packages/sync/src/extensions.harness.ts:34`).
pub async fn replay_extensions(input: String) -> Result<Vec<serde_json::Value>, String> {
    // The adapter's futures hold `Rc`s, so they run here and napi's future stays `Send`.
    pollster::block_on(replay(&input))
}
