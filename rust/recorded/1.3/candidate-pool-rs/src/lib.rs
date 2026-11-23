//! JS bindings for `packages/core/src/candidatePool.harness.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[allow(dead_code)]
fn run<T>(f: impl FnOnce() -> Result<T, String>) -> napi::Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `poolGrid`.
#[napi(js_name = "poolGrid")]
pub fn pool_grid(
    histories: String,
    bookmark_lists: String,
    keys: Vec<String>,
) -> napi::Result<String> {
    run(move || {
        let histories = crate::js::from_json(&histories)?;
        let bookmark_lists = crate::js::from_json(&bookmark_lists)?;
        let out = port::pool_grid(histories, bookmark_lists, keys)?;
        crate::js::to_json(&out)
    })
}

/// `preparedPool`.
#[napi(js_name = "preparedPool")]
pub fn prepared_pool(source: String, _deadline: f64, keys: Vec<String>) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::prepared_pool(source, _deadline, keys)?;
        crate::js::to_json(&out)
    })
}

/// `tabsOverView`.
#[napi(js_name = "tabsOverView")]
pub fn tabs_over_view(source: String, current_tab_id: Option<String>) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::tabs_over_view(source, current_tab_id)?;
        crate::js::to_json(&out)
    })
}

/// `savedVisits`.
#[napi(js_name = "savedVisits")]
pub fn saved_visits(history: String, urls: Vec<String>) -> napi::Result<String> {
    run(move || {
        let history = crate::js::from_json(&history)?;
        let out = port::saved_visits(history, urls)?;
        crate::js::to_json(&out)
    })
}

/// `savedBookmarks`.
#[napi(js_name = "savedBookmarks")]
pub fn saved_bookmarks(bookmarks: String, urls: Vec<String>) -> napi::Result<String> {
    run(move || {
        let bookmarks = crate::js::from_json(&bookmarks)?;
        let out = port::saved_bookmarks(bookmarks, urls)?;
        crate::js::to_json(&out)
    })
}

// ---- the binary wire ------------------------------------------------------------------------

type WireBuffers = (
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint32Array,
);

thread_local! {
    static WIRE: std::cell::RefCell<Option<WireBuffers>> = const { std::cell::RefCell::new(None) };
    static PENDING: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static SCRATCH: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers the argument, result and argument-length buffers `index.cjs` uses.
#[napi(js_name = "rustAnythingWireInit")]
pub fn rust_anything_wire_init(
    input: napi::bindgen_prelude::Uint8Array,
    output: napi::bindgen_prelude::Uint8Array,
    len: napi::bindgen_prelude::Uint32Array,
) {
    WIRE.with(|w| *w.borrow_mut() = Some((input, output, len)));
}

/// Copies a result that did not fit into the (now larger) result buffer.
#[napi(js_name = "rustAnythingWireFlush")]
pub fn rust_anything_wire_flush() -> f64 {
    let bytes = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    WIRE.with(|w| {
        let mut w = w.borrow_mut();
        let Some((_, output, _)) = w.as_mut() else {
            return -1.0;
        };
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        n as f64
    })
}

/// Reads the argument bytes (`wLen[0]` of them), runs `f`, and writes its result; a negative
/// return is the size of a result that did not fit, kept for `rustAnythingWireFlush`.
fn wire_io(
    f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>,
) -> napi::Result<f64> {
    WIRE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some((input, output, len)) = guard.as_mut() else {
            return Err(napi::Error::from_reason(
                "rust-anything: the wire buffers are not registered",
            ));
        };
        let len = (len.first().copied().unwrap_or(0) as usize).min(input.len());
        let mut w = wire::Writer(SCRATCH.with(|s| std::mem::take(&mut *s.borrow_mut())));
        w.0.clear();
        let result = run(|| f(&mut wire::Reader::new(&input[..len]), &mut w));
        let keep = |w: wire::Writer| SCRATCH.with(|s| *s.borrow_mut() = w.0);
        match result {
            // A result of another shape than the TS type: JS runs the call through JSON.
            Err(e) if e.reason.starts_with(wire::MISMATCH) => {
                keep(w);
                return Ok(f64::NAN);
            }
            Err(e) => {
                keep(w);
                let mut m = wire::Writer::default();
                m.string16(&e.reason);
                // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
                let out = unsafe { output.as_mut() };
                if m.0.len() > out.len() {
                    return Err(e);
                }
                out[..m.0.len()].copy_from_slice(&m.0);
                return Ok(f64::INFINITY);
            }
            Ok(()) => {}
        }
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = w.0.len();
        if n > out.len() {
            PENDING.with(|p| *p.borrow_mut() = w.0);
            return Ok(-(n as f64));
        }
        out[..n].copy_from_slice(&w.0);
        keep(w);
        Ok(n as f64)
    })
}

static S_POOL_GRID_0: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);
static S_POOL_GRID_1: wire::Schema = wire::Schema::Dyn;
static S_POOL_GRID_OUT: wire::Schema = wire::Schema::Dyn;

/// `poolGrid` over the binary wire.
#[napi(js_name = "poolGridWire")]
pub fn pool_grid_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let histories: Vec<Value> = r.list(|r| wire::read_value(r, S_POOL_GRID_0.inner()))?;
        let bookmark_lists: Value = wire::read_value(r, &S_POOL_GRID_1)?;
        let keys: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::pool_grid(histories, bookmark_lists, keys)?;
        wire::write_value(w, out, &S_POOL_GRID_OUT)?;
        Ok(())
    })
}

static S_PREPARED_POOL_0: wire::Schema = wire::Schema::Dyn;
static S_PREPARED_POOL_OUT: wire::Schema = wire::Schema::Dyn;

/// `preparedPool` over the binary wire.
#[napi(js_name = "preparedPoolWire")]
pub fn prepared_pool_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: Value = wire::read_value(r, &S_PREPARED_POOL_0)?;
        let _deadline: f64 = r.f64()?;
        let keys: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::prepared_pool(source, _deadline, keys)?;
        wire::write_value(w, out, &S_PREPARED_POOL_OUT)?;
        Ok(())
    })
}

static S_TABS_OVER_VIEW_0: wire::Schema = wire::Schema::Dyn;
static S_TABS_OVER_VIEW_OUT: wire::Schema = wire::Schema::Dyn;

/// `tabsOverView` over the binary wire.
#[napi(js_name = "tabsOverViewWire")]
pub fn tabs_over_view_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: Value = wire::read_value(r, &S_TABS_OVER_VIEW_0)?;
        let current_tab_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::tabs_over_view(source, current_tab_id)?;
        wire::write_value(w, out, &S_TABS_OVER_VIEW_OUT)?;
        Ok(())
    })
}

static S_SAVED_VISITS_0: wire::Schema = wire::Schema::Dyn;
static S_SAVED_VISITS_OUT: wire::Schema = wire::Schema::Dyn;

/// `savedVisits` over the binary wire.
#[napi(js_name = "savedVisitsWire")]
pub fn saved_visits_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let history: Value = wire::read_value(r, &S_SAVED_VISITS_0)?;
        let urls: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::saved_visits(history, urls)?;
        wire::write_value(w, out, &S_SAVED_VISITS_OUT)?;
        Ok(())
    })
}

static S_SAVED_BOOKMARKS_0: wire::Schema = wire::Schema::Dyn;
static S_SAVED_BOOKMARKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `savedBookmarks` over the binary wire.
#[napi(js_name = "savedBookmarksWire")]
pub fn saved_bookmarks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let bookmarks: Value = wire::read_value(r, &S_SAVED_BOOKMARKS_0)?;
        let urls: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::saved_bookmarks(bookmarks, urls)?;
        wire::write_value(w, out, &S_SAVED_BOOKMARKS_OUT)?;
        Ok(())
    })
}
