//! JS bindings for `apps/browser/src/lib/urlParts.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `canonicalIpv6`.
#[napi(js_name = "canonicalIpv6")]
pub fn canonical_ipv6(text: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::canonical_ipv6(text)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `asciiHost`.
#[napi(js_name = "asciiHost")]
pub fn ascii_host(host: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::ascii_host(host)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `urlParts`.
#[napi(js_name = "urlParts")]
pub fn url_parts(href: String) -> napi::Result<String> {
    run(move || {
        let out = port::url_parts(href)?;
        crate::js::to_json(&out)
    })
}

/// `effectivePort`.
#[napi(js_name = "effectivePort")]
pub fn effective_port(parts: String) -> napi::Result<String> {
    run(move || {
        let parts = crate::js::from_json(&parts)?;
        let out = port::effective_port(parts)?;
        crate::js::to_json(&out)
    })
}

/// `hostWithPort`.
#[napi(js_name = "hostWithPort")]
pub fn host_with_port(parts: String) -> napi::Result<String> {
    run(move || {
        let parts = crate::js::from_json(&parts)?;
        let out = port::host_with_port(parts)?;
        crate::js::to_json(&out)
    })
}

/// `originOf`.
#[napi(js_name = "originOf")]
pub fn origin_of(parts: String) -> napi::Result<String> {
    run(move || {
        let parts = crate::js::from_json(&parts)?;
        let out = port::origin_of(parts)?;
        crate::js::to_json(&out)
    })
}

/// `formatUrl`.
#[napi(js_name = "formatUrl")]
pub fn format_url(parts: String) -> napi::Result<String> {
    run(move || {
        let parts = crate::js::from_json(&parts)?;
        let out = port::format_url(parts)?;
        crate::js::to_json(&out)
    })
}

/// `webUrl`.
#[napi(js_name = "webUrl")]
pub fn web_url(href: String) -> napi::Result<String> {
    run(move || {
        let out = port::web_url(href)?;
        crate::js::to_json(&out)
    })
}

/// `minimizedUrl`.
#[napi(js_name = "minimizedUrl")]
pub fn minimized_url(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::minimized_url(url)?;
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

/// `canonicalIpv6` over the binary wire.
#[napi(js_name = "canonicalIpv6Wire")]
pub fn canonical_ipv6_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::canonical_ipv6(text)?;
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

/// `asciiHost` over the binary wire.
#[napi(js_name = "asciiHostWire")]
pub fn ascii_host_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let host: String = r.string16()?;
        let out = &port::ascii_host(host)?;
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

static S_URL_PARTS_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("scheme", wire::Schema::Str16),
    ("host", wire::Schema::Str16),
    ("port", wire::Schema::Str16),
    ("path", wire::Schema::Str16),
    ("query", wire::Schema::Str16),
    ("hash", wire::Schema::Str16),
    ("authority", wire::Schema::Bool),
]));

/// `urlParts` over the binary wire.
#[napi(js_name = "urlPartsWire")]
pub fn url_parts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let href: String = r.string16()?;
        let out = &port::url_parts(href)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_URL_PARTS_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_EFFECTIVE_PORT_0: wire::Schema = wire::Schema::Dyn;
static S_EFFECTIVE_PORT_OUT: wire::Schema = wire::Schema::Dyn;

/// `effectivePort` over the binary wire.
#[napi(js_name = "effectivePortWire")]
pub fn effective_port_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let parts: serde_json::Value = wire::read_value(r, &S_EFFECTIVE_PORT_0)?;
        let out = &port::effective_port(parts)?;
        wire::write_value(w, out, &S_EFFECTIVE_PORT_OUT)?;
        Ok(())
    })
}

static S_HOST_WITH_PORT_0: wire::Schema = wire::Schema::Dyn;
static S_HOST_WITH_PORT_OUT: wire::Schema = wire::Schema::Dyn;

/// `hostWithPort` over the binary wire.
#[napi(js_name = "hostWithPortWire")]
pub fn host_with_port_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let parts: serde_json::Value = wire::read_value(r, &S_HOST_WITH_PORT_0)?;
        let out = &port::host_with_port(parts)?;
        wire::write_value(w, out, &S_HOST_WITH_PORT_OUT)?;
        Ok(())
    })
}

static S_ORIGIN_OF_0: wire::Schema = wire::Schema::Dyn;
static S_ORIGIN_OF_OUT: wire::Schema = wire::Schema::Dyn;

/// `originOf` over the binary wire.
#[napi(js_name = "originOfWire")]
pub fn origin_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let parts: serde_json::Value = wire::read_value(r, &S_ORIGIN_OF_0)?;
        let out = &port::origin_of(parts)?;
        wire::write_value(w, out, &S_ORIGIN_OF_OUT)?;
        Ok(())
    })
}

static S_FORMAT_URL_0: wire::Schema = wire::Schema::Dyn;
static S_FORMAT_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `formatUrl` over the binary wire.
#[napi(js_name = "formatUrlWire")]
pub fn format_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let parts: serde_json::Value = wire::read_value(r, &S_FORMAT_URL_0)?;
        let out = &port::format_url(parts)?;
        wire::write_value(w, out, &S_FORMAT_URL_OUT)?;
        Ok(())
    })
}

static S_WEB_URL_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("scheme", wire::Schema::Str16),
    ("host", wire::Schema::Str16),
    ("port", wire::Schema::Str16),
    ("path", wire::Schema::Str16),
    ("query", wire::Schema::Str16),
    ("hash", wire::Schema::Str16),
    ("authority", wire::Schema::Bool),
]));

/// `webUrl` over the binary wire.
#[napi(js_name = "webUrlWire")]
pub fn web_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let href: String = r.string16()?;
        let out = &port::web_url(href)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_WEB_URL_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `minimizedUrl` over the binary wire.
#[napi(js_name = "minimizedUrlWire")]
pub fn minimized_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::minimized_url(url)?;
        w.string16(out);
        Ok(())
    })
}
