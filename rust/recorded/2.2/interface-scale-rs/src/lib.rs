//! JS bindings for `apps/browser/src/lib/interfaceScale.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `interfaceScalePercent`.
#[napi(js_name = "interfaceScalePercent")]
pub fn interface_scale_percent(scale: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::interface_scale_percent(scale)?;
        Ok(out)
    })
}

/// `validInterfaceScale`.
#[napi(js_name = "validInterfaceScale")]
pub fn valid_interface_scale(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::valid_interface_scale(value)?;
        Ok(out)
    })
}

/// `steppedInterfaceScale`.
#[napi(js_name = "steppedInterfaceScale")]
pub fn stepped_interface_scale(value: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::stepped_interface_scale(value)?;
        Ok(out)
    })
}

/// `sameInterfaceScale`.
#[napi(js_name = "sameInterfaceScale")]
pub fn same_interface_scale(a: f64, b: f64) -> napi::Result<bool> {
    run(move || {
        let out = port::same_interface_scale(a, b)?;
        Ok(out)
    })
}

/// `interfaceScaleLabel`.
#[napi(js_name = "interfaceScaleLabel")]
pub fn interface_scale_label(scale: f64) -> napi::Result<String> {
    run(move || {
        let out = port::interface_scale_label(scale)?;
        Ok(out)
    })
}

/// `rootLength`.
#[napi(js_name = "rootLength")]
pub fn root_length(window_length: f64, scale: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::root_length(window_length, scale)?;
        Ok(out)
    })
}

/// `windowLength`.
#[napi(js_name = "windowLength")]
pub fn window_length(root_points: f64, scale: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::window_length(root_points, scale)?;
        Ok(out)
    })
}

/// `snappedToGrid`.
#[napi(js_name = "snappedToGrid")]
pub fn snapped_to_grid(value: f64, pixels_per_point: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::snapped_to_grid(value, pixels_per_point)?;
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

/// `interfaceScalePercent` over the binary wire.
#[napi(js_name = "interfaceScalePercentWire")]
pub fn interface_scale_percent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let scale: f64 = r.f64()?;
        let out = &port::interface_scale_percent(scale)?;
        w.f64(*out);
        Ok(())
    })
}

static S_VALID_INTERFACE_SCALE_0: wire::Schema = wire::Schema::Dyn;

/// `validInterfaceScale` over the binary wire.
#[napi(js_name = "validInterfaceScaleWire")]
pub fn valid_interface_scale_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_VALID_INTERFACE_SCALE_0)?;
        let out = &port::valid_interface_scale(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `steppedInterfaceScale` over the binary wire.
#[napi(js_name = "steppedInterfaceScaleWire")]
pub fn stepped_interface_scale_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: f64 = r.f64()?;
        let out = &port::stepped_interface_scale(value)?;
        w.f64(*out);
        Ok(())
    })
}

/// `sameInterfaceScale` over the binary wire.
#[napi(js_name = "sameInterfaceScaleWire")]
pub fn same_interface_scale_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: f64 = r.f64()?;
        let b: f64 = r.f64()?;
        let out = &port::same_interface_scale(a, b)?;
        w.bool(*out);
        Ok(())
    })
}

/// `interfaceScaleLabel` over the binary wire.
#[napi(js_name = "interfaceScaleLabelWire")]
pub fn interface_scale_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let scale: f64 = r.f64()?;
        let out = &port::interface_scale_label(scale)?;
        w.string16(out);
        Ok(())
    })
}

/// `rootLength` over the binary wire.
#[napi(js_name = "rootLengthWire")]
pub fn root_length_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let window_length: f64 = r.f64()?;
        let scale: f64 = r.f64()?;
        let out = &port::root_length(window_length, scale)?;
        w.f64(*out);
        Ok(())
    })
}

/// `windowLength` over the binary wire.
#[napi(js_name = "windowLengthWire")]
pub fn window_length_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root_points: f64 = r.f64()?;
        let scale: f64 = r.f64()?;
        let out = &port::window_length(root_points, scale)?;
        w.f64(*out);
        Ok(())
    })
}

/// `snappedToGrid` over the binary wire.
#[napi(js_name = "snappedToGridWire")]
pub fn snapped_to_grid_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: f64 = r.f64()?;
        let pixels_per_point: f64 = r.f64()?;
        let out = &port::snapped_to_grid(value, pixels_per_point)?;
        w.f64(*out);
        Ok(())
    })
}
