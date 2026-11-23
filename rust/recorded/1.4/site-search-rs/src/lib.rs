//! JS bindings for `packages/core/src/siteSearch.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `providerScope`.
#[napi(js_name = "providerScope")]
pub fn provider_scope(text: String, providers: String) -> napi::Result<String> {
    run(move || {
        let providers = crate::js::from_json(&providers)?;
        let out = port::provider_scope(text, providers)?;
        crate::js::to_json(&out)
    })
}

/// `findScope`.
#[napi(js_name = "findScope")]
pub fn find_scope(text: String, arg1: String) -> napi::Result<String> {
    run(move || {
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::find_scope(text, arg1)?;
        crate::js::to_json(&out)
    })
}

/// `scopedSearchUrl`.
#[napi(js_name = "scopedSearchUrl")]
pub fn scoped_search_url(
    scope: String,
    query: String,
    default_engine: String,
) -> napi::Result<String> {
    run(move || {
        let scope = crate::js::from_json(&scope)?;
        let default_engine = crate::js::from_json(&default_engine)?;
        let out = port::scoped_search_url(scope, query, default_engine)?;
        Ok(out)
    })
}

/// `offeredSiteSearch`.
#[napi(js_name = "offeredSiteSearch")]
pub fn offered_site_search(
    typed: String,
    completion: String,
    sources: String,
) -> napi::Result<String> {
    run(move || {
        let sources = crate::js::from_json(&sources)?;
        let out = port::offered_site_search(typed, completion, sources)?;
        crate::js::to_json(&out)
    })
}

/// `keywordBySpace`.
#[napi(js_name = "keywordBySpace")]
pub fn keyword_by_space(typed: String, next: String) -> napi::Result<String> {
    run(move || {
        let out = port::keyword_by_space(typed, next)?;
        crate::js::to_json(&out)
    })
}

/// `asksForBackspace`.
#[napi(js_name = "asksForBackspace")]
pub fn asks_for_backspace(scope: String) -> napi::Result<bool> {
    run(move || {
        let scope = crate::js::from_json(&scope)?;
        let out = port::asks_for_backspace(scope)?;
        Ok(out)
    })
}

/// `siteSearchKey`.
#[napi(js_name = "siteSearchKey")]
pub fn site_search_key(key: String, modifiers: String, state: String) -> napi::Result<String> {
    run(move || {
        let modifiers = crate::js::from_json(&modifiers)?;
        let state = crate::js::from_json(&state)?;
        let out = port::site_search_key(key, modifiers, state)?;
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

static S_PROVIDER_SCOPE_1: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_SCOPE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("host", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("engineId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("providerId", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `providerScope` over the binary wire.
#[napi(js_name = "providerScopeWire")]
pub fn provider_scope_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let providers: Value = wire::read_value(r, &S_PROVIDER_SCOPE_1)?;
        let out = &port::provider_scope(text, providers)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PROVIDER_SCOPE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_FIND_SCOPE_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_SCOPE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("host", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("engineId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("providerId", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `findScope` over the binary wire.
#[napi(js_name = "findScopeWire")]
pub fn find_scope_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let arg1: Value = wire::read_value(r, &S_FIND_SCOPE_1)?;
        let out = &port::find_scope(text, arg1)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_FIND_SCOPE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_SCOPED_SEARCH_URL_0: wire::Schema = wire::Schema::Dyn;
static S_SCOPED_SEARCH_URL_2: wire::Schema = wire::Schema::Dyn;

/// `scopedSearchUrl` over the binary wire.
#[napi(js_name = "scopedSearchUrlWire")]
pub fn scoped_search_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let scope: Value = wire::read_value(r, &S_SCOPED_SEARCH_URL_0)?;
        let query: String = r.string16()?;
        let default_engine: Value = wire::read_value(r, &S_SCOPED_SEARCH_URL_2)?;
        let out = &port::scoped_search_url(scope, query, default_engine)?;
        w.string16(out);
        Ok(())
    })
}

static S_OFFERED_SITE_SEARCH_2: wire::Schema = wire::Schema::Dyn;
static S_OFFERED_SITE_SEARCH_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("host", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("engineId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("providerId", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `offeredSiteSearch` over the binary wire.
#[napi(js_name = "offeredSiteSearchWire")]
pub fn offered_site_search_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let typed: String = r.string16()?;
        let completion: String = r.string16()?;
        let sources: Value = wire::read_value(r, &S_OFFERED_SITE_SEARCH_2)?;
        let out = &port::offered_site_search(typed, completion, sources)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_OFFERED_SITE_SEARCH_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_KEYWORD_BY_SPACE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("keyword", wire::Schema::Str16),
    ("from", wire::Schema::Str16),
]));

/// `keywordBySpace` over the binary wire.
#[napi(js_name = "keywordBySpaceWire")]
pub fn keyword_by_space_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let typed: String = r.string16()?;
        let next: String = r.string16()?;
        let out = &port::keyword_by_space(typed, next)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_KEYWORD_BY_SPACE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `asksForBackspace` over the binary wire.
#[napi(js_name = "asksForBackspaceWire")]
pub fn asks_for_backspace_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let scope: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::asks_for_backspace(scope)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SITE_SEARCH_KEY_1: wire::Schema = wire::Schema::Dyn;
static S_SITE_SEARCH_KEY_2: wire::Schema = wire::Schema::Dyn;

/// `siteSearchKey` over the binary wire.
#[napi(js_name = "siteSearchKeyWire")]
pub fn site_search_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let key: String = r.string16()?;
        let modifiers: Value = wire::read_value(r, &S_SITE_SEARCH_KEY_1)?;
        let state: Value = wire::read_value(r, &S_SITE_SEARCH_KEY_2)?;
        let out = &port::site_search_key(key, modifiers, state)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
