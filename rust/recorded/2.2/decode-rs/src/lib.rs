//! JS bindings for `apps/browser/src/lib/decode.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isFields`.
#[napi(js_name = "isFields")]
pub fn is_fields(v: String) -> napi::Result<bool> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::is_fields(v)?;
        Ok(out)
    })
}

/// `isString`.
#[napi(js_name = "isString")]
pub fn is_string(v: String) -> napi::Result<bool> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::is_string(v)?;
        Ok(out)
    })
}

/// `isNumber`.
#[napi(js_name = "isNumber")]
pub fn is_number(v: String) -> napi::Result<bool> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::is_number(v)?;
        Ok(out)
    })
}

/// `isStringList`.
#[napi(js_name = "isStringList")]
pub fn is_string_list(v: String) -> napi::Result<bool> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::is_string_list(v)?;
        Ok(out)
    })
}

/// `stringOr`.
#[napi(js_name = "stringOr")]
pub fn string_or(v: String, fallback: String) -> napi::Result<String> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let fallback = crate::js::from_json(&fallback)?;
        let out = port::string_or(v, fallback)?;
        crate::js::to_json(&out)
    })
}

/// `numberOr`.
#[napi(js_name = "numberOr")]
pub fn number_or(v: String, fallback: String) -> napi::Result<String> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let fallback = crate::js::from_json(&fallback)?;
        let out = port::number_or(v, fallback)?;
        crate::js::to_json(&out)
    })
}

/// `listOf`.
#[napi(js_name = "listOf")]
pub fn list_of(v: String) -> napi::Result<String> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::list_of(v)?;
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

static S_IS_FIELDS_0: wire::Schema = wire::Schema::Dyn;

/// `isFields` over the binary wire.
#[napi(js_name = "isFieldsWire")]
pub fn is_fields_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: Value = wire::read_value(r, &S_IS_FIELDS_0)?;
        let out = &port::is_fields(v)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_STRING_0: wire::Schema = wire::Schema::Dyn;

/// `isString` over the binary wire.
#[napi(js_name = "isStringWire")]
pub fn is_string_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: Value = wire::read_value(r, &S_IS_STRING_0)?;
        let out = &port::is_string(v)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_NUMBER_0: wire::Schema = wire::Schema::Dyn;

/// `isNumber` over the binary wire.
#[napi(js_name = "isNumberWire")]
pub fn is_number_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: Value = wire::read_value(r, &S_IS_NUMBER_0)?;
        let out = &port::is_number(v)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_STRING_LIST_0: wire::Schema = wire::Schema::Dyn;

/// `isStringList` over the binary wire.
#[napi(js_name = "isStringListWire")]
pub fn is_string_list_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: Value = wire::read_value(r, &S_IS_STRING_LIST_0)?;
        let out = &port::is_string_list(v)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_STRING_OR_0: wire::Schema = wire::Schema::Dyn;
static S_STRING_OR_1: wire::Schema = wire::Schema::Dyn;
static S_STRING_OR_OUT: wire::Schema = wire::Schema::Dyn;

/// `stringOr` over the binary wire.
#[napi(js_name = "stringOrWire")]
pub fn string_or_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: Value = wire::read_value(r, &S_STRING_OR_0)?;
        let fallback: Value = wire::read_value(r, &S_STRING_OR_1)?;
        let out = &port::string_or(v, fallback)?;
        wire::write_value(w, out, &S_STRING_OR_OUT)?;
        Ok(())
    })
}

static S_NUMBER_OR_0: wire::Schema = wire::Schema::Dyn;
static S_NUMBER_OR_1: wire::Schema = wire::Schema::Dyn;
static S_NUMBER_OR_OUT: wire::Schema = wire::Schema::Dyn;

/// `numberOr` over the binary wire.
#[napi(js_name = "numberOrWire")]
pub fn number_or_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: Value = wire::read_value(r, &S_NUMBER_OR_0)?;
        let fallback: Value = wire::read_value(r, &S_NUMBER_OR_1)?;
        let out = &port::number_or(v, fallback)?;
        wire::write_value(w, out, &S_NUMBER_OR_OUT)?;
        Ok(())
    })
}

static S_LIST_OF_0: wire::Schema = wire::Schema::Dyn;
static S_LIST_OF_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `listOf` over the binary wire.
#[napi(js_name = "listOfWire")]
pub fn list_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: Value = wire::read_value(r, &S_LIST_OF_0)?;
        let out = &port::list_of(v)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_LIST_OF_OUT.inner())?;
        }
        Ok(())
    })
}
