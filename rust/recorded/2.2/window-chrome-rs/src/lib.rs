//! JS bindings for `apps/browser/src/lib/windowChrome.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `captionRoom`.
#[napi(js_name = "captionRoom")]
pub fn caption_room(from_right: f64, from_top: Option<f64>, chrome: String) -> napi::Result<f64> {
    run(move || {
        let chrome = crate::js::from_json(&chrome)?;
        let out = port::caption_room(from_right, from_top, chrome)?;
        Ok(out)
    })
}

/// `captionClearTop`.
#[napi(js_name = "captionClearTop")]
pub fn caption_clear_top(
    top: f64,
    row_height: f64,
    from_right: f64,
    chrome: String,
) -> napi::Result<f64> {
    run(move || {
        let chrome = crate::js::from_json(&chrome)?;
        let out = port::caption_clear_top(top, row_height, from_right, chrome)?;
        Ok(out)
    })
}

/// `trailingClearance`.
#[napi(js_name = "trailingClearance")]
pub fn trailing_clearance(
    top: f64,
    margin: f64,
    controls_bottom: f64,
    chrome: String,
) -> napi::Result<f64> {
    run(move || {
        let chrome = crate::js::from_json(&chrome)?;
        let out = port::trailing_clearance(top, margin, controls_bottom, chrome)?;
        Ok(out)
    })
}

/// `trafficLightsDrawn`.
#[napi(js_name = "trafficLightsDrawn")]
pub fn traffic_lights_drawn(chrome: String) -> napi::Result<bool> {
    run(move || {
        let chrome = crate::js::from_json(&chrome)?;
        let out = port::traffic_lights_drawn(chrome)?;
        Ok(out)
    })
}

/// `windowControlsDrawn`.
#[napi(js_name = "windowControlsDrawn")]
pub fn window_controls_drawn(chrome: String) -> napi::Result<bool> {
    run(move || {
        let chrome = crate::js::from_json(&chrome)?;
        let out = port::window_controls_drawn(chrome)?;
        Ok(out)
    })
}

/// `windowChromeVersion`.
#[napi(js_name = "windowChromeVersion")]
pub fn window_chrome_version() -> napi::Result<f64> {
    run(move || {
        let out = port::window_chrome_version()?;
        Ok(out)
    })
}

/// `noteWindowChromeChanged`.
#[napi(js_name = "noteWindowChromeChanged")]
pub fn note_window_chrome_changed() -> napi::Result<String> {
    run(move || {
        let out = port::note_window_chrome_changed()?;
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

static S_CAPTION_ROOM_2: wire::Schema = wire::Schema::Dyn;

/// `captionRoom` over the binary wire.
#[napi(js_name = "captionRoomWire")]
pub fn caption_room_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let from_right: f64 = r.f64()?;
        let from_top: Option<f64> = r.opt_js(|r| r.f64())?;
        let chrome: Value = wire::read_value(r, &S_CAPTION_ROOM_2)?;
        let out = &port::caption_room(from_right, from_top, chrome)?;
        w.f64(*out);
        Ok(())
    })
}

static S_CAPTION_CLEAR_TOP_3: wire::Schema = wire::Schema::Dyn;

/// `captionClearTop` over the binary wire.
#[napi(js_name = "captionClearTopWire")]
pub fn caption_clear_top_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let top: f64 = r.f64()?;
        let row_height: f64 = r.f64()?;
        let from_right: f64 = r.f64()?;
        let chrome: Value = wire::read_value(r, &S_CAPTION_CLEAR_TOP_3)?;
        let out = &port::caption_clear_top(top, row_height, from_right, chrome)?;
        w.f64(*out);
        Ok(())
    })
}

static S_TRAILING_CLEARANCE_3: wire::Schema = wire::Schema::Dyn;

/// `trailingClearance` over the binary wire.
#[napi(js_name = "trailingClearanceWire")]
pub fn trailing_clearance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let top: f64 = r.f64()?;
        let margin: f64 = r.f64()?;
        let controls_bottom: f64 = r.f64()?;
        let chrome: Value = wire::read_value(r, &S_TRAILING_CLEARANCE_3)?;
        let out = &port::trailing_clearance(top, margin, controls_bottom, chrome)?;
        w.f64(*out);
        Ok(())
    })
}

static S_TRAFFIC_LIGHTS_DRAWN_0: wire::Schema = wire::Schema::Dyn;

/// `trafficLightsDrawn` over the binary wire.
#[napi(js_name = "trafficLightsDrawnWire")]
pub fn traffic_lights_drawn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let chrome: Value = wire::read_value(r, &S_TRAFFIC_LIGHTS_DRAWN_0)?;
        let out = &port::traffic_lights_drawn(chrome)?;
        w.bool(*out);
        Ok(())
    })
}

static S_WINDOW_CONTROLS_DRAWN_0: wire::Schema = wire::Schema::Dyn;

/// `windowControlsDrawn` over the binary wire.
#[napi(js_name = "windowControlsDrawnWire")]
pub fn window_controls_drawn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let chrome: Value = wire::read_value(r, &S_WINDOW_CONTROLS_DRAWN_0)?;
        let out = &port::window_controls_drawn(chrome)?;
        w.bool(*out);
        Ok(())
    })
}

/// `windowChromeVersion` over the binary wire.
#[napi(js_name = "windowChromeVersionWire")]
pub fn window_chrome_version_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::window_chrome_version()?;
        w.f64(*out);
        Ok(())
    })
}

static S_NOTE_WINDOW_CHROME_CHANGED_OUT: wire::Schema = wire::Schema::Dyn;

/// `noteWindowChromeChanged` over the binary wire.
#[napi(js_name = "noteWindowChromeChangedWire")]
pub fn note_window_chrome_changed_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::note_window_chrome_changed()?;
        wire::write_value(w, out, &S_NOTE_WINDOW_CHROME_CHANGED_OUT)?;
        Ok(())
    })
}
