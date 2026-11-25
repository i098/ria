//! JS bindings for `apps/browser/src/lib/toolbarRow.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `toolbarRowOf`.
#[napi(js_name = "toolbarRowOf")]
pub fn toolbar_row_of(compact: bool) -> napi::Result<String> {
    run(move || {
        let out = port::toolbar_row_of(compact)?;
        crate::js::to_json(&out)
    })
}

/// `urlFieldOf`.
#[napi(js_name = "urlFieldOf")]
pub fn url_field_of(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::url_field_of(row)?;
        crate::js::to_json(&out)
    })
}

/// `compactLift`.
#[napi(js_name = "compactLift")]
pub fn compact_lift(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::compact_lift(row)?;
        crate::js::to_json(&out)
    })
}

/// `fieldInsetOf`.
#[napi(js_name = "fieldInsetOf")]
pub fn field_inset_of(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::field_inset_of(row)?;
        crate::js::to_json(&out)
    })
}

/// `fieldAloneInsetOf`.
#[napi(js_name = "fieldAloneInsetOf")]
pub fn field_alone_inset_of(row: String, card_radius: f64) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::field_alone_inset_of(row, card_radius)?;
        crate::js::to_json(&out)
    })
}

/// `toolbarTrailing`.
#[napi(js_name = "toolbarTrailing")]
pub fn toolbar_trailing(row: String, o: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let o = crate::js::from_json(&o)?;
        let out = port::toolbar_trailing(row, o)?;
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

static S_TOOLBAR_ROW_OF_OUT: wire::Schema = wire::Schema::Obj(&[
    ("height", wire::Schema::Num),
    ("center", wire::Schema::Num),
    ("button", wire::Schema::Num),
    ("buttonRadius", wire::Schema::Num),
    ("field", wire::Schema::Num),
    ("fieldRadius", wire::Schema::Num),
    ("fieldButton", wire::Schema::Num),
    ("fieldButtonRadius", wire::Schema::Num),
    ("extensionButton", wire::Schema::Num),
]);

/// `toolbarRowOf` over the binary wire.
#[napi(js_name = "toolbarRowOfWire")]
pub fn toolbar_row_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let compact: bool = r.bool()?;
        let out = &port::toolbar_row_of(compact)?;
        wire::write_value(w, out, &S_TOOLBAR_ROW_OF_OUT)?;
        Ok(())
    })
}

static S_URL_FIELD_OF_0: wire::Schema = wire::Schema::Dyn;
static S_URL_FIELD_OF_OUT: wire::Schema = wire::Schema::Dyn;

/// `urlFieldOf` over the binary wire.
#[napi(js_name = "urlFieldOfWire")]
pub fn url_field_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_URL_FIELD_OF_0)?;
        let out = &port::url_field_of(row)?;
        wire::write_value(w, out, &S_URL_FIELD_OF_OUT)?;
        Ok(())
    })
}

static S_COMPACT_LIFT_0: wire::Schema = wire::Schema::Dyn;
static S_COMPACT_LIFT_OUT: wire::Schema = wire::Schema::Dyn;

/// `compactLift` over the binary wire.
#[napi(js_name = "compactLiftWire")]
pub fn compact_lift_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_COMPACT_LIFT_0)?;
        let out = &port::compact_lift(row)?;
        wire::write_value(w, out, &S_COMPACT_LIFT_OUT)?;
        Ok(())
    })
}

static S_FIELD_INSET_OF_0: wire::Schema = wire::Schema::Dyn;
static S_FIELD_INSET_OF_OUT: wire::Schema = wire::Schema::Dyn;

/// `fieldInsetOf` over the binary wire.
#[napi(js_name = "fieldInsetOfWire")]
pub fn field_inset_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_FIELD_INSET_OF_0)?;
        let out = &port::field_inset_of(row)?;
        wire::write_value(w, out, &S_FIELD_INSET_OF_OUT)?;
        Ok(())
    })
}

static S_FIELD_ALONE_INSET_OF_0: wire::Schema = wire::Schema::Dyn;
static S_FIELD_ALONE_INSET_OF_OUT: wire::Schema = wire::Schema::Dyn;

/// `fieldAloneInsetOf` over the binary wire.
#[napi(js_name = "fieldAloneInsetOfWire")]
pub fn field_alone_inset_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_FIELD_ALONE_INSET_OF_0)?;
        let card_radius: f64 = r.f64()?;
        let out = &port::field_alone_inset_of(row, card_radius)?;
        wire::write_value(w, out, &S_FIELD_ALONE_INSET_OF_OUT)?;
        Ok(())
    })
}

static S_TOOLBAR_TRAILING_0: wire::Schema = wire::Schema::Dyn;
static S_TOOLBAR_TRAILING_1: wire::Schema = wire::Schema::Dyn;
static S_TOOLBAR_TRAILING_OUT: wire::Schema = wire::Schema::Dyn;

/// `toolbarTrailing` over the binary wire.
#[napi(js_name = "toolbarTrailingWire")]
pub fn toolbar_trailing_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_TOOLBAR_TRAILING_0)?;
        let o: serde_json::Value = wire::read_value(r, &S_TOOLBAR_TRAILING_1)?;
        let out = &port::toolbar_trailing(row, o)?;
        wire::write_value(w, out, &S_TOOLBAR_TRAILING_OUT)?;
        Ok(())
    })
}
