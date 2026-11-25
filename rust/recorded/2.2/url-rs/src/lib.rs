//! JS bindings for `apps/browser/src/lib/url.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `withoutFragment`.
#[napi(js_name = "withoutFragment")]
pub fn without_fragment(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::without_fragment(url)?;
        crate::js::to_json(&out)
    })
}

/// `withoutUserInfo`.
#[napi(js_name = "withoutUserInfo")]
pub fn without_user_info(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::without_user_info(url)?;
        crate::js::to_json(&out)
    })
}

/// `samePage`.
#[napi(js_name = "samePage")]
pub fn same_page(a: String, b: String) -> napi::Result<String> {
    run(move || {
        let out = port::same_page(a, b)?;
        crate::js::to_json(&out)
    })
}

/// `isWebPageUrl`.
#[napi(js_name = "isWebPageUrl")]
pub fn is_web_page_url(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::is_web_page_url(url)?;
        crate::js::to_json(&out)
    })
}

/// `displayHostOf`.
#[napi(js_name = "displayHostOf")]
pub fn display_host_of(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::display_host_of(url)?;
        crate::js::to_json(&out)
    })
}

/// `pageHostLabel`.
#[napi(js_name = "pageHostLabel")]
pub fn page_host_label(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::page_host_label(url)?;
        crate::js::to_json(&out)
    })
}

/// `urlTrail`.
#[napi(js_name = "urlTrail")]
pub fn url_trail(url: String, title: String, arg2: String) -> napi::Result<String> {
    run(move || {
        let arg2 = crate::js::from_json(&arg2)?;
        let out = port::url_trail(url, title, arg2)?;
        Ok(out)
    })
}

/// `hostOrOrigin`.
#[napi(js_name = "hostOrOrigin")]
pub fn host_or_origin(origin: String) -> napi::Result<String> {
    run(move || {
        let out = port::host_or_origin(origin)?;
        crate::js::to_json(&out)
    })
}

/// `originForSecurityDisplay`.
#[napi(js_name = "originForSecurityDisplay")]
pub fn origin_for_security_display(origin: String) -> napi::Result<String> {
    run(move || {
        let out = port::origin_for_security_display(origin)?;
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

static S_WITHOUT_FRAGMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `withoutFragment` over the binary wire.
#[napi(js_name = "withoutFragmentWire")]
pub fn without_fragment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::without_fragment(url)?;
        wire::write_value(w, out, &S_WITHOUT_FRAGMENT_OUT)?;
        Ok(())
    })
}

static S_WITHOUT_USER_INFO_OUT: wire::Schema = wire::Schema::Dyn;

/// `withoutUserInfo` over the binary wire.
#[napi(js_name = "withoutUserInfoWire")]
pub fn without_user_info_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::without_user_info(url)?;
        wire::write_value(w, out, &S_WITHOUT_USER_INFO_OUT)?;
        Ok(())
    })
}

static S_SAME_PAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `samePage` over the binary wire.
#[napi(js_name = "samePageWire")]
pub fn same_page_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: String = r.string16()?;
        let b: String = r.string16()?;
        let out = &port::same_page(a, b)?;
        wire::write_value(w, out, &S_SAME_PAGE_OUT)?;
        Ok(())
    })
}

static S_IS_WEB_PAGE_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `isWebPageUrl` over the binary wire.
#[napi(js_name = "isWebPageUrlWire")]
pub fn is_web_page_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::is_web_page_url(url)?;
        wire::write_value(w, out, &S_IS_WEB_PAGE_URL_OUT)?;
        Ok(())
    })
}

static S_DISPLAY_HOST_OF_OUT: wire::Schema = wire::Schema::Dyn;

/// `displayHostOf` over the binary wire.
#[napi(js_name = "displayHostOfWire")]
pub fn display_host_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::display_host_of(url)?;
        wire::write_value(w, out, &S_DISPLAY_HOST_OF_OUT)?;
        Ok(())
    })
}

static S_PAGE_HOST_LABEL_OUT: wire::Schema = wire::Schema::Dyn;

/// `pageHostLabel` over the binary wire.
#[napi(js_name = "pageHostLabelWire")]
pub fn page_host_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::page_host_label(url)?;
        wire::write_value(w, out, &S_PAGE_HOST_LABEL_OUT)?;
        Ok(())
    })
}

static S_URL_TRAIL_2: wire::Schema = wire::Schema::Dyn;

/// `urlTrail` over the binary wire.
#[napi(js_name = "urlTrailWire")]
pub fn url_trail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let title: String = r.string16()?;
        let arg2: serde_json::Value = wire::read_value(r, &S_URL_TRAIL_2)?;
        let out = &port::url_trail(url, title, arg2)?;
        w.string16(out);
        Ok(())
    })
}

static S_HOST_OR_ORIGIN_OUT: wire::Schema = wire::Schema::Dyn;

/// `hostOrOrigin` over the binary wire.
#[napi(js_name = "hostOrOriginWire")]
pub fn host_or_origin_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let origin: String = r.string16()?;
        let out = &port::host_or_origin(origin)?;
        wire::write_value(w, out, &S_HOST_OR_ORIGIN_OUT)?;
        Ok(())
    })
}

/// `originForSecurityDisplay` over the binary wire.
#[napi(js_name = "originForSecurityDisplayWire")]
pub fn origin_for_security_display_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let origin: String = r.string16()?;
        let out = &port::origin_for_security_display(origin)?;
        w.string16(out);
        Ok(())
    })
}
