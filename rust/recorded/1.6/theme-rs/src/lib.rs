//! JS bindings for `packages/core/src/theme.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `padColor`.
#[napi(js_name = "padColor")]
pub fn pad_color(x: f64, y: f64, lightness: String) -> napi::Result<String> {
    run(move || {
        let lightness = crate::js::from_json(&lightness)?;
        let out = port::pad_color(x, y, lightness)?;
        crate::js::to_json(&out)
    })
}

/// `padPosition`.
#[napi(js_name = "padPosition")]
pub fn pad_position(color: String) -> napi::Result<String> {
    run(move || {
        let color = crate::js::from_json(&color)?;
        let out = port::pad_position(color)?;
        crate::js::to_json(&out)
    })
}

/// `oklab`.
#[napi(js_name = "oklab")]
pub fn oklab(c: String) -> napi::Result<String> {
    run(move || {
        let c = crate::js::from_json(&c)?;
        let out = port::oklab(c)?;
        crate::js::to_json(&out)
    })
}

/// `fromOklab`.
#[napi(js_name = "fromOklab")]
pub fn from_oklab(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::from_oklab(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `colorHex`.
#[napi(js_name = "colorHex")]
pub fn color_hex(color: String) -> napi::Result<String> {
    run(move || {
        let color = crate::js::from_json(&color)?;
        let out = port::color_hex(color)?;
        Ok(out)
    })
}

/// `hexColor`.
#[napi(js_name = "hexColor")]
pub fn hex_color(hex: String) -> napi::Result<String> {
    run(move || {
        let out = port::hex_color(hex)?;
        crate::js::to_json(&out)
    })
}

/// `mixOklab`.
#[napi(js_name = "mixOklab")]
pub fn mix_oklab(a: String, b: String, t: f64) -> napi::Result<String> {
    run(move || {
        let a = crate::js::from_json(&a)?;
        let b = crate::js::from_json(&b)?;
        let out = port::mix_oklab(a, b, t)?;
        crate::js::to_json(&out)
    })
}

/// `lookAngle`.
#[napi(js_name = "lookAngle")]
pub fn look_angle(theme: String) -> napi::Result<f64> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::look_angle(theme)?;
        Ok(out)
    })
}

/// `themeArea`.
#[napi(js_name = "themeArea")]
pub fn theme_area(theme: String, x: f64, y: f64, width: f64, height: f64) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_area(theme, x, y, width, height)?;
        crate::js::to_json(&out)
    })
}

/// `themeDark`.
#[napi(js_name = "themeDark")]
pub fn theme_dark(theme: String, app_dark: bool) -> napi::Result<bool> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_dark(theme, app_dark)?;
        Ok(out)
    })
}

/// `resolveTheme`.
#[napi(js_name = "resolveTheme")]
pub fn resolve_theme(theme: String, dark: bool) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::resolve_theme(theme, dark)?;
        crate::js::to_json(&out)
    })
}

/// `darkenedLook`.
#[napi(js_name = "darkenedLook")]
pub fn darkened_look(theme: String, keep: String) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let keep = crate::js::from_json(&keep)?;
        let out = port::darkened_look(theme, keep)?;
        crate::js::to_json(&out)
    })
}

/// `requestedDarkFor`.
#[napi(js_name = "requestedDarkFor")]
pub fn requested_dark_for(theme: String, app_dark: bool, source: String) -> napi::Result<bool> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::requested_dark_for(theme, app_dark, source)?;
        Ok(out)
    })
}

/// `shownLook`.
#[napi(js_name = "shownLook")]
pub fn shown_look(theme: String, app_dark: bool, source: String) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::shown_look(theme, app_dark, source)?;
        crate::js::to_json(&out)
    })
}

/// `arcLayerAlpha`.
#[napi(js_name = "arcLayerAlpha")]
pub fn arc_layer_alpha(intensity: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::arc_layer_alpha(intensity)?;
        Ok(out)
    })
}

/// `themeLayerAlpha`.
#[napi(js_name = "themeLayerAlpha")]
pub fn theme_layer_alpha(theme: String) -> napi::Result<f64> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_layer_alpha(theme)?;
        Ok(out)
    })
}

/// `themeUnderlayAlpha`.
#[napi(js_name = "themeUnderlayAlpha")]
pub fn theme_underlay_alpha(theme: String, dark: bool) -> napi::Result<f64> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_underlay_alpha(theme, dark)?;
        Ok(out)
    })
}

/// `themeGrainAlpha`.
#[napi(js_name = "themeGrainAlpha")]
pub fn theme_grain_alpha(theme: String) -> napi::Result<f64> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_grain_alpha(theme)?;
        Ok(out)
    })
}

/// `flatColors`.
#[napi(js_name = "flatColors")]
pub fn flat_colors(theme: String) -> napi::Result<Vec<f64>> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::flat_colors(theme)?;
        Ok(out)
    })
}

/// `themeAccent`.
#[napi(js_name = "themeAccent")]
pub fn theme_accent(theme: String) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_accent(theme)?;
        Ok(out)
    })
}

/// `themePalette`.
#[napi(js_name = "themePalette")]
pub fn theme_palette(theme: String) -> napi::Result<Vec<String>> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_palette(theme)?;
        Ok(out)
    })
}

/// `backdropUnderColor`.
#[napi(js_name = "backdropUnderColor")]
pub fn backdrop_under_color(underlay_alpha: f64, dark: bool) -> napi::Result<String> {
    run(move || {
        let out = port::backdrop_under_color(underlay_alpha, dark)?;
        Ok(out)
    })
}

/// `themeUnderColor`.
#[napi(js_name = "themeUnderColor")]
pub fn theme_under_color(theme: String, dark: bool) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_under_color(theme, dark)?;
        Ok(out)
    })
}

/// `opaqueThemeTint`.
#[napi(js_name = "opaqueThemeTint")]
pub fn opaque_theme_tint(theme: String, dark: bool) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::opaque_theme_tint(theme, dark)?;
        crate::js::to_json(&out)
    })
}

/// `themeLuminance`.
#[napi(js_name = "themeLuminance")]
pub fn theme_luminance(theme: String, dark: bool) -> napi::Result<f64> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::theme_luminance(theme, dark)?;
        Ok(out)
    })
}

/// `readableDark`.
#[napi(js_name = "readableDark")]
pub fn readable_dark(theme: String, dark: bool) -> napi::Result<bool> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::readable_dark(theme, dark)?;
        Ok(out)
    })
}

/// `stopLightness`.
#[napi(js_name = "stopLightness")]
pub fn stop_lightness(stop: String) -> napi::Result<f64> {
    run(move || {
        let stop = crate::js::from_json(&stop)?;
        let out = port::stop_lightness(stop)?;
        Ok(out)
    })
}

/// `withStopAt`.
#[napi(js_name = "withStopAt")]
pub fn with_stop_at(theme: String, index: f64, x: f64, y: f64) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::with_stop_at(theme, index, x, y)?;
        crate::js::to_json(&out)
    })
}

/// `withStopLightness`.
#[napi(js_name = "withStopLightness")]
pub fn with_stop_lightness(theme: String, index: f64, lightness: f64) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::with_stop_lightness(theme, index, lightness)?;
        crate::js::to_json(&out)
    })
}

/// `withAngle`.
#[napi(js_name = "withAngle")]
pub fn with_angle(theme: String, degrees: f64) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::with_angle(theme, degrees)?;
        crate::js::to_json(&out)
    })
}

/// `withAddedStop`.
#[napi(js_name = "withAddedStop")]
pub fn with_added_stop(theme: String) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::with_added_stop(theme)?;
        crate::js::to_json(&out)
    })
}

/// `withoutStop`.
#[napi(js_name = "withoutStop")]
pub fn without_stop(theme: String, index: f64) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::without_stop(theme, index)?;
        crate::js::to_json(&out)
    })
}

/// `withTexture`.
#[napi(js_name = "withTexture")]
pub fn with_texture(theme: String, texture: String) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let texture = crate::js::from_json(&texture)?;
        let out = port::with_texture(theme, texture)?;
        crate::js::to_json(&out)
    })
}

/// `startingTheme`.
#[napi(js_name = "startingTheme")]
pub fn starting_theme(swatch: String) -> napi::Result<String> {
    run(move || {
        let out = port::starting_theme(swatch)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeTheme`.
#[napi(js_name = "sanitizeTheme")]
pub fn sanitize_theme(v: String) -> napi::Result<String> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::sanitize_theme(v)?;
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

static S_PAD_COLOR_2: wire::Schema = wire::Schema::Dyn;
static S_PAD_COLOR_OUT: wire::Schema = wire::Schema::Obj(&[
    ("r", wire::Schema::Num),
    ("g", wire::Schema::Num),
    ("b", wire::Schema::Num),
    ("a", wire::Schema::Num),
]);

/// `padColor` over the binary wire.
#[napi(js_name = "padColorWire")]
pub fn pad_color_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let x: f64 = r.f64()?;
        let y: f64 = r.f64()?;
        let lightness: serde_json::Value = wire::read_value(r, &S_PAD_COLOR_2)?;
        let out = &port::pad_color(x, y, lightness)?;
        wire::write_value(w, out, &S_PAD_COLOR_OUT)?;
        Ok(())
    })
}

static S_PAD_POSITION_0: wire::Schema = wire::Schema::Dyn;
static S_PAD_POSITION_OUT: wire::Schema =
    wire::Schema::Obj(&[("x", wire::Schema::Num), ("y", wire::Schema::Num)]);

/// `padPosition` over the binary wire.
#[napi(js_name = "padPositionWire")]
pub fn pad_position_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let color: serde_json::Value = wire::read_value(r, &S_PAD_POSITION_0)?;
        let out = &port::pad_position(color)?;
        wire::write_value(w, out, &S_PAD_POSITION_OUT)?;
        Ok(())
    })
}

static S_OKLAB_0: wire::Schema = wire::Schema::Dyn;
static S_OKLAB_OUT: wire::Schema = wire::Schema::Dyn;

/// `oklab` over the binary wire.
#[napi(js_name = "oklabWire")]
pub fn oklab_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let c: serde_json::Value = wire::read_value(r, &S_OKLAB_0)?;
        let out = &port::oklab(c)?;
        wire::write_value(w, out, &S_OKLAB_OUT)?;
        Ok(())
    })
}

static S_FROM_OKLAB_0: wire::Schema = wire::Schema::Dyn;
static S_FROM_OKLAB_OUT: wire::Schema = wire::Schema::Dyn;

/// `fromOklab` over the binary wire.
#[napi(js_name = "fromOklabWire")]
pub fn from_oklab_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: serde_json::Value = wire::read_value(r, &S_FROM_OKLAB_0)?;
        let out = &port::from_oklab(arg0)?;
        wire::write_value(w, out, &S_FROM_OKLAB_OUT)?;
        Ok(())
    })
}

static S_COLOR_HEX_0: wire::Schema = wire::Schema::Dyn;

/// `colorHex` over the binary wire.
#[napi(js_name = "colorHexWire")]
pub fn color_hex_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let color: serde_json::Value = wire::read_value(r, &S_COLOR_HEX_0)?;
        let out = &port::color_hex(color)?;
        w.string16(out);
        Ok(())
    })
}

static S_HEX_COLOR_OUT: wire::Schema = wire::Schema::Obj(&[
    ("r", wire::Schema::Num),
    ("g", wire::Schema::Num),
    ("b", wire::Schema::Num),
    ("a", wire::Schema::Num),
]);

/// `hexColor` over the binary wire.
#[napi(js_name = "hexColorWire")]
pub fn hex_color_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let hex: String = r.string16()?;
        let out = &port::hex_color(hex)?;
        wire::write_value(w, out, &S_HEX_COLOR_OUT)?;
        Ok(())
    })
}

static S_MIX_OKLAB_0: wire::Schema = wire::Schema::Dyn;
static S_MIX_OKLAB_1: wire::Schema = wire::Schema::Dyn;
static S_MIX_OKLAB_OUT: wire::Schema = wire::Schema::Dyn;

/// `mixOklab` over the binary wire.
#[napi(js_name = "mixOklabWire")]
pub fn mix_oklab_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: serde_json::Value = wire::read_value(r, &S_MIX_OKLAB_0)?;
        let b: serde_json::Value = wire::read_value(r, &S_MIX_OKLAB_1)?;
        let t: f64 = r.f64()?;
        let out = &port::mix_oklab(a, b, t)?;
        wire::write_value(w, out, &S_MIX_OKLAB_OUT)?;
        Ok(())
    })
}

static S_LOOK_ANGLE_0: wire::Schema = wire::Schema::Dyn;

/// `lookAngle` over the binary wire.
#[napi(js_name = "lookAngleWire")]
pub fn look_angle_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_LOOK_ANGLE_0)?;
        let out = &port::look_angle(theme)?;
        w.f64(*out);
        Ok(())
    })
}

static S_THEME_AREA_0: wire::Schema = wire::Schema::Dyn;
static S_THEME_AREA_OUT: wire::Schema = wire::Schema::Dyn;

/// `themeArea` over the binary wire.
#[napi(js_name = "themeAreaWire")]
pub fn theme_area_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_AREA_0)?;
        let x: f64 = r.f64()?;
        let y: f64 = r.f64()?;
        let width: f64 = r.f64()?;
        let height: f64 = r.f64()?;
        let out = &port::theme_area(theme, x, y, width, height)?;
        wire::write_value(w, out, &S_THEME_AREA_OUT)?;
        Ok(())
    })
}

static S_THEME_DARK_0: wire::Schema = wire::Schema::Dyn;

/// `themeDark` over the binary wire.
#[napi(js_name = "themeDarkWire")]
pub fn theme_dark_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_DARK_0)?;
        let app_dark: bool = r.bool()?;
        let out = &port::theme_dark(theme, app_dark)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_THEME_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_THEME_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
    ("mode", wire::Schema::Str16),
    (
        "dark",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "stops",
                wire::Schema::List(&wire::Schema::Obj(&[
                    (
                        "color",
                        wire::Schema::Obj(&[
                            ("r", wire::Schema::Num),
                            ("g", wire::Schema::Num),
                            ("b", wire::Schema::Num),
                            ("a", wire::Schema::Num),
                        ]),
                    ),
                    ("x", wire::Schema::Num),
                    ("y", wire::Schema::Num),
                ])),
            ),
            ("intensity", wire::Schema::Num),
            ("grain", wire::Schema::Num),
            ("texture", wire::Schema::Dyn),
            ("translucency", wire::Schema::Num),
            ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
]);

/// `resolveTheme` over the binary wire.
#[napi(js_name = "resolveThemeWire")]
pub fn resolve_theme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_RESOLVE_THEME_0)?;
        let dark: bool = r.bool()?;
        let out = &port::resolve_theme(theme, dark)?;
        wire::write_value(w, out, &S_RESOLVE_THEME_OUT)?;
        Ok(())
    })
}

static S_DARKENED_LOOK_0: wire::Schema = wire::Schema::Dyn;
static S_DARKENED_LOOK_1: wire::Schema = wire::Schema::Dyn;
static S_DARKENED_LOOK_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
    ("mode", wire::Schema::Str16),
    (
        "dark",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "stops",
                wire::Schema::List(&wire::Schema::Obj(&[
                    (
                        "color",
                        wire::Schema::Obj(&[
                            ("r", wire::Schema::Num),
                            ("g", wire::Schema::Num),
                            ("b", wire::Schema::Num),
                            ("a", wire::Schema::Num),
                        ]),
                    ),
                    ("x", wire::Schema::Num),
                    ("y", wire::Schema::Num),
                ])),
            ),
            ("intensity", wire::Schema::Num),
            ("grain", wire::Schema::Num),
            ("texture", wire::Schema::Dyn),
            ("translucency", wire::Schema::Num),
            ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
]);

/// `darkenedLook` over the binary wire.
#[napi(js_name = "darkenedLookWire")]
pub fn darkened_look_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_DARKENED_LOOK_0)?;
        let keep: serde_json::Value = wire::read_value(r, &S_DARKENED_LOOK_1)?;
        let out = &port::darkened_look(theme, keep)?;
        wire::write_value(w, out, &S_DARKENED_LOOK_OUT)?;
        Ok(())
    })
}

static S_REQUESTED_DARK_FOR_0: wire::Schema = wire::Schema::Dyn;

/// `requestedDarkFor` over the binary wire.
#[napi(js_name = "requestedDarkForWire")]
pub fn requested_dark_for_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_REQUESTED_DARK_FOR_0)?;
        let app_dark: bool = r.bool()?;
        let source: String = r.string16()?;
        let out = &port::requested_dark_for(theme, app_dark, source)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOWN_LOOK_0: wire::Schema = wire::Schema::Dyn;
static S_SHOWN_LOOK_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "look",
        wire::Schema::Obj(&[
            (
                "stops",
                wire::Schema::List(&wire::Schema::Obj(&[
                    (
                        "color",
                        wire::Schema::Obj(&[
                            ("r", wire::Schema::Num),
                            ("g", wire::Schema::Num),
                            ("b", wire::Schema::Num),
                            ("a", wire::Schema::Num),
                        ]),
                    ),
                    ("x", wire::Schema::Num),
                    ("y", wire::Schema::Num),
                ])),
            ),
            ("intensity", wire::Schema::Num),
            ("grain", wire::Schema::Num),
            ("texture", wire::Schema::Dyn),
            ("translucency", wire::Schema::Num),
            ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
            ("mode", wire::Schema::Str16),
            (
                "dark",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    (
                        "stops",
                        wire::Schema::List(&wire::Schema::Obj(&[
                            (
                                "color",
                                wire::Schema::Obj(&[
                                    ("r", wire::Schema::Dyn),
                                    ("g", wire::Schema::Dyn),
                                    ("b", wire::Schema::Dyn),
                                    ("a", wire::Schema::Dyn),
                                ]),
                            ),
                            ("x", wire::Schema::Num),
                            ("y", wire::Schema::Num),
                        ])),
                    ),
                    ("intensity", wire::Schema::Num),
                    ("grain", wire::Schema::Num),
                    ("texture", wire::Schema::Dyn),
                    ("translucency", wire::Schema::Num),
                    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
                ])),
            ),
        ]),
    ),
    ("requestedDark", wire::Schema::Bool),
    ("dark", wire::Schema::Bool),
]);

/// `shownLook` over the binary wire.
#[napi(js_name = "shownLookWire")]
pub fn shown_look_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_SHOWN_LOOK_0)?;
        let app_dark: bool = r.bool()?;
        let source: String = r.string16()?;
        let out = &port::shown_look(theme, app_dark, source)?;
        wire::write_value(w, out, &S_SHOWN_LOOK_OUT)?;
        Ok(())
    })
}

/// `arcLayerAlpha` over the binary wire.
#[napi(js_name = "arcLayerAlphaWire")]
pub fn arc_layer_alpha_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let intensity: f64 = r.f64()?;
        let out = &port::arc_layer_alpha(intensity)?;
        w.f64(*out);
        Ok(())
    })
}

static S_THEME_LAYER_ALPHA_0: wire::Schema = wire::Schema::Dyn;

/// `themeLayerAlpha` over the binary wire.
#[napi(js_name = "themeLayerAlphaWire")]
pub fn theme_layer_alpha_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_LAYER_ALPHA_0)?;
        let out = &port::theme_layer_alpha(theme)?;
        w.f64(*out);
        Ok(())
    })
}

static S_THEME_UNDERLAY_ALPHA_0: wire::Schema = wire::Schema::Dyn;

/// `themeUnderlayAlpha` over the binary wire.
#[napi(js_name = "themeUnderlayAlphaWire")]
pub fn theme_underlay_alpha_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_UNDERLAY_ALPHA_0)?;
        let dark: bool = r.bool()?;
        let out = &port::theme_underlay_alpha(theme, dark)?;
        w.f64(*out);
        Ok(())
    })
}

static S_THEME_GRAIN_ALPHA_0: wire::Schema = wire::Schema::Dyn;

/// `themeGrainAlpha` over the binary wire.
#[napi(js_name = "themeGrainAlphaWire")]
pub fn theme_grain_alpha_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_GRAIN_ALPHA_0)?;
        let out = &port::theme_grain_alpha(theme)?;
        w.f64(*out);
        Ok(())
    })
}

static S_FLAT_COLORS_0: wire::Schema = wire::Schema::Dyn;

/// `flatColors` over the binary wire.
#[napi(js_name = "flatColorsWire")]
pub fn flat_colors_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_FLAT_COLORS_0)?;
        let out = &port::flat_colors(theme)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.f64(*x0);
        }
        Ok(())
    })
}

static S_THEME_ACCENT_0: wire::Schema = wire::Schema::Dyn;

/// `themeAccent` over the binary wire.
#[napi(js_name = "themeAccentWire")]
pub fn theme_accent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_ACCENT_0)?;
        let out = &port::theme_accent(theme)?;
        w.string16(out);
        Ok(())
    })
}

static S_THEME_PALETTE_0: wire::Schema = wire::Schema::Dyn;

/// `themePalette` over the binary wire.
#[napi(js_name = "themePaletteWire")]
pub fn theme_palette_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_PALETTE_0)?;
        let out = &port::theme_palette(theme)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

/// `backdropUnderColor` over the binary wire.
#[napi(js_name = "backdropUnderColorWire")]
pub fn backdrop_under_color_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let underlay_alpha: f64 = r.f64()?;
        let dark: bool = r.bool()?;
        let out = &port::backdrop_under_color(underlay_alpha, dark)?;
        w.string16(out);
        Ok(())
    })
}

static S_THEME_UNDER_COLOR_0: wire::Schema = wire::Schema::Dyn;

/// `themeUnderColor` over the binary wire.
#[napi(js_name = "themeUnderColorWire")]
pub fn theme_under_color_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_UNDER_COLOR_0)?;
        let dark: bool = r.bool()?;
        let out = &port::theme_under_color(theme, dark)?;
        w.string16(out);
        Ok(())
    })
}

static S_OPAQUE_THEME_TINT_0: wire::Schema = wire::Schema::Dyn;
static S_OPAQUE_THEME_TINT_OUT: wire::Schema = wire::Schema::Dyn;

/// `opaqueThemeTint` over the binary wire.
#[napi(js_name = "opaqueThemeTintWire")]
pub fn opaque_theme_tint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_OPAQUE_THEME_TINT_0)?;
        let dark: bool = r.bool()?;
        let out = &port::opaque_theme_tint(theme, dark)?;
        wire::write_value(w, out, &S_OPAQUE_THEME_TINT_OUT)?;
        Ok(())
    })
}

static S_THEME_LUMINANCE_0: wire::Schema = wire::Schema::Dyn;

/// `themeLuminance` over the binary wire.
#[napi(js_name = "themeLuminanceWire")]
pub fn theme_luminance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_THEME_LUMINANCE_0)?;
        let dark: bool = r.bool()?;
        let out = &port::theme_luminance(theme, dark)?;
        w.f64(*out);
        Ok(())
    })
}

static S_READABLE_DARK_0: wire::Schema = wire::Schema::Dyn;

/// `readableDark` over the binary wire.
#[napi(js_name = "readableDarkWire")]
pub fn readable_dark_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_READABLE_DARK_0)?;
        let dark: bool = r.bool()?;
        let out = &port::readable_dark(theme, dark)?;
        w.bool(*out);
        Ok(())
    })
}

static S_STOP_LIGHTNESS_0: wire::Schema = wire::Schema::Dyn;

/// `stopLightness` over the binary wire.
#[napi(js_name = "stopLightnessWire")]
pub fn stop_lightness_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let stop: serde_json::Value = wire::read_value(r, &S_STOP_LIGHTNESS_0)?;
        let out = &port::stop_lightness(stop)?;
        w.f64(*out);
        Ok(())
    })
}

static S_WITH_STOP_AT_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_STOP_AT_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
]);

/// `withStopAt` over the binary wire.
#[napi(js_name = "withStopAtWire")]
pub fn with_stop_at_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_WITH_STOP_AT_0)?;
        let index: f64 = r.f64()?;
        let x: f64 = r.f64()?;
        let y: f64 = r.f64()?;
        let out = &port::with_stop_at(theme, index, x, y)?;
        wire::write_value(w, out, &S_WITH_STOP_AT_OUT)?;
        Ok(())
    })
}

static S_WITH_STOP_LIGHTNESS_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_STOP_LIGHTNESS_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
]);

/// `withStopLightness` over the binary wire.
#[napi(js_name = "withStopLightnessWire")]
pub fn with_stop_lightness_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_WITH_STOP_LIGHTNESS_0)?;
        let index: f64 = r.f64()?;
        let lightness: f64 = r.f64()?;
        let out = &port::with_stop_lightness(theme, index, lightness)?;
        wire::write_value(w, out, &S_WITH_STOP_LIGHTNESS_OUT)?;
        Ok(())
    })
}

static S_WITH_ANGLE_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_ANGLE_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
]);

/// `withAngle` over the binary wire.
#[napi(js_name = "withAngleWire")]
pub fn with_angle_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_WITH_ANGLE_0)?;
        let degrees: f64 = r.f64()?;
        let out = &port::with_angle(theme, degrees)?;
        wire::write_value(w, out, &S_WITH_ANGLE_OUT)?;
        Ok(())
    })
}

static S_WITH_ADDED_STOP_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_ADDED_STOP_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
]);

/// `withAddedStop` over the binary wire.
#[napi(js_name = "withAddedStopWire")]
pub fn with_added_stop_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_WITH_ADDED_STOP_0)?;
        let out = &port::with_added_stop(theme)?;
        wire::write_value(w, out, &S_WITH_ADDED_STOP_OUT)?;
        Ok(())
    })
}

static S_WITHOUT_STOP_0: wire::Schema = wire::Schema::Dyn;
static S_WITHOUT_STOP_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
]);

/// `withoutStop` over the binary wire.
#[napi(js_name = "withoutStopWire")]
pub fn without_stop_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_WITHOUT_STOP_0)?;
        let index: f64 = r.f64()?;
        let out = &port::without_stop(theme, index)?;
        wire::write_value(w, out, &S_WITHOUT_STOP_OUT)?;
        Ok(())
    })
}

static S_WITH_TEXTURE_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_TEXTURE_1: wire::Schema = wire::Schema::Dyn;
static S_WITH_TEXTURE_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
]);

/// `withTexture` over the binary wire.
#[napi(js_name = "withTextureWire")]
pub fn with_texture_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_WITH_TEXTURE_0)?;
        let texture: serde_json::Value = wire::read_value(r, &S_WITH_TEXTURE_1)?;
        let out = &port::with_texture(theme, texture)?;
        wire::write_value(w, out, &S_WITH_TEXTURE_OUT)?;
        Ok(())
    })
}

static S_STARTING_THEME_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
    ("mode", wire::Schema::Str16),
    (
        "dark",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "stops",
                wire::Schema::List(&wire::Schema::Obj(&[
                    (
                        "color",
                        wire::Schema::Obj(&[
                            ("r", wire::Schema::Num),
                            ("g", wire::Schema::Num),
                            ("b", wire::Schema::Num),
                            ("a", wire::Schema::Num),
                        ]),
                    ),
                    ("x", wire::Schema::Num),
                    ("y", wire::Schema::Num),
                ])),
            ),
            ("intensity", wire::Schema::Num),
            ("grain", wire::Schema::Num),
            ("texture", wire::Schema::Dyn),
            ("translucency", wire::Schema::Num),
            ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
]);

/// `startingTheme` over the binary wire.
#[napi(js_name = "startingThemeWire")]
pub fn starting_theme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let swatch: String = r.string16()?;
        let out = &port::starting_theme(swatch)?;
        wire::write_value(w, out, &S_STARTING_THEME_OUT)?;
        Ok(())
    })
}

static S_SANITIZE_THEME_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_THEME_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    (
        "stops",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "color",
                wire::Schema::Obj(&[
                    ("r", wire::Schema::Num),
                    ("g", wire::Schema::Num),
                    ("b", wire::Schema::Num),
                    ("a", wire::Schema::Num),
                ]),
            ),
            ("x", wire::Schema::Num),
            ("y", wire::Schema::Num),
        ])),
    ),
    ("intensity", wire::Schema::Num),
    ("grain", wire::Schema::Num),
    ("texture", wire::Schema::Dyn),
    ("translucency", wire::Schema::Num),
    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
    ("mode", wire::Schema::Str16),
    (
        "dark",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "stops",
                wire::Schema::List(&wire::Schema::Obj(&[
                    (
                        "color",
                        wire::Schema::Obj(&[
                            ("r", wire::Schema::Num),
                            ("g", wire::Schema::Num),
                            ("b", wire::Schema::Num),
                            ("a", wire::Schema::Num),
                        ]),
                    ),
                    ("x", wire::Schema::Num),
                    ("y", wire::Schema::Num),
                ])),
            ),
            ("intensity", wire::Schema::Num),
            ("grain", wire::Schema::Num),
            ("texture", wire::Schema::Dyn),
            ("translucency", wire::Schema::Num),
            ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
]));

/// `sanitizeTheme` over the binary wire.
#[napi(js_name = "sanitizeThemeWire")]
pub fn sanitize_theme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: serde_json::Value = wire::read_value(r, &S_SANITIZE_THEME_0)?;
        let out = &port::sanitize_theme(v)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_SANITIZE_THEME_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}
