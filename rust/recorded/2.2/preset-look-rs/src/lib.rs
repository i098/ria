//! JS bindings for `apps/browser/src/lib/presetLook.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `presetStartingTheme`.
#[napi(js_name = "presetStartingTheme")]
pub fn preset_starting_theme(swatch: String) -> napi::Result<String> {
    run(move || {
        let out = port::preset_starting_theme(swatch)?;
        crate::js::to_json(&out)
    })
}

/// `presetThemeOf`.
#[napi(js_name = "presetThemeOf")]
pub fn preset_theme_of(profile: String, color: String) -> napi::Result<String> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let color = crate::js::from_json(&color)?;
        let out = port::preset_theme_of(profile, color)?;
        crate::js::to_json(&out)
    })
}

/// `snapToStart`.
#[napi(js_name = "snapToStart")]
pub fn snap_to_start(value: f64, start: f64, track_pixels: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::snap_to_start(value, start, track_pixels)?;
        Ok(out)
    })
}

/// `snapPointToStart`.
#[napi(js_name = "snapPointToStart")]
pub fn snap_point_to_start(point: String, start: String, pad_pixels: f64) -> napi::Result<String> {
    run(move || {
        let point = crate::js::from_json(&point)?;
        let start = crate::js::from_json(&start)?;
        let out = port::snap_point_to_start(point, start, pad_pixels)?;
        crate::js::to_json(&out)
    })
}

/// `isUntouched`.
#[napi(js_name = "isUntouched")]
pub fn is_untouched(theme: String, start: String) -> napi::Result<bool> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let start = crate::js::from_json(&start)?;
        let out = port::is_untouched(theme, start)?;
        Ok(out)
    })
}

/// `sameTheme`.
#[napi(js_name = "sameTheme")]
pub fn same_theme(a: String, b: String) -> napi::Result<bool> {
    run(move || {
        let a = crate::js::from_json(&a)?;
        let b = crate::js::from_json(&b)?;
        let out = port::same_theme(a, b)?;
        Ok(out)
    })
}

/// `withOwnLook`.
#[napi(js_name = "withOwnLook")]
pub fn with_own_look(profile: String, theme: String) -> napi::Result<String> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let theme = crate::js::from_json(&theme)?;
        let out = port::with_own_look(profile, theme)?;
        crate::js::to_json(&out)
    })
}

/// `withPresetChosen`.
#[napi(js_name = "withPresetChosen")]
pub fn with_preset_chosen(profile: String, color: String) -> napi::Result<String> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let color = crate::js::from_json(&color)?;
        let out = port::with_preset_chosen(profile, color)?;
        crate::js::to_json(&out)
    })
}

/// `withSyncedColor`.
#[napi(js_name = "withSyncedColor")]
pub fn with_synced_color(profile: String, color: String, synced: String) -> napi::Result<String> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let color = crate::js::from_json(&color)?;
        let synced = crate::js::from_json(&synced)?;
        let out = port::with_synced_color(profile, color, synced)?;
        crate::js::to_json(&out)
    })
}

/// `editedPresetCount`.
#[napi(js_name = "editedPresetCount")]
pub fn edited_preset_count(profile: String) -> napi::Result<f64> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let out = port::edited_preset_count(profile)?;
        Ok(out)
    })
}

/// `withOwnLooksSeeded`.
#[napi(js_name = "withOwnLooksSeeded")]
pub fn with_own_looks_seeded(profile: String) -> napi::Result<String> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let out = port::with_own_looks_seeded(profile)?;
        crate::js::to_json(&out)
    })
}

/// `profilesWithOwnLooks`.
#[napi(js_name = "profilesWithOwnLooks")]
pub fn profiles_with_own_looks(profiles: String) -> napi::Result<String> {
    run(move || {
        let profiles = crate::js::from_json(&profiles)?;
        let out = port::profiles_with_own_looks(profiles)?;
        crate::js::to_json(&out)
    })
}

/// `trackValueAt`.
#[napi(js_name = "trackValueAt")]
pub fn track_value_at(
    page_x: f64,
    track_x: f64,
    track_pixels: f64,
    snap_to: Option<f64>,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let out = port::track_value_at(page_x, track_x, track_pixels, snap_to)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
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

static S_PRESET_STARTING_THEME_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `presetStartingTheme` over the binary wire.
#[napi(js_name = "presetStartingThemeWire")]
pub fn preset_starting_theme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let swatch: String = r.string16()?;
        let out = &port::preset_starting_theme(swatch)?;
        wire::write_value(w, out, &S_PRESET_STARTING_THEME_OUT)?;
        Ok(())
    })
}

static S_PRESET_THEME_OF_0: wire::Schema = wire::Schema::Dyn;
static S_PRESET_THEME_OF_1: wire::Schema = wire::Schema::Dyn;
static S_PRESET_THEME_OF_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
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

/// `presetThemeOf` over the binary wire.
#[napi(js_name = "presetThemeOfWire")]
pub fn preset_theme_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: serde_json::Value = wire::read_value(r, &S_PRESET_THEME_OF_0)?;
        let color: serde_json::Value = wire::read_value(r, &S_PRESET_THEME_OF_1)?;
        let out = &port::preset_theme_of(profile, color)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PRESET_THEME_OF_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `snapToStart` over the binary wire.
#[napi(js_name = "snapToStartWire")]
pub fn snap_to_start_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: f64 = r.f64()?;
        let start: f64 = r.f64()?;
        let track_pixels: f64 = r.f64()?;
        let out = &port::snap_to_start(value, start, track_pixels)?;
        w.f64(*out);
        Ok(())
    })
}

static S_SNAP_POINT_TO_START_0: wire::Schema = wire::Schema::Dyn;
static S_SNAP_POINT_TO_START_1: wire::Schema = wire::Schema::Dyn;
static S_SNAP_POINT_TO_START_OUT: wire::Schema =
    wire::Schema::Obj(&[("x", wire::Schema::Num), ("y", wire::Schema::Num)]);

/// `snapPointToStart` over the binary wire.
#[napi(js_name = "snapPointToStartWire")]
pub fn snap_point_to_start_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let point: serde_json::Value = wire::read_value(r, &S_SNAP_POINT_TO_START_0)?;
        let start: serde_json::Value = wire::read_value(r, &S_SNAP_POINT_TO_START_1)?;
        let pad_pixels: f64 = r.f64()?;
        let out = &port::snap_point_to_start(point, start, pad_pixels)?;
        wire::write_value(w, out, &S_SNAP_POINT_TO_START_OUT)?;
        Ok(())
    })
}

static S_IS_UNTOUCHED_0: wire::Schema = wire::Schema::Dyn;
static S_IS_UNTOUCHED_1: wire::Schema = wire::Schema::Dyn;

/// `isUntouched` over the binary wire.
#[napi(js_name = "isUntouchedWire")]
pub fn is_untouched_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_IS_UNTOUCHED_0)?;
        let start: serde_json::Value = wire::read_value(r, &S_IS_UNTOUCHED_1)?;
        let out = &port::is_untouched(theme, start)?;
        w.bool(*out);
        Ok(())
    })
}

/// `sameTheme` over the binary wire.
#[napi(js_name = "sameThemeWire")]
pub fn same_theme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let b: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::same_theme(a, b)?;
        w.bool(*out);
        Ok(())
    })
}

static S_WITH_OWN_LOOK_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_OWN_LOOK_1: wire::Schema = wire::Schema::Dyn;
static S_WITH_OWN_LOOK_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "theme",
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
            ("mode", wire::Schema::Str16),
            (
                "dark",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    (
                        "stops",
                        wire::Schema::List(&wire::Schema::Obj(&[
                            ("color", wire::Schema::Dyn),
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
        ])),
    ),
    ("presetThemes", wire::Schema::Dyn),
]);

/// `withOwnLook` over the binary wire.
#[napi(js_name = "withOwnLookWire")]
pub fn with_own_look_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: serde_json::Value = wire::read_value(r, &S_WITH_OWN_LOOK_0)?;
        let theme: serde_json::Value = wire::read_value(r, &S_WITH_OWN_LOOK_1)?;
        let out = &port::with_own_look(profile, theme)?;
        wire::write_value(w, out, &S_WITH_OWN_LOOK_OUT)?;
        Ok(())
    })
}

static S_WITH_PRESET_CHOSEN_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_PRESET_CHOSEN_1: wire::Schema = wire::Schema::Dyn;
static S_WITH_PRESET_CHOSEN_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("color", wire::Schema::Dyn),
    (
        "theme",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "stops",
                wire::Schema::List(&wire::Schema::Obj(&[
                    ("color", wire::Schema::Dyn),
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
                            ("color", wire::Schema::Dyn),
                            ("x", wire::Schema::Dyn),
                            ("y", wire::Schema::Dyn),
                        ])),
                    ),
                    ("intensity", wire::Schema::Num),
                    ("grain", wire::Schema::Num),
                    ("texture", wire::Schema::Dyn),
                    ("translucency", wire::Schema::Num),
                    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
                ])),
            ),
        ])),
    ),
]));

/// `withPresetChosen` over the binary wire.
#[napi(js_name = "withPresetChosenWire")]
pub fn with_preset_chosen_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: serde_json::Value = wire::read_value(r, &S_WITH_PRESET_CHOSEN_0)?;
        let color: serde_json::Value = wire::read_value(r, &S_WITH_PRESET_CHOSEN_1)?;
        let out = &port::with_preset_chosen(profile, color)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_WITH_PRESET_CHOSEN_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_WITH_SYNCED_COLOR_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_SYNCED_COLOR_1: wire::Schema = wire::Schema::Dyn;
static S_WITH_SYNCED_COLOR_2: wire::Schema = wire::Schema::Dyn;
static S_WITH_SYNCED_COLOR_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("color", wire::Schema::Dyn),
    (
        "theme",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "stops",
                wire::Schema::List(&wire::Schema::Obj(&[
                    ("color", wire::Schema::Dyn),
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
                            ("color", wire::Schema::Dyn),
                            ("x", wire::Schema::Dyn),
                            ("y", wire::Schema::Dyn),
                        ])),
                    ),
                    ("intensity", wire::Schema::Num),
                    ("grain", wire::Schema::Num),
                    ("texture", wire::Schema::Dyn),
                    ("translucency", wire::Schema::Num),
                    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
                ])),
            ),
        ])),
    ),
]));

/// `withSyncedColor` over the binary wire.
#[napi(js_name = "withSyncedColorWire")]
pub fn with_synced_color_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: serde_json::Value = wire::read_value(r, &S_WITH_SYNCED_COLOR_0)?;
        let color: serde_json::Value = wire::read_value(r, &S_WITH_SYNCED_COLOR_1)?;
        let synced: serde_json::Value = wire::read_value(r, &S_WITH_SYNCED_COLOR_2)?;
        let out = &port::with_synced_color(profile, color, synced)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_WITH_SYNCED_COLOR_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_EDITED_PRESET_COUNT_0: wire::Schema = wire::Schema::Dyn;

/// `editedPresetCount` over the binary wire.
#[napi(js_name = "editedPresetCountWire")]
pub fn edited_preset_count_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: serde_json::Value = wire::read_value(r, &S_EDITED_PRESET_COUNT_0)?;
        let out = &port::edited_preset_count(profile)?;
        w.f64(*out);
        Ok(())
    })
}

static S_WITH_OWN_LOOKS_SEEDED_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_OWN_LOOKS_SEEDED_OUT: wire::Schema = wire::Schema::Obj(&[
    ("color", wire::Schema::Dyn),
    (
        "theme",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "stops",
                wire::Schema::List(&wire::Schema::Obj(&[
                    ("color", wire::Schema::Dyn),
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
                            ("color", wire::Schema::Dyn),
                            ("x", wire::Schema::Dyn),
                            ("y", wire::Schema::Dyn),
                        ])),
                    ),
                    ("intensity", wire::Schema::Num),
                    ("grain", wire::Schema::Num),
                    ("texture", wire::Schema::Dyn),
                    ("translucency", wire::Schema::Num),
                    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
                ])),
            ),
        ])),
    ),
    ("presetThemes", wire::Schema::Dyn),
    (
        "seenTheme",
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
        ])),
    ),
    ("seenColor", wire::Schema::Dyn),
]);

/// `withOwnLooksSeeded` over the binary wire.
#[napi(js_name = "withOwnLooksSeededWire")]
pub fn with_own_looks_seeded_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: serde_json::Value = wire::read_value(r, &S_WITH_OWN_LOOKS_SEEDED_0)?;
        let out = &port::with_own_looks_seeded(profile)?;
        wire::write_value(w, out, &S_WITH_OWN_LOOKS_SEEDED_OUT)?;
        Ok(())
    })
}

static S_PROFILES_WITH_OWN_LOOKS_0: wire::Schema = wire::Schema::Dyn;
static S_PROFILES_WITH_OWN_LOOKS_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Obj(&[
    ("color", wire::Schema::Dyn),
    (
        "theme",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("stops", wire::Schema::List(&wire::Schema::Dyn)),
            ("intensity", wire::Schema::Num),
            ("grain", wire::Schema::Num),
            ("texture", wire::Schema::Dyn),
            ("translucency", wire::Schema::Num),
            ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
            ("mode", wire::Schema::Str16),
            (
                "dark",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("stops", wire::Schema::List(&wire::Schema::Dyn)),
                    ("intensity", wire::Schema::Num),
                    ("grain", wire::Schema::Num),
                    ("texture", wire::Schema::Dyn),
                    ("translucency", wire::Schema::Num),
                    ("angle", wire::Schema::OptJs(&wire::Schema::Num)),
                ])),
            ),
        ])),
    ),
    ("presetThemes", wire::Schema::Dyn),
]));

/// `profilesWithOwnLooks` over the binary wire.
#[napi(js_name = "profilesWithOwnLooksWire")]
pub fn profiles_with_own_looks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profiles: serde_json::Value = wire::read_value(r, &S_PROFILES_WITH_OWN_LOOKS_0)?;
        let out = &port::profiles_with_own_looks(profiles)?;
        wire::write_value(w, out, &S_PROFILES_WITH_OWN_LOOKS_OUT)?;
        Ok(())
    })
}

/// `trackValueAt` over the binary wire.
#[napi(js_name = "trackValueAtWire")]
pub fn track_value_at_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let page_x: f64 = r.f64()?;
        let track_x: f64 = r.f64()?;
        let track_pixels: f64 = r.f64()?;
        let snap_to: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::track_value_at(page_x, track_x, track_pixels, snap_to)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}
