//! JS bindings for `apps/browser/src/lib/windowLayout.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `sidebarWidthLimit`.
#[napi(js_name = "sidebarWidthLimit")]
pub fn sidebar_width_limit(window_width: Option<f64>) -> napi::Result<f64> {
    run(move || {
        let out = port::sidebar_width_limit(window_width)?;
        Ok(out)
    })
}

/// `shownSidebarWidth`.
#[napi(js_name = "shownSidebarWidth")]
pub fn shown_sidebar_width(saved: f64, window_width: Option<f64>) -> napi::Result<f64> {
    run(move || {
        let out = port::shown_sidebar_width(saved, window_width)?;
        Ok(out)
    })
}

/// `validPageFrame`.
#[napi(js_name = "validPageFrame")]
pub fn valid_page_frame(v: String) -> napi::Result<bool> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::valid_page_frame(v)?;
        Ok(out)
    })
}

/// `pageFrameInsetOf`.
#[napi(js_name = "pageFrameInsetOf")]
pub fn page_frame_inset_of(inset: f64, opts: String) -> napi::Result<f64> {
    run(move || {
        let opts = crate::js::from_json(&opts)?;
        let out = port::page_frame_inset_of(inset, opts)?;
        Ok(out)
    })
}

/// `pageFrameInset`.
#[napi(js_name = "pageFrameInset")]
pub fn page_frame_inset(settings: String) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let out = port::page_frame_inset(settings)?;
        crate::js::to_json(&out)
    })
}

/// `pageFrameOf`.
#[napi(js_name = "pageFrameOf")]
pub fn page_frame_of(
    inset: f64,
    window_radius: f64,
    drawn_window_radius: String,
) -> napi::Result<String> {
    run(move || {
        let drawn_window_radius = crate::js::from_json(&drawn_window_radius)?;
        let out = port::page_frame_of(inset, window_radius, drawn_window_radius)?;
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

/// `sidebarWidthLimit` over the binary wire.
#[napi(js_name = "sidebarWidthLimitWire")]
pub fn sidebar_width_limit_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let window_width: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::sidebar_width_limit(window_width)?;
        w.f64(*out);
        Ok(())
    })
}

/// `shownSidebarWidth` over the binary wire.
#[napi(js_name = "shownSidebarWidthWire")]
pub fn shown_sidebar_width_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let saved: f64 = r.f64()?;
        let window_width: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::shown_sidebar_width(saved, window_width)?;
        w.f64(*out);
        Ok(())
    })
}

static S_VALID_PAGE_FRAME_0: wire::Schema = wire::Schema::Dyn;

/// `validPageFrame` over the binary wire.
#[napi(js_name = "validPageFrameWire")]
pub fn valid_page_frame_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: serde_json::Value = wire::read_value(r, &S_VALID_PAGE_FRAME_0)?;
        let out = &port::valid_page_frame(v)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_PAGE_FRAME_INSET_OF_1: wire::Schema = wire::Schema::Dyn;

/// `pageFrameInsetOf` over the binary wire.
#[napi(js_name = "pageFrameInsetOfWire")]
pub fn page_frame_inset_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let inset: f64 = r.f64()?;
        let opts: serde_json::Value = wire::read_value(r, &S_PAGE_FRAME_INSET_OF_1)?;
        let out = &port::page_frame_inset_of(inset, opts)?;
        w.f64(*out);
        Ok(())
    })
}

static S_PAGE_FRAME_INSET_0: wire::Schema = wire::Schema::Dyn;
static S_PAGE_FRAME_INSET_OUT: wire::Schema = wire::Schema::Num;

/// `pageFrameInset` over the binary wire.
#[napi(js_name = "pageFrameInsetWire")]
pub fn page_frame_inset_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: serde_json::Value = wire::read_value(r, &S_PAGE_FRAME_INSET_0)?;
        let out = &port::page_frame_inset(settings)?;
        wire::write_value(w, out, &S_PAGE_FRAME_INSET_OUT)?;
        Ok(())
    })
}

static S_PAGE_FRAME_OF_2: wire::Schema = wire::Schema::Dyn;
static S_PAGE_FRAME_OF_OUT: wire::Schema = wire::Schema::Obj(&[
    ("inset", wire::Schema::Num),
    ("radius", wire::Schema::Num),
    ("splitGap", wire::Schema::Num),
    (
        "peek",
        wire::Schema::Obj(&[
            ("left", wire::Schema::Num),
            ("top", wire::Schema::Num),
            ("bottom", wire::Schema::Num),
            ("radius", wire::Schema::Num),
        ]),
    ),
]);

/// `pageFrameOf` over the binary wire.
#[napi(js_name = "pageFrameOfWire")]
pub fn page_frame_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let inset: f64 = r.f64()?;
        let window_radius: f64 = r.f64()?;
        let drawn_window_radius: serde_json::Value = wire::read_value(r, &S_PAGE_FRAME_OF_2)?;
        let out = &port::page_frame_of(inset, window_radius, drawn_window_radius)?;
        wire::write_value(w, out, &S_PAGE_FRAME_OF_OUT)?;
        Ok(())
    })
}
