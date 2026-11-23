//! JS bindings for `packages/core/src/engines.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `validSearchTemplate`.
#[napi(js_name = "validSearchTemplate")]
pub fn valid_search_template(url: String) -> napi::Result<bool> {
    run(move || {
        let url = crate::js::from_json(&url)?;
        let out = port::valid_search_template(url)?;
        Ok(out)
    })
}

/// `customEngineFrom`.
#[napi(js_name = "customEngineFrom")]
pub fn custom_engine_from(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::custom_engine_from(raw)?;
        crate::js::to_json(&out)
    })
}

/// `customEnginesFrom`.
#[napi(js_name = "customEnginesFrom")]
pub fn custom_engines_from(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::custom_engines_from(raw)?;
        crate::js::to_json(&out)
    })
}

/// `allEngines`.
#[napi(js_name = "allEngines")]
pub fn all_engines(custom: String, extension: String) -> napi::Result<String> {
    run(move || {
        let custom = crate::js::from_json(&custom)?;
        let extension = crate::js::from_json(&extension)?;
        let out = port::all_engines(custom, extension)?;
        crate::js::to_json(&out)
    })
}

/// `extensionEngineId`.
#[napi(js_name = "extensionEngineId")]
pub fn extension_engine_id(extension_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::extension_engine_id(extension_id)?;
        Ok(out)
    })
}

/// `controllingExtensionEngine`.
#[napi(js_name = "controllingExtensionEngine")]
pub fn controlling_extension_engine(extension: String) -> napi::Result<String> {
    run(move || {
        let extension = crate::js::from_json(&extension)?;
        let out = port::controlling_extension_engine(extension)?;
        crate::js::to_json(&out)
    })
}

/// `engineById`.
#[napi(js_name = "engineById")]
pub fn engine_by_id(engines: String, id: Option<String>) -> napi::Result<String> {
    run(move || {
        let engines = crate::js::from_json(&engines)?;
        let out = port::engine_by_id(engines, id)?;
        crate::js::to_json(&out)
    })
}

/// `searchUrl`.
#[napi(js_name = "searchUrl")]
pub fn search_url(engine: String, query: String) -> napi::Result<String> {
    run(move || {
        let engine = crate::js::from_json(&engine)?;
        let out = port::search_url(engine, query)?;
        Ok(out)
    })
}

/// `suggestRequestUrl`.
#[napi(js_name = "suggestRequestUrl")]
pub fn suggest_request_url(
    engine: String,
    query: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let engine = crate::js::from_json(&engine)?;
        let out = port::suggest_request_url(engine, query)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `engineHost`.
#[napi(js_name = "engineHost")]
pub fn engine_host(engine: String) -> napi::Result<String> {
    run(move || {
        let engine = crate::js::from_json(&engine)?;
        let out = port::engine_host(engine)?;
        Ok(out)
    })
}

/// `suggestReplyJson`.
#[napi(js_name = "suggestReplyJson")]
pub fn suggest_reply_json(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::suggest_reply_json(text)?;
        crate::js::to_json(&out)
    })
}

/// `parseRemoteSuggestions`.
#[napi(js_name = "parseRemoteSuggestions")]
pub fn parse_remote_suggestions(body: String) -> napi::Result<String> {
    run(move || {
        let body = crate::js::from_json(&body)?;
        let out = port::parse_remote_suggestions(body)?;
        crate::js::to_json(&out)
    })
}

/// `parseSuggestions`.
#[napi(js_name = "parseSuggestions")]
pub fn parse_suggestions(body: String) -> napi::Result<Vec<String>> {
    run(move || {
        let body = crate::js::from_json(&body)?;
        let out = port::parse_suggestions(body)?;
        Ok(out)
    })
}

/// `validateEngine`.
#[napi(js_name = "validateEngine")]
pub fn validate_engine(
    input: String,
    engines: String,
    ignore_id: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let engines = crate::js::from_json(&engines)?;
        let out = port::validate_engine(input, engines, ignore_id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `makeCustomEngine`.
#[napi(js_name = "makeCustomEngine")]
pub fn make_custom_engine(input: String, id: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::make_custom_engine(input, id)?;
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

static S_VALID_SEARCH_TEMPLATE_0: wire::Schema = wire::Schema::Dyn;

/// `validSearchTemplate` over the binary wire.
#[napi(js_name = "validSearchTemplateWire")]
pub fn valid_search_template_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: Value = wire::read_value(r, &S_VALID_SEARCH_TEMPLATE_0)?;
        let out = &port::valid_search_template(url)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_CUSTOM_ENGINE_FROM_0: wire::Schema = wire::Schema::Dyn;
static S_CUSTOM_ENGINE_FROM_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("keyword", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
]));

/// `customEngineFrom` over the binary wire.
#[napi(js_name = "customEngineFromWire")]
pub fn custom_engine_from_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Value = wire::read_value(r, &S_CUSTOM_ENGINE_FROM_0)?;
        let out = &port::custom_engine_from(raw)?;
        wire::write_value(w, out, &S_CUSTOM_ENGINE_FROM_OUT)?;
        Ok(())
    })
}

static S_CUSTOM_ENGINES_FROM_0: wire::Schema = wire::Schema::Dyn;
static S_CUSTOM_ENGINES_FROM_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("keyword", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
]));

/// `customEnginesFrom` over the binary wire.
#[napi(js_name = "customEnginesFromWire")]
pub fn custom_engines_from_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Value = wire::read_value(r, &S_CUSTOM_ENGINES_FROM_0)?;
        let out = &port::custom_engines_from(raw)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_CUSTOM_ENGINES_FROM_OUT.inner())?;
        }
        Ok(())
    })
}

static S_ALL_ENGINES_0: wire::Schema = wire::Schema::Dyn;
static S_ALL_ENGINES_1: wire::Schema = wire::Schema::Dyn;
static S_ALL_ENGINES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("keyword", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("suggestUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("custom", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "extension",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
        ])),
    ),
]));

/// `allEngines` over the binary wire.
#[napi(js_name = "allEnginesWire")]
pub fn all_engines_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let custom: Value = wire::read_value(r, &S_ALL_ENGINES_0)?;
        let extension: Value = wire::read_value(r, &S_ALL_ENGINES_1)?;
        let out = &port::all_engines(custom, extension)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_ALL_ENGINES_OUT.inner())?;
        }
        Ok(())
    })
}

/// `extensionEngineId` over the binary wire.
#[napi(js_name = "extensionEngineIdWire")]
pub fn extension_engine_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let extension_id: String = r.string16()?;
        let out = &port::extension_engine_id(extension_id)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_CONTROLLING_EXTENSION_ENGINE_0: wire::Schema = wire::Schema::Dyn;
static S_CONTROLLING_EXTENSION_ENGINE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("profile", wire::Schema::Str16),
        ("extensionId", wire::Schema::Str16),
        ("extensionName", wire::Schema::Str16),
        ("name", wire::Schema::Str16),
        ("keyword", wire::Schema::Str16),
        ("url", wire::Schema::Str16),
        ("suggestUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("isDefault", wire::Schema::Bool),
    ]));

/// `controllingExtensionEngine` over the binary wire.
#[napi(js_name = "controllingExtensionEngineWire")]
pub fn controlling_extension_engine_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let extension: Value = wire::read_value(r, &S_CONTROLLING_EXTENSION_ENGINE_0)?;
        let out = &port::controlling_extension_engine(extension)?;
        wire::write_value(w, out, &S_CONTROLLING_EXTENSION_ENGINE_OUT)?;
        Ok(())
    })
}

static S_ENGINE_BY_ID_0: wire::Schema = wire::Schema::Dyn;
static S_ENGINE_BY_ID_OUT: wire::Schema = wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("keyword", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("suggestUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("custom", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "extension",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
        ])),
    ),
]);

/// `engineById` over the binary wire.
#[napi(js_name = "engineByIdWire")]
pub fn engine_by_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let engines: Value = wire::read_value(r, &S_ENGINE_BY_ID_0)?;
        let id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::engine_by_id(engines, id)?;
        wire::write_value(w, out, &S_ENGINE_BY_ID_OUT)?;
        Ok(())
    })
}

static S_SEARCH_URL_0: wire::Schema = wire::Schema::Dyn;

/// `searchUrl` over the binary wire.
#[napi(js_name = "searchUrlWire")]
pub fn search_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let engine: Value = wire::read_value(r, &S_SEARCH_URL_0)?;
        let query: String = r.string16()?;
        let out = &port::search_url(engine, query)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SUGGEST_REQUEST_URL_0: wire::Schema = wire::Schema::Dyn;

/// `suggestRequestUrl` over the binary wire.
#[napi(js_name = "suggestRequestUrlWire")]
pub fn suggest_request_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let engine: Value = wire::read_value(r, &S_SUGGEST_REQUEST_URL_0)?;
        let query: String = r.string16()?;
        let out = &port::suggest_request_url(engine, query)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_ENGINE_HOST_0: wire::Schema = wire::Schema::Dyn;

/// `engineHost` over the binary wire.
#[napi(js_name = "engineHostWire")]
pub fn engine_host_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let engine: Value = wire::read_value(r, &S_ENGINE_HOST_0)?;
        let out = &port::engine_host(engine)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SUGGEST_REPLY_JSON_OUT: wire::Schema = wire::Schema::Dyn;

/// `suggestReplyJson` over the binary wire.
#[napi(js_name = "suggestReplyJsonWire")]
pub fn suggest_reply_json_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::suggest_reply_json(text)?;
        wire::write_value(w, out, &S_SUGGEST_REPLY_JSON_OUT)?;
        Ok(())
    })
}

static S_PARSE_REMOTE_SUGGESTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_REMOTE_SUGGESTIONS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("queries", wire::Schema::List(&wire::Schema::Str16)),
    (
        "sites",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("url", wire::Schema::Str16),
            ("title", wire::Schema::Str16),
        ])),
    ),
]);

/// `parseRemoteSuggestions` over the binary wire.
#[napi(js_name = "parseRemoteSuggestionsWire")]
pub fn parse_remote_suggestions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let body: Value = wire::read_value(r, &S_PARSE_REMOTE_SUGGESTIONS_0)?;
        let out = &port::parse_remote_suggestions(body)?;
        wire::write_value(w, out, &S_PARSE_REMOTE_SUGGESTIONS_OUT)?;
        Ok(())
    })
}

static S_PARSE_SUGGESTIONS_0: wire::Schema = wire::Schema::Dyn;

/// `parseSuggestions` over the binary wire.
#[napi(js_name = "parseSuggestionsWire")]
pub fn parse_suggestions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let body: Value = wire::read_value(r, &S_PARSE_SUGGESTIONS_0)?;
        let out = &port::parse_suggestions(body)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_VALIDATE_ENGINE_0: wire::Schema = wire::Schema::Dyn;
static S_VALIDATE_ENGINE_1: wire::Schema = wire::Schema::Dyn;

/// `validateEngine` over the binary wire.
#[napi(js_name = "validateEngineWire")]
pub fn validate_engine_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_VALIDATE_ENGINE_0)?;
        let engines: Value = wire::read_value(r, &S_VALIDATE_ENGINE_1)?;
        let ignore_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::validate_engine(input, engines, ignore_id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_MAKE_CUSTOM_ENGINE_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_CUSTOM_ENGINE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("keyword", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
]);

/// `makeCustomEngine` over the binary wire.
#[napi(js_name = "makeCustomEngineWire")]
pub fn make_custom_engine_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_MAKE_CUSTOM_ENGINE_0)?;
        let id: String = r.string16()?;
        let out = &port::make_custom_engine(input, id)?;
        wire::write_value(w, out, &S_MAKE_CUSTOM_ENGINE_OUT)?;
        Ok(())
    })
}
