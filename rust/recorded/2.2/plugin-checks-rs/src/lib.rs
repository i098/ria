//! JS bindings for `apps/browser/src/lib/pluginChecks.harness.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `constantsCase`.
#[napi(js_name = "constantsCase")]
pub fn constants_case() -> napi::Result<String> {
    run(move || {
        let out = port::constants_case()?;
        crate::js::to_json(&out)
    })
}

/// `patternCase`.
#[napi(js_name = "patternCase")]
pub fn pattern_case(name: String, value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::pattern_case(name, value)?;
        Ok(out)
    })
}

/// `atCase`.
#[napi(js_name = "atCase")]
pub fn at_case(path: String, key: String) -> napi::Result<String> {
    run(move || {
        let key = crate::js::from_json(&key)?;
        let out = port::at_case(path, key)?;
        crate::js::to_json(&out)
    })
}

/// `onlyCase`.
#[napi(js_name = "onlyCase")]
pub fn only_case(path: String, doc: String, keys: Vec<String>) -> napi::Result<String> {
    run(move || {
        let doc = crate::js::from_json(&doc)?;
        let out = port::only_case(path, doc, keys)?;
        crate::js::to_json(&out)
    })
}

/// `textCase`.
#[napi(js_name = "textCase")]
pub fn text_case(
    path: String,
    value: String,
    limit_name: String,
    required: bool,
) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::text_case(path, value, limit_name, required)?;
        crate::js::to_json(&out)
    })
}

/// `matchingCase`.
#[napi(js_name = "matchingCase")]
pub fn matching_case(
    path: String,
    value: String,
    pattern_name: String,
    rule: String,
    required: bool,
) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::matching_case(path, value, pattern_name, rule, required)?;
        crate::js::to_json(&out)
    })
}

/// `listCase`.
#[napi(js_name = "listCase")]
pub fn list_case(path: String, value: String, limit_name: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::list_case(path, value, limit_name)?;
        crate::js::to_json(&out)
    })
}

/// `objectCase`.
#[napi(js_name = "objectCase")]
pub fn object_case(path: String, value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::object_case(path, value)?;
        crate::js::to_json(&out)
    })
}

/// `integerCase`.
#[napi(js_name = "integerCase")]
pub fn integer_case(
    path: String,
    value: String,
    min: f64,
    max: f64,
    fallback: f64,
) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::integer_case(path, value, min, max, fallback)?;
        crate::js::to_json(&out)
    })
}

/// `flagCase`.
#[napi(js_name = "flagCase")]
pub fn flag_case(path: String, value: String, fallback: bool) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::flag_case(path, value, fallback)?;
        crate::js::to_json(&out)
    })
}

/// `uniqueCase`.
#[napi(js_name = "uniqueCase")]
pub fn unique_case(path: String, names: String, what: String) -> napi::Result<String> {
    run(move || {
        let names = crate::js::from_json(&names)?;
        let out = port::unique_case(path, names, what)?;
        crate::js::to_json(&out)
    })
}

/// `symbolOfCase`.
#[napi(js_name = "symbolOfCase")]
pub fn symbol_of_case(path: String, value: String, fallback: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::symbol_of_case(path, value, fallback)?;
        crate::js::to_json(&out)
    })
}

/// `isPluginPathCase`.
#[napi(js_name = "isPluginPathCase")]
pub fn is_plugin_path_case(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_plugin_path_case(value)?;
        Ok(out)
    })
}

/// `isArgumentCase`.
#[napi(js_name = "isArgumentCase")]
pub fn is_argument_case(arg: String) -> napi::Result<bool> {
    run(move || {
        let arg = crate::js::from_json(&arg)?;
        let out = port::is_argument_case(arg)?;
        Ok(out)
    })
}

/// `pluginFileCase`.
#[napi(js_name = "pluginFileCase")]
pub fn plugin_file_case(
    path: String,
    value: String,
    rule: String,
    extension: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::plugin_file_case(path, value, rule, extension)?;
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

static S_CONSTANTS_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `constantsCase` over the binary wire.
#[napi(js_name = "constantsCaseWire")]
pub fn constants_case_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::constants_case()?;
        wire::write_value(w, out, &S_CONSTANTS_CASE_OUT)?;
        Ok(())
    })
}

/// `patternCase` over the binary wire.
#[napi(js_name = "patternCaseWire")]
pub fn pattern_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: String = r.string16()?;
        let value: String = r.string16()?;
        let out = &port::pattern_case(name, value)?;
        w.bool(*out);
        Ok(())
    })
}

static S_AT_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_AT_CASE_OUT: wire::Schema = wire::Schema::Str16;

/// `atCase` over the binary wire.
#[napi(js_name = "atCaseWire")]
pub fn at_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let key: Value = wire::read_value(r, &S_AT_CASE_1)?;
        let out = &port::at_case(path, key)?;
        wire::write_value(w, out, &S_AT_CASE_OUT)?;
        Ok(())
    })
}

static S_ONLY_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_ONLY_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `onlyCase` over the binary wire.
#[napi(js_name = "onlyCaseWire")]
pub fn only_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let doc: Value = wire::read_value(r, &S_ONLY_CASE_1)?;
        let keys: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::only_case(path, doc, keys)?;
        wire::write_value(w, out, &S_ONLY_CASE_OUT)?;
        Ok(())
    })
}

static S_TEXT_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_TEXT_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `textCase` over the binary wire.
#[napi(js_name = "textCaseWire")]
pub fn text_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_TEXT_CASE_1)?;
        let limit_name: String = r.string16()?;
        let required: bool = r.bool()?;
        let out = &port::text_case(path, value, limit_name, required)?;
        wire::write_value(w, out, &S_TEXT_CASE_OUT)?;
        Ok(())
    })
}

static S_MATCHING_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_MATCHING_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `matchingCase` over the binary wire.
#[napi(js_name = "matchingCaseWire")]
pub fn matching_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_MATCHING_CASE_1)?;
        let pattern_name: String = r.string16()?;
        let rule: String = r.string16()?;
        let required: bool = r.bool()?;
        let out = &port::matching_case(path, value, pattern_name, rule, required)?;
        wire::write_value(w, out, &S_MATCHING_CASE_OUT)?;
        Ok(())
    })
}

static S_LIST_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_LIST_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `listCase` over the binary wire.
#[napi(js_name = "listCaseWire")]
pub fn list_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_LIST_CASE_1)?;
        let limit_name: String = r.string16()?;
        let out = &port::list_case(path, value, limit_name)?;
        wire::write_value(w, out, &S_LIST_CASE_OUT)?;
        Ok(())
    })
}

static S_OBJECT_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_OBJECT_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `objectCase` over the binary wire.
#[napi(js_name = "objectCaseWire")]
pub fn object_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_OBJECT_CASE_1)?;
        let out = &port::object_case(path, value)?;
        wire::write_value(w, out, &S_OBJECT_CASE_OUT)?;
        Ok(())
    })
}

static S_INTEGER_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_INTEGER_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `integerCase` over the binary wire.
#[napi(js_name = "integerCaseWire")]
pub fn integer_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_INTEGER_CASE_1)?;
        let min: f64 = r.f64()?;
        let max: f64 = r.f64()?;
        let fallback: f64 = r.f64()?;
        let out = &port::integer_case(path, value, min, max, fallback)?;
        wire::write_value(w, out, &S_INTEGER_CASE_OUT)?;
        Ok(())
    })
}

static S_FLAG_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_FLAG_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `flagCase` over the binary wire.
#[napi(js_name = "flagCaseWire")]
pub fn flag_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_FLAG_CASE_1)?;
        let fallback: bool = r.bool()?;
        let out = &port::flag_case(path, value, fallback)?;
        wire::write_value(w, out, &S_FLAG_CASE_OUT)?;
        Ok(())
    })
}

static S_UNIQUE_CASE_1: wire::Schema =
    wire::Schema::List(&wire::Schema::OptJs(&wire::Schema::Str16));
static S_UNIQUE_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `uniqueCase` over the binary wire.
#[napi(js_name = "uniqueCaseWire")]
pub fn unique_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let names: Vec<Value> = r.list(|r| wire::read_value(r, S_UNIQUE_CASE_1.inner()))?;
        let what: String = r.string16()?;
        let out = &port::unique_case(path, names, what)?;
        wire::write_value(w, out, &S_UNIQUE_CASE_OUT)?;
        Ok(())
    })
}

static S_SYMBOL_OF_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_SYMBOL_OF_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `symbolOfCase` over the binary wire.
#[napi(js_name = "symbolOfCaseWire")]
pub fn symbol_of_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_SYMBOL_OF_CASE_1)?;
        let fallback: String = r.string16()?;
        let out = &port::symbol_of_case(path, value, fallback)?;
        wire::write_value(w, out, &S_SYMBOL_OF_CASE_OUT)?;
        Ok(())
    })
}

/// `isPluginPathCase` over the binary wire.
#[napi(js_name = "isPluginPathCaseWire")]
pub fn is_plugin_path_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_plugin_path_case(value)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_ARGUMENT_CASE_0: wire::Schema = wire::Schema::Dyn;

/// `isArgumentCase` over the binary wire.
#[napi(js_name = "isArgumentCaseWire")]
pub fn is_argument_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg: Value = wire::read_value(r, &S_IS_ARGUMENT_CASE_0)?;
        let out = &port::is_argument_case(arg)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PLUGIN_FILE_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_PLUGIN_FILE_CASE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("result", wire::Schema::Dyn),
    ("problems", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `pluginFileCase` over the binary wire.
#[napi(js_name = "pluginFileCaseWire")]
pub fn plugin_file_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_PLUGIN_FILE_CASE_1)?;
        let rule: String = r.string16()?;
        let extension: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::plugin_file_case(path, value, rule, extension)?;
        wire::write_value(w, out, &S_PLUGIN_FILE_CASE_OUT)?;
        Ok(())
    })
}
