//! JS bindings for `apps/browser/src/lib/color.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isHexColor`.
#[napi(js_name = "isHexColor")]
pub fn is_hex_color(color: String) -> napi::Result<String> {
    run(move || {
        let out = port::is_hex_color(color)?;
        crate::js::to_json(&out)
    })
}

/// `isRgbFunction`.
#[napi(js_name = "isRgbFunction")]
pub fn is_rgb_function(color: String) -> napi::Result<String> {
    run(move || {
        let out = port::is_rgb_function(color)?;
        crate::js::to_json(&out)
    })
}

/// `parseColor`.
#[napi(js_name = "parseColor")]
pub fn parse_color(color: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_color(color)?;
        crate::js::to_json(&out)
    })
}

/// `toHex`.
#[napi(js_name = "toHex")]
pub fn to_hex(channels: Vec<f64>) -> napi::Result<String> {
    run(move || {
        let out = port::to_hex(channels)?;
        crate::js::to_json(&out)
    })
}

/// `mix`.
#[napi(js_name = "mix")]
pub fn mix(a: String, b: String, t: f64) -> napi::Result<String> {
    run(move || {
        let out = port::mix(a, b, t)?;
        Ok(out)
    })
}

/// `over`.
#[napi(js_name = "over")]
pub fn over(top: String, under: String) -> napi::Result<String> {
    run(move || {
        let out = port::over(top, under)?;
        Ok(out)
    })
}

/// `colorDistance`.
#[napi(js_name = "colorDistance")]
pub fn color_distance(a: String, b: String) -> napi::Result<f64> {
    run(move || {
        let out = port::color_distance(a, b)?;
        Ok(out)
    })
}

/// `luminance`.
#[napi(js_name = "luminance")]
pub fn luminance(color: String) -> napi::Result<f64> {
    run(move || {
        let out = port::luminance(color)?;
        Ok(out)
    })
}

/// `contrastRatio`.
#[napi(js_name = "contrastRatio")]
pub fn contrast_ratio(a: String, b: String) -> napi::Result<f64> {
    run(move || {
        let out = port::contrast_ratio(a, b)?;
        Ok(out)
    })
}

/// `isLight`.
#[napi(js_name = "isLight")]
pub fn is_light(color: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_light(color)?;
        Ok(out)
    })
}

/// `hslToRgb`.
#[napi(js_name = "hslToRgb")]
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> napi::Result<String> {
    run(move || {
        let out = port::hsl_to_rgb(h, s, l)?;
        crate::js::to_json(&out)
    })
}

/// `rgbToHsl`.
#[napi(js_name = "rgbToHsl")]
pub fn rgb_to_hsl(red: f64, green: f64, blue: f64) -> napi::Result<String> {
    run(move || {
        let out = port::rgb_to_hsl(red, green, blue)?;
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

static S_IS_HEX_COLOR_OUT: wire::Schema = wire::Schema::Dyn;

/// `isHexColor` over the binary wire.
#[napi(js_name = "isHexColorWire")]
pub fn is_hex_color_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let color: String = r.string16()?;
        let out = &port::is_hex_color(color)?;
        wire::write_value(w, out, &S_IS_HEX_COLOR_OUT)?;
        Ok(())
    })
}

static S_IS_RGB_FUNCTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `isRgbFunction` over the binary wire.
#[napi(js_name = "isRgbFunctionWire")]
pub fn is_rgb_function_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let color: String = r.string16()?;
        let out = &port::is_rgb_function(color)?;
        wire::write_value(w, out, &S_IS_RGB_FUNCTION_OUT)?;
        Ok(())
    })
}

static S_PARSE_COLOR_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseColor` over the binary wire.
#[napi(js_name = "parseColorWire")]
pub fn parse_color_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let color: String = r.string16()?;
        let out = &port::parse_color(color)?;
        wire::write_value(w, out, &S_PARSE_COLOR_OUT)?;
        Ok(())
    })
}

static S_TO_HEX_OUT: wire::Schema = wire::Schema::Dyn;

/// `toHex` over the binary wire.
#[napi(js_name = "toHexWire")]
pub fn to_hex_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let channels: Vec<f64> = r.list(|r| r.f64())?;
        let out = &port::to_hex(channels)?;
        wire::write_value(w, out, &S_TO_HEX_OUT)?;
        Ok(())
    })
}

/// `mix` over the binary wire.
#[napi(js_name = "mixWire")]
pub fn mix_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: String = r.string16()?;
        let b: String = r.string16()?;
        let t: f64 = r.f64()?;
        let out = &port::mix(a, b, t)?;
        w.string16(out);
        Ok(())
    })
}

/// `over` over the binary wire.
#[napi(js_name = "overWire")]
pub fn over_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let top: String = r.string16()?;
        let under: String = r.string16()?;
        let out = &port::over(top, under)?;
        w.string16(out);
        Ok(())
    })
}

/// `colorDistance` over the binary wire.
#[napi(js_name = "colorDistanceWire")]
pub fn color_distance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: String = r.string16()?;
        let b: String = r.string16()?;
        let out = &port::color_distance(a, b)?;
        w.f64(*out);
        Ok(())
    })
}

/// `luminance` over the binary wire.
#[napi(js_name = "luminanceWire")]
pub fn luminance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let color: String = r.string16()?;
        let out = &port::luminance(color)?;
        w.f64(*out);
        Ok(())
    })
}

/// `contrastRatio` over the binary wire.
#[napi(js_name = "contrastRatioWire")]
pub fn contrast_ratio_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: String = r.string16()?;
        let b: String = r.string16()?;
        let out = &port::contrast_ratio(a, b)?;
        w.f64(*out);
        Ok(())
    })
}

/// `isLight` over the binary wire.
#[napi(js_name = "isLightWire")]
pub fn is_light_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let color: String = r.string16()?;
        let out = &port::is_light(color)?;
        w.bool(*out);
        Ok(())
    })
}

static S_HSL_TO_RGB_OUT: wire::Schema = wire::Schema::Dyn;

/// `hslToRgb` over the binary wire.
#[napi(js_name = "hslToRgbWire")]
pub fn hsl_to_rgb_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let h: f64 = r.f64()?;
        let s: f64 = r.f64()?;
        let l: f64 = r.f64()?;
        let out = &port::hsl_to_rgb(h, s, l)?;
        wire::write_value(w, out, &S_HSL_TO_RGB_OUT)?;
        Ok(())
    })
}

static S_RGB_TO_HSL_OUT: wire::Schema = wire::Schema::Dyn;

/// `rgbToHsl` over the binary wire.
#[napi(js_name = "rgbToHslWire")]
pub fn rgb_to_hsl_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let red: f64 = r.f64()?;
        let green: f64 = r.f64()?;
        let blue: f64 = r.f64()?;
        let out = &port::rgb_to_hsl(red, green, blue)?;
        wire::write_value(w, out, &S_RGB_TO_HSL_OUT)?;
        Ok(())
    })
}
