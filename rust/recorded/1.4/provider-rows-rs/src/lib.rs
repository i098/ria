//! JS bindings for `packages/core/src/providerRows.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;
pub mod utf16;

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

/// `providerText`.
#[napi(js_name = "providerText")]
pub fn provider_text(value: String, max: f64) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::provider_text(value, max)?;
        Ok(out)
    })
}

/// `providerUrl`.
#[napi(js_name = "providerUrl")]
pub fn provider_url(value: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::provider_url(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `providerIcon`.
#[napi(js_name = "providerIcon")]
pub fn provider_icon(value: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::provider_icon(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `safeProviderRows`.
#[napi(js_name = "safeProviderRows")]
pub fn safe_provider_rows(
    section: String,
    rows: String,
    limit: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let section = crate::js::from_json(&section)?;
        let rows = crate::js::from_json(&rows)?;
        let out = port::safe_provider_rows(section, rows, limit)?;
        crate::js::to_json(&out)
    })
}

/// `fillProviderTemplate`.
#[napi(js_name = "fillProviderTemplate")]
pub fn fill_provider_template(
    template: String,
    query: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::fill_provider_template(template, query)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `parseProviderReply`.
#[napi(js_name = "parseProviderReply")]
pub fn parse_provider_reply(body: String, template: Option<String>) -> napi::Result<String> {
    run(move || {
        let body = crate::js::from_json(&body)?;
        let out = port::parse_provider_reply(body, template)?;
        crate::js::to_json(&out)
    })
}

/// `matchProviderRows`.
#[napi(js_name = "matchProviderRows")]
pub fn match_provider_rows(rows: String, text: String) -> napi::Result<String> {
    run(move || {
        let rows = crate::js::from_json(&rows)?;
        let out = port::match_provider_rows(rows, text)?;
        crate::js::to_json(&out)
    })
}

/// `globalProviderRows`.
#[napi(js_name = "globalProviderRows")]
pub fn global_provider_rows(sections: String, shown_rows: String) -> napi::Result<String> {
    run(move || {
        let sections = crate::js::from_json(&sections)?;
        let shown_rows = crate::js::from_json(&shown_rows)?;
        let out = port::global_provider_rows(sections, shown_rows)?;
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

static S_PROVIDER_TEXT_0: wire::Schema = wire::Schema::Dyn;

/// `providerText` over the binary wire.
#[napi(js_name = "providerTextWire")]
pub fn provider_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_PROVIDER_TEXT_0)?;
        let max: f64 = r.f64()?;
        let out = &port::provider_text(value, max)?;
        w.string16(out);
        Ok(())
    })
}

static S_PROVIDER_URL_0: wire::Schema = wire::Schema::Dyn;

/// `providerUrl` over the binary wire.
#[napi(js_name = "providerUrlWire")]
pub fn provider_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_PROVIDER_URL_0)?;
        let out = &port::provider_url(value)?;
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

static S_PROVIDER_ICON_0: wire::Schema = wire::Schema::Dyn;

/// `providerIcon` over the binary wire.
#[napi(js_name = "providerIconWire")]
pub fn provider_icon_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_PROVIDER_ICON_0)?;
        let out = &port::provider_icon(value)?;
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

static S_SAFE_PROVIDER_ROWS_0: wire::Schema = wire::Schema::Dyn;
static S_SAFE_PROVIDER_ROWS_1: wire::Schema = wire::Schema::Dyn;
static S_SAFE_PROVIDER_ROWS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `safeProviderRows` over the binary wire.
#[napi(js_name = "safeProviderRowsWire")]
pub fn safe_provider_rows_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let section: Value = wire::read_value(r, &S_SAFE_PROVIDER_ROWS_0)?;
        let rows: Value = wire::read_value(r, &S_SAFE_PROVIDER_ROWS_1)?;
        let limit: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::safe_provider_rows(section, rows, limit)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_SAFE_PROVIDER_ROWS_OUT.inner())?;
        }
        Ok(())
    })
}

/// `fillProviderTemplate` over the binary wire.
#[napi(js_name = "fillProviderTemplateWire")]
pub fn fill_provider_template_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let template: String = r.string16()?;
        let query: String = r.string16()?;
        let out = &port::fill_provider_template(template, query)?;
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

static S_PARSE_PROVIDER_REPLY_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_PROVIDER_REPLY_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("title", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("subtitle", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("icon", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("badge", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `parseProviderReply` over the binary wire.
#[napi(js_name = "parseProviderReplyWire")]
pub fn parse_provider_reply_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let body: Value = wire::read_value(r, &S_PARSE_PROVIDER_REPLY_0)?;
        let template: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::parse_provider_reply(body, template)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_PROVIDER_REPLY_OUT.inner())?;
        }
        Ok(())
    })
}

static S_MATCH_PROVIDER_ROWS_0: wire::Schema = wire::Schema::Dyn;
static S_MATCH_PROVIDER_ROWS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("title", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("subtitle", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("icon", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("badge", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `matchProviderRows` over the binary wire.
#[napi(js_name = "matchProviderRowsWire")]
pub fn match_provider_rows_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let rows: Value = wire::read_value(r, &S_MATCH_PROVIDER_ROWS_0)?;
        let text: String = r.string16()?;
        let out = &port::match_provider_rows(rows, text)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_MATCH_PROVIDER_ROWS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_GLOBAL_PROVIDER_ROWS_0: wire::Schema = wire::Schema::Dyn;
static S_GLOBAL_PROVIDER_ROWS_1: wire::Schema = wire::Schema::Dyn;
static S_GLOBAL_PROVIDER_ROWS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `globalProviderRows` over the binary wire.
#[napi(js_name = "globalProviderRowsWire")]
pub fn global_provider_rows_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sections: Value = wire::read_value(r, &S_GLOBAL_PROVIDER_ROWS_0)?;
        let shown_rows: Value = wire::read_value(r, &S_GLOBAL_PROVIDER_ROWS_1)?;
        let out = &port::global_provider_rows(sections, shown_rows)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_GLOBAL_PROVIDER_ROWS_OUT.inner())?;
        }
        Ok(())
    })
}
