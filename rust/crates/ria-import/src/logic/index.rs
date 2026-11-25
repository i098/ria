// Ported from ardia packages/import/src/index.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The calls of `index.ts` (lines 236 to 329): the job loop, the options that reach the native
//! module, and the parse of its JSON text. The native module is the trait `ImportNative`.
//!
//! Changes from Ardia:
//! - `requireNativeModule("ArdiaImport")` has no Rust counterpart. `ImportNative` is the same
//!   surface, and `ImportClient` holds the one value that implements it.
//! - Calls are synchronous. A TypeScript promise settles later; here the native call returns when
//!   it is done, and events and aborts that come during it reach the callbacks in the same order.
//! - `jobSeq` was a module variable. It belongs to the `ImportClient` here, so each client counts
//!   job ids from `import-1`.
//! - `AbortSignal` is a small type with the one thing `job` uses: `aborted` and the `abort`
//!   listeners.
//! - A result is `Ok(None)` where TypeScript returns `undefined` (the native module gave `null`).
//!   Text that is JSON `null` is an error for a typed result here; TypeScript returns `null`.
//! - Types are checked on read: TypeScript casts the parsed JSON, serde rejects JSON that does not
//!   fit the type. A JSON error has the serde message, not the V8 one.
//! - A history chunk with bad JSON is dropped here; in TypeScript the throw leaves the listener.

use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use ria_core::errors::{ArdiaError, ardia_error};
use serde::de::{DeserializeOwned, IgnoredAny};

use super::types::*;

/// What a native call gives back: JSON text, or `None` for `null`.
pub type NativeText = Result<Option<String>, Box<dyn Error + Send + Sync>>;

/// The listener of native events; see `ImportNative::add_listener`.
pub type EventListener = Box<dyn FnMut(ImportEvent) + Send>;

/// The handle of one listener (`EventSubscription`).
pub trait Subscription {
    fn remove(self: Box<Self>);
}

/// The native module `ArdiaImport` (`index.ts:236`).
pub trait ImportNative: Send + Sync + 'static {
    fn add_listener(&self, listener: EventListener) -> Box<dyn Subscription>;
    fn list_browsers(&self) -> NativeText;
    fn import_data(
        &self,
        job_id: &str,
        browser_id: &str,
        profile_id: &str,
        kinds: &[ImportKind],
        options: &NativeImportOptions,
    ) -> NativeText;
    fn cancel_import(&self, job_id: &str);
    fn unlock_browser(&self, browser_id: &str, primary_password: Option<&str>) -> NativeText;
    fn is_browser_unlocked(&self, browser_id: &str) -> bool;
    fn forget_unlocked_keys(&self);
    fn write_imported(&self, token: &str, profile: &str, kinds: &[WriteKind]) -> NativeText;
    fn discard_imported(&self, token: &str);
    fn list_extensions(&self, browser_id: &str, profile_id: &str) -> NativeText;
    fn import_safari_export(&self, job_id: &str, path: &str) -> NativeText;
    fn safari_has_full_disk_access(&self) -> bool;
    fn open_full_disk_access_settings(&self) -> NativeText;
    fn import_safari_direct(&self, job_id: &str) -> NativeText;
    fn import_bookmarks_html(&self, path: &str) -> NativeText;
    fn import_passwords_csv(&self, path: &str) -> NativeText;
    fn choose_import_file(
        &self,
        kind: ImportFileKind,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>>;
    fn read_dia_tabs(&self) -> NativeText;
}

type AbortListener = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
struct SignalState {
    aborted: bool,
    next_id: u64,
    listeners: Vec<(u64, AbortListener)>,
}

/// The part of `AbortSignal` that `job` uses.
#[derive(Default)]
pub struct AbortSignal {
    state: Mutex<SignalState>,
}

impl AbortSignal {
    fn lock(&self) -> std::sync::MutexGuard<'_, SignalState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn aborted(&self) -> bool {
        self.lock().aborted
    }

    /// Marks the signal aborted and runs each listener once; a second call does nothing.
    pub fn abort(&self) {
        let listeners: Vec<AbortListener> = {
            let mut state = self.lock();
            if state.aborted {
                return;
            }
            state.aborted = true;
            state.listeners.iter().map(|(_, l)| Arc::clone(l)).collect()
        };
        for listener in listeners {
            listener();
        }
    }

    /// How many listeners are still registered; a finished job leaves none.
    pub fn listener_count(&self) -> usize {
        self.lock().listeners.len()
    }

    fn add_listener(&self, listener: AbortListener) -> u64 {
        let mut state = self.lock();
        state.next_id += 1;
        let id = state.next_id;
        state.listeners.push((id, listener));
        id
    }

    fn remove_listener(&self, id: u64) {
        self.lock().listeners.retain(|(i, _)| *i != id);
    }
}

/// `ImportOptions` (`index.ts:206`).
#[derive(Default)]
pub struct ImportOptions {
    pub history_limit: Option<f64>,
    pub history_since: Option<f64>,
    pub space_ids: Option<Vec<String>>,
    pub on_progress: Option<Box<dyn FnMut(ImportProgress) + Send>>,
    pub on_history_chunk: Option<Box<dyn FnMut(Vec<ImportedHistoryEntry>) + Send>>,
    pub chunk_size: Option<f64>,
    pub signal: Option<Arc<AbortSignal>>,
}

/// `call` (`index.ts:259`): `None` when the native module gave `null`; its error or a parse
/// error becomes an `ArdiaError`.
fn call<T: DeserializeOwned>(result: NativeText) -> Result<Option<T>, ArdiaError> {
    match result {
        Ok(None) => Ok(None),
        Ok(Some(json)) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| ardia_error(&e)),
        Err(e) => Err(ardia_error(e.as_ref())),
    }
}

fn call_void(result: NativeText) -> Result<(), ArdiaError> {
    call::<IgnoredAny>(result).map(|_| ())
}

/// The calls of `index.ts` over one native module.
pub struct ImportClient<N: ImportNative> {
    native: Arc<N>,
    job_seq: AtomicU64,
}

impl<N: ImportNative> ImportClient<N> {
    pub fn new(native: N) -> Self {
        Self {
            native: Arc::new(native),
            job_seq: AtomicU64::new(0),
        }
    }

    /// `job` (`index.ts:268`).
    fn job<T: DeserializeOwned>(
        &self,
        options: ImportOptions,
        start: impl FnOnce(&str) -> NativeText,
    ) -> Result<Option<T>, ArdiaError> {
        let job_id = format!(
            "import-{}",
            self.job_seq.fetch_add(1, Ordering::Relaxed) + 1
        );
        if options.signal.as_ref().is_some_and(|s| s.aborted()) {
            return Err(ArdiaError::new("cancelled", "Import cancelled"));
        }
        let ImportOptions {
            mut on_progress,
            mut on_history_chunk,
            signal,
            ..
        } = options;
        let sub = self.native.add_listener({
            let job_id = job_id.clone();
            Box::new(move |event| match event {
                ImportEvent::History {
                    job_id: id,
                    entries,
                } if id == job_id => {
                    if let (Some(f), Ok(entries)) =
                        (on_history_chunk.as_mut(), serde_json::from_str(&entries))
                    {
                        f(entries);
                    }
                }
                ImportEvent::Progress {
                    job_id: id,
                    progress,
                } if id == job_id => {
                    if let Some(f) = on_progress.as_mut() {
                        f(progress);
                    }
                }
                _ => {}
            })
        });
        let abort = signal.as_ref().map(|s| {
            let native = Arc::clone(&self.native);
            let job_id = job_id.clone();
            s.add_listener(Arc::new(move || native.cancel_import(&job_id)))
        });
        let result = call(start(&job_id));
        if let (Some(s), Some(id)) = (&signal, abort) {
            s.remove_listener(id);
        }
        sub.remove();
        result
    }

    pub fn list_browsers(&self) -> Result<Option<Vec<BrowserSource>>, ArdiaError> {
        call(self.native.list_browsers())
    }

    pub fn import_data(
        &self,
        browser_id: &str,
        profile_id: &str,
        kinds: &[ImportKind],
        options: ImportOptions,
    ) -> Result<Option<ImportResult>, ArdiaError> {
        let native = NativeImportOptions {
            history_limit: options.history_limit,
            history_since: options.history_since,
            space_ids: options.space_ids.clone(),
            chunk_size: options.chunk_size,
            stream_history: options.on_history_chunk.is_some(),
        };
        self.job(options, |job_id| {
            self.native
                .import_data(job_id, browser_id, profile_id, kinds, &native)
        })
    }

    pub fn unlock_browser(
        &self,
        browser_id: &str,
        primary_password: Option<&str>,
    ) -> Result<(), ArdiaError> {
        call_void(self.native.unlock_browser(browser_id, primary_password))
    }

    pub fn is_browser_unlocked(&self, browser_id: &str) -> bool {
        self.native.is_browser_unlocked(browser_id)
    }

    pub fn forget_unlocked_keys(&self) {
        self.native.forget_unlocked_keys()
    }

    pub fn write_imported(
        &self,
        token: &str,
        profile: &str,
        kinds: &[WriteKind],
    ) -> Result<Option<WrittenCounts>, ArdiaError> {
        call(self.native.write_imported(token, profile, kinds))
    }

    pub fn discard_imported(&self, token: &str) {
        self.native.discard_imported(token)
    }

    pub fn list_extensions(
        &self,
        browser_id: &str,
        profile_id: &str,
    ) -> Result<Option<Vec<ImportedExtension>>, ArdiaError> {
        call(self.native.list_extensions(browser_id, profile_id))
    }

    pub fn import_safari_export(
        &self,
        path: &str,
        signal: Option<Arc<AbortSignal>>,
    ) -> Result<Option<SafariExport>, ArdiaError> {
        let options = ImportOptions {
            signal,
            ..Default::default()
        };
        self.job(options, |job_id| {
            self.native.import_safari_export(job_id, path)
        })
    }

    pub fn safari_has_full_disk_access(&self) -> bool {
        self.native.safari_has_full_disk_access()
    }

    pub fn open_full_disk_access_settings(&self) -> Result<(), ArdiaError> {
        call_void(self.native.open_full_disk_access_settings())
    }

    pub fn import_safari_direct(
        &self,
        signal: Option<Arc<AbortSignal>>,
    ) -> Result<Option<SafariExport>, ArdiaError> {
        let options = ImportOptions {
            signal,
            ..Default::default()
        };
        self.job(options, |job_id| self.native.import_safari_direct(job_id))
    }

    pub fn import_bookmarks_html(&self, path: &str) -> Result<Option<BookmarkNode>, ArdiaError> {
        call(self.native.import_bookmarks_html(path))
    }

    pub fn import_passwords_csv(&self, path: &str) -> Result<Option<Vec<Credential>>, ArdiaError> {
        call(self.native.import_passwords_csv(path))
    }

    /// Not wrapped by `call`: the text of the native module is the path, and its error is raw.
    pub fn choose_import_file(
        &self,
        kind: ImportFileKind,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        self.native.choose_import_file(kind)
    }

    pub fn read_dia_tabs(&self) -> Result<Option<DiaTabsResult>, ArdiaError> {
        call(self.native.read_dia_tabs())
    }
}
