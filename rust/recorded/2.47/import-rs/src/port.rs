// Ria original, no Ardia source.

//! The Rust side of `import.harness.ts`: it reads the same steps, drives `ria_import` over a fake
//! native module that follows the same script, and writes the same trace.

#![forbid(unsafe_code)]

use std::error::Error;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ria_core::errors::ArdiaError;
use ria_import::logic::*;
use serde::Serialize;
use serde_json::{Value, json};

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Default)]
struct Shared {
    listeners: Mutex<Vec<(u64, EventListener)>>,
    next_listener: Mutex<u64>,
    calls: Mutex<Vec<Value>>,
    script: Mutex<Value>,
    signal: Mutex<Option<Arc<AbortSignal>>>,
}

type Failure = Box<dyn Error + Send + Sync>;

impl Shared {
    fn log(&self, name: &str, args: Vec<Value>) {
        let mut call = vec![json!(name)];
        call.extend(args);
        lock(&self.calls).push(Value::Array(call));
    }

    /// `answer` of the harness: run the actions of the script, then give its reply.
    fn answer(&self, job_id: Option<&str>) -> Result<Value, Failure> {
        let script = lock(&self.script).clone();
        for action in script["actions"].as_array().into_iter().flatten() {
            if action.get("abort").is_some() {
                let signal = lock(&self.signal).clone();
                if let Some(signal) = signal {
                    signal.abort();
                }
            } else if let Some(Value::Object(event)) = action.get("emit") {
                let mut event = event.clone();
                if event.get("jobId") == Some(&json!("$job")) {
                    event.insert("jobId".into(), json!(job_id));
                }
                if let Ok(event) = serde_json::from_value::<ImportEvent>(Value::Object(event)) {
                    for (_, listener) in lock(&self.listeners).iter_mut() {
                        listener(event.clone());
                    }
                }
            }
        }
        let reply = &script["reply"];
        if let Some(error) = reply.get("error") {
            let message = error["message"].as_str().unwrap_or_default();
            return Err(match error["code"].as_str() {
                Some(code) => Box::new(ArdiaError::new(code, message)),
                None => Box::new(std::io::Error::other(message)),
            });
        }
        Ok(reply
            .get("text")
            .or_else(|| reply.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    fn text(&self, name: &str, args: Vec<Value>, job_id: Option<&str>) -> NativeText {
        self.log(name, args);
        Ok(self.answer(job_id)?.as_str().map(str::to_owned))
    }

    fn flag(&self, name: &str, args: Vec<Value>) -> bool {
        self.log(name, args);
        self.answer(None)
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or_default()
    }
}

struct Remove(Arc<Shared>, u64);

impl Subscription for Remove {
    fn remove(self: Box<Self>) {
        lock(&self.0.listeners).retain(|(id, _)| *id != self.1);
    }
}

struct Fake(Arc<Shared>);

fn value<T: Serialize + ?Sized>(v: &T) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

impl ImportNative for Fake {
    fn add_listener(&self, listener: EventListener) -> Box<dyn Subscription> {
        let mut next = lock(&self.0.next_listener);
        *next += 1;
        lock(&self.0.listeners).push((*next, listener));
        Box::new(Remove(Arc::clone(&self.0), *next))
    }
    fn list_browsers(&self) -> NativeText {
        self.0.text("listBrowsers", vec![], None)
    }
    fn import_data(
        &self,
        job_id: &str,
        browser_id: &str,
        profile_id: &str,
        kinds: &[ImportKind],
        options: &NativeImportOptions,
    ) -> NativeText {
        let args = vec![
            json!(job_id),
            json!(browser_id),
            json!(profile_id),
            value(kinds),
            value(options),
        ];
        self.0.text("importData", args, Some(job_id))
    }
    fn cancel_import(&self, job_id: &str) {
        self.0.log("cancelImport", vec![json!(job_id)]);
    }
    fn unlock_browser(&self, browser_id: &str, primary_password: Option<&str>) -> NativeText {
        self.0.text(
            "unlockBrowser",
            vec![json!(browser_id), json!(primary_password)],
            None,
        )
    }
    fn is_browser_unlocked(&self, browser_id: &str) -> bool {
        self.0.flag("isBrowserUnlocked", vec![json!(browser_id)])
    }
    fn forget_unlocked_keys(&self) {
        self.0.flag("forgetUnlockedKeys", vec![]);
    }
    fn write_imported(&self, token: &str, profile: &str, kinds: &[WriteKind]) -> NativeText {
        self.0.text(
            "writeImported",
            vec![json!(token), json!(profile), value(kinds)],
            None,
        )
    }
    fn discard_imported(&self, token: &str) {
        self.0.flag("discardImported", vec![json!(token)]);
    }
    fn list_extensions(&self, browser_id: &str, profile_id: &str) -> NativeText {
        self.0.text(
            "listExtensions",
            vec![json!(browser_id), json!(profile_id)],
            None,
        )
    }
    fn import_safari_export(&self, job_id: &str, path: &str) -> NativeText {
        self.0.text(
            "importSafariExport",
            vec![json!(job_id), json!(path)],
            Some(job_id),
        )
    }
    fn safari_has_full_disk_access(&self) -> bool {
        self.0.flag("safariHasFullDiskAccess", vec![])
    }
    fn open_full_disk_access_settings(&self) -> NativeText {
        self.0.text("openFullDiskAccessSettings", vec![], None)
    }
    fn import_safari_direct(&self, job_id: &str) -> NativeText {
        self.0
            .text("importSafariDirect", vec![json!(job_id)], Some(job_id))
    }
    fn import_bookmarks_html(&self, path: &str) -> NativeText {
        self.0.text("importBookmarksHTML", vec![json!(path)], None)
    }
    fn import_passwords_csv(&self, path: &str) -> NativeText {
        self.0.text("importPasswordsCSV", vec![json!(path)], None)
    }
    fn choose_import_file(&self, kind: ImportFileKind) -> Result<Option<String>, Failure> {
        self.0.log("chooseImportFile", vec![value(&kind)]);
        Ok(self.0.answer(None)?.as_str().map(str::to_owned))
    }
    fn read_dia_tabs(&self) -> NativeText {
        self.0.text("readDiaTabs", vec![], None)
    }
}

fn error_json(name: &str, code: Option<&str>, message: &str) -> Value {
    json!({ "name": name, "code": code, "message": message })
}

fn ardia_json(e: &ArdiaError) -> Value {
    error_json("ArdiaError", Some(&e.code), &e.message)
}

fn parse<T: serde::de::DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).expect("the harness sends well-formed arguments")
}

fn signal_for(shared: &Shared, mode: &Value) -> Option<Arc<AbortSignal>> {
    let mode = mode.as_str().unwrap_or("none");
    if mode == "none" {
        return None;
    }
    let signal = Arc::new(AbortSignal::default());
    if mode == "aborted" {
        signal.abort();
    }
    *lock(&shared.signal) = Some(Arc::clone(&signal));
    Some(signal)
}

fn out<T: Serialize>(r: Result<Option<T>, ArdiaError>) -> Result<Value, Value> {
    r.map(|v| value(&v)).map_err(|e| ardia_json(&e))
}

fn unit(r: Result<(), ArdiaError>) -> Result<Value, Value> {
    r.map(|()| Value::Null).map_err(|e| ardia_json(&e))
}

struct Collected {
    progress: Arc<Mutex<Vec<Value>>>,
    chunks: Arc<Mutex<Vec<Value>>>,
}

fn options_for(shared: &Shared, o: &Value, seen: &Collected) -> ImportOptions {
    let progress = Arc::clone(&seen.progress);
    let chunks = Arc::clone(&seen.chunks);
    ImportOptions {
        history_limit: o["historyLimit"].as_f64(),
        history_since: o["historySince"].as_f64(),
        space_ids: o.get("spaceIds").map(parse),
        chunk_size: o["chunkSize"].as_f64(),
        on_progress: (o["onProgress"] == json!(true))
            .then(|| Box::new(move |p| lock(&progress).push(value(&p))) as _),
        on_history_chunk: (o["onHistoryChunk"] == json!(true)).then(|| {
            Box::new(move |c: Vec<ImportedHistoryEntry>| lock(&chunks).push(value(&c))) as _
        }),
        signal: signal_for(shared, &o["signal"]),
    }
}

fn run(
    client: &ImportClient<Fake>,
    shared: &Shared,
    op: &str,
    a: &[Value],
    seen: &Collected,
) -> Result<Value, Value> {
    match op {
        "listBrowsers" => out(client.list_browsers()),
        "importData" => out(client.import_data(
            a[0].as_str().unwrap_or_default(),
            a[1].as_str().unwrap_or_default(),
            &parse::<Vec<ImportKind>>(&a[2]),
            options_for(shared, &a[3], seen),
        )),
        "unlockBrowser" => {
            unit(client.unlock_browser(a[0].as_str().unwrap_or_default(), a[1].as_str()))
        }
        "isBrowserUnlocked" => Ok(json!(
            client.is_browser_unlocked(a[0].as_str().unwrap_or_default())
        )),
        "forgetUnlockedKeys" => {
            client.forget_unlocked_keys();
            Ok(Value::Null)
        }
        "writeImported" => out(client.write_imported(
            a[0].as_str().unwrap_or_default(),
            a[1].as_str().unwrap_or_default(),
            &parse::<Vec<WriteKind>>(&a[2]),
        )),
        "discardImported" => {
            client.discard_imported(a[0].as_str().unwrap_or_default());
            Ok(Value::Null)
        }
        "listExtensions" => out(client.list_extensions(
            a[0].as_str().unwrap_or_default(),
            a[1].as_str().unwrap_or_default(),
        )),
        "importSafariExport" => out(client
            .import_safari_export(a[0].as_str().unwrap_or_default(), signal_for(shared, &a[1]))),
        "safariHasFullDiskAccess" => Ok(json!(client.safari_has_full_disk_access())),
        "openFullDiskAccessSettings" => unit(client.open_full_disk_access_settings()),
        "importSafariDirect" => out(client.import_safari_direct(signal_for(shared, &a[0]))),
        "importBookmarksHTML" => {
            out(client.import_bookmarks_html(a[0].as_str().unwrap_or_default()))
        }
        "importPasswordsCSV" => out(client.import_passwords_csv(a[0].as_str().unwrap_or_default())),
        "chooseImportFile" => client
            .choose_import_file(parse(&a[0]))
            .map(|v| json!(v))
            .map_err(|e| match e.downcast_ref::<ArdiaError>() {
                Some(e) => ardia_json(e),
                None => error_json("Error", None, &e.to_string()),
            }),
        "readDiaTabs" => out(client.read_dia_tabs()),
        _ => Err(error_json(
            "TypeError",
            None,
            &format!("unknown export {op}"),
        )),
    }
}

/// Port of `replayImport` (`packages/import/src/import.harness.ts:22`).
pub async fn replay_import(input: String) -> Result<Vec<serde_json::Value>, String> {
    let steps: Vec<Vec<Value>> =
        serde_json::from_str(&input).map_err(|e| format!("TypeError: {e}"))?;
    let shared = Arc::new(Shared::default());
    let client = ImportClient::new(Fake(Arc::clone(&shared)));
    let mut trace = Vec::with_capacity(steps.len());
    for mut step in steps {
        let script = step.pop().unwrap_or(Value::Null);
        let args = step.split_off(1);
        let op = step[0].as_str().unwrap_or_default();
        *lock(&shared.script) = script;
        lock(&shared.calls).clear();
        *lock(&shared.signal) = None;
        let seen = Collected {
            progress: Arc::default(),
            chunks: Arc::default(),
        };
        let (result, error) = match run(&client, &shared, op, &args, &seen) {
            Ok(v) => (v, Value::Null),
            Err(e) => (Value::Null, e),
        };
        let abort_listeners = lock(&shared.signal)
            .as_ref()
            .map_or(0, |s| s.listener_count());
        trace.push(json!({
            "result": result,
            "error": error,
            "calls": *lock(&shared.calls),
            "progress": *lock(&seen.progress),
            "chunks": *lock(&seen.chunks),
            "listeners": lock(&shared.listeners).len(),
            "abortListeners": abort_listeners,
        }));
    }
    Ok(trace)
}
