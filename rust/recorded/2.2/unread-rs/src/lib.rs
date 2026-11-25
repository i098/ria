//! JS bindings for `apps/browser/src/lib/unread.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `unreadCount`.
#[napi(js_name = "unreadCount")]
pub fn unread_count(n: f64, more: Option<bool>) -> napi::Result<String> {
    run(move || {
        let out = port::unread_count(n, more)?;
        crate::js::to_json(&out)
    })
}

/// `titleWithoutUnread`.
#[napi(js_name = "titleWithoutUnread")]
pub fn title_without_unread(title: String, unread: String) -> napi::Result<String> {
    run(move || {
        let unread = crate::js::from_json(&unread)?;
        let out = port::title_without_unread(title, unread)?;
        Ok(out)
    })
}

/// `hostOfOrigin`.
#[napi(js_name = "hostOfOrigin")]
pub fn host_of_origin(origin: String) -> napi::Result<String> {
    run(move || {
        let out = port::host_of_origin(origin)?;
        Ok(out)
    })
}

/// `unreadInTitle`.
#[napi(js_name = "unreadInTitle")]
pub fn unread_in_title(raw: String, host: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::unread_in_title(raw, host)?;
        crate::js::to_json(&out)
    })
}

/// `originOf`.
#[napi(js_name = "originOf")]
pub fn origin_of(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::origin_of(url)?;
        Ok(out)
    })
}

/// `faviconIsBadgeNamed`.
#[napi(js_name = "faviconIsBadgeNamed")]
pub fn favicon_is_badge_named(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::favicon_is_badge_named(url)?;
        crate::js::to_json(&out)
    })
}

/// `faviconIsOrdinary`.
#[napi(js_name = "faviconIsOrdinary")]
pub fn favicon_is_ordinary(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::favicon_is_ordinary(url)?;
        crate::js::to_json(&out)
    })
}

/// `faviconLooksBadged`.
#[napi(js_name = "faviconLooksBadged")]
pub fn favicon_looks_badged(home: String, now: String) -> napi::Result<bool> {
    run(move || {
        let out = port::favicon_looks_badged(home, now)?;
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

/// `unreadCount` over the binary wire.
#[napi(js_name = "unreadCountWire")]
pub fn unread_count_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let n: f64 = r.f64()?;
        let more: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::unread_count(n, more)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `titleWithoutUnread` over the binary wire.
#[napi(js_name = "titleWithoutUnreadWire")]
pub fn title_without_unread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let title: String = r.string16()?;
        let unread: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::title_without_unread(title, unread)?;
        w.string16(out);
        Ok(())
    })
}

/// `hostOfOrigin` over the binary wire.
#[napi(js_name = "hostOfOriginWire")]
pub fn host_of_origin_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let origin: String = r.string16()?;
        let out = &port::host_of_origin(origin)?;
        w.string16(out);
        Ok(())
    })
}

/// `unreadInTitle` over the binary wire.
#[napi(js_name = "unreadInTitleWire")]
pub fn unread_in_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let host: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::unread_in_title(raw, host)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `originOf` over the binary wire.
#[napi(js_name = "originOfWire")]
pub fn origin_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::origin_of(url)?;
        w.string16(out);
        Ok(())
    })
}

static S_FAVICON_IS_BADGE_NAMED_OUT: wire::Schema = wire::Schema::Dyn;

/// `faviconIsBadgeNamed` over the binary wire.
#[napi(js_name = "faviconIsBadgeNamedWire")]
pub fn favicon_is_badge_named_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::favicon_is_badge_named(url)?;
        wire::write_value(w, out, &S_FAVICON_IS_BADGE_NAMED_OUT)?;
        Ok(())
    })
}

static S_FAVICON_IS_ORDINARY_OUT: wire::Schema = wire::Schema::Dyn;

/// `faviconIsOrdinary` over the binary wire.
#[napi(js_name = "faviconIsOrdinaryWire")]
pub fn favicon_is_ordinary_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::favicon_is_ordinary(url)?;
        wire::write_value(w, out, &S_FAVICON_IS_ORDINARY_OUT)?;
        Ok(())
    })
}

/// `faviconLooksBadged` over the binary wire.
#[napi(js_name = "faviconLooksBadgedWire")]
pub fn favicon_looks_badged_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home: String = r.string16()?;
        let now: String = r.string16()?;
        let out = &port::favicon_looks_badged(home, now)?;
        w.bool(*out);
        Ok(())
    })
}
