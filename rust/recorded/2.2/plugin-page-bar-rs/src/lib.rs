//! JS bindings for `apps/browser/src/lib/pluginPageBar.harness.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isHostActionCase`.
#[napi(js_name = "isHostActionCase")]
pub fn is_host_action_case(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_host_action_case(value)?;
        Ok(out)
    })
}

/// `pageBarCase`.
#[napi(js_name = "pageBarCase")]
pub fn page_bar_case(switches: String, bar: String, rows: String) -> napi::Result<String> {
    run(move || {
        let switches = crate::js::from_json(&switches)?;
        let bar = crate::js::from_json(&bar)?;
        let rows = crate::js::from_json(&rows)?;
        let out = port::page_bar_case(switches, bar, rows)?;
        crate::js::to_json(&out)
    })
}

/// `switchOriginCase`.
#[napi(js_name = "switchOriginCase")]
pub fn switch_origin_case(url: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::switch_origin_case(url)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isLocalOriginCase`.
#[napi(js_name = "isLocalOriginCase")]
pub fn is_local_origin_case(origin: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_local_origin_case(origin)?;
        Ok(out)
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

static S_IS_HOST_ACTION_CASE_0: wire::Schema = wire::Schema::Dyn;

/// `isHostActionCase` over the binary wire.
#[napi(js_name = "isHostActionCaseWire")]
pub fn is_host_action_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_IS_HOST_ACTION_CASE_0)?;
        let out = &port::is_host_action_case(value)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PAGE_BAR_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_PAGE_BAR_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_PAGE_BAR_CASE_2: wire::Schema = wire::Schema::Dyn;
static S_PAGE_BAR_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `pageBarCase` over the binary wire.
#[napi(js_name = "pageBarCaseWire")]
pub fn page_bar_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let switches: Value = wire::read_value(r, &S_PAGE_BAR_CASE_0)?;
        let bar: Value = wire::read_value(r, &S_PAGE_BAR_CASE_1)?;
        let rows: Value = wire::read_value(r, &S_PAGE_BAR_CASE_2)?;
        let out = &port::page_bar_case(switches, bar, rows)?;
        wire::write_value(w, out, &S_PAGE_BAR_CASE_OUT)?;
        Ok(())
    })
}

/// `switchOriginCase` over the binary wire.
#[napi(js_name = "switchOriginCaseWire")]
pub fn switch_origin_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::switch_origin_case(url)?;
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

/// `isLocalOriginCase` over the binary wire.
#[napi(js_name = "isLocalOriginCaseWire")]
pub fn is_local_origin_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let origin: String = r.string16()?;
        let out = &port::is_local_origin_case(origin)?;
        w.bool(*out);
        Ok(())
    })
}
