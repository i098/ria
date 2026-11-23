//! JS bindings for `packages/core/src/omnibox.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `hostOf`.
#[napi(js_name = "hostOf")]
pub fn host_of(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::host_of(url)?;
        Ok(out)
    })
}

/// `isJavaScriptUrl`.
#[napi(js_name = "isJavaScriptUrl")]
pub fn is_java_script_url(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::is_java_script_url(url)?;
        crate::js::to_json(&out)
    })
}

/// `withoutJavaScriptScheme`.
#[napi(js_name = "withoutJavaScriptScheme")]
pub fn without_java_script_scheme(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::without_java_script_scheme(text)?;
        Ok(out)
    })
}

/// `fixupUrl`.
#[napi(js_name = "fixupUrl")]
pub fn fixup_url(raw: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::fixup_url(raw)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `sameAddress`.
#[napi(js_name = "sameAddress")]
pub fn same_address(a: String, b: String) -> napi::Result<String> {
    run(move || {
        let out = port::same_address(a, b)?;
        crate::js::to_json(&out)
    })
}

/// `spacedAddressGuess`.
#[napi(js_name = "spacedAddressGuess")]
pub fn spaced_address_guess(
    raw: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::spaced_address_guess(raw)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `searchUrlFor`.
#[napi(js_name = "searchUrlFor")]
pub fn search_url_for(template: String, query: String) -> napi::Result<String> {
    run(move || {
        let out = port::search_url_for(template, query)?;
        Ok(out)
    })
}

/// `resolveInput`.
#[napi(js_name = "resolveInput")]
pub fn resolve_input(raw: String, search: String) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_input(raw, search)?;
        Ok(out)
    })
}

/// `classifyPaste`.
#[napi(js_name = "classifyPaste")]
pub fn classify_paste(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::classify_paste(text)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizePaste`.
#[napi(js_name = "sanitizePaste")]
pub fn sanitize_paste(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::sanitize_paste(text)?;
        Ok(out)
    })
}

/// `breadcrumb`.
#[napi(js_name = "breadcrumb")]
pub fn breadcrumb(url: String, title: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::breadcrumb(url, title)?;
        crate::js::to_json(&out)
    })
}

/// `urlForDisplay`.
#[napi(js_name = "urlForDisplay")]
pub fn url_for_display(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::url_for_display(url)?;
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

/// `hostOf` over the binary wire.
#[napi(js_name = "hostOfWire")]
pub fn host_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::host_of(url)?;
        w.string16(out);
        Ok(())
    })
}

static S_IS_JAVA_SCRIPT_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `isJavaScriptUrl` over the binary wire.
#[napi(js_name = "isJavaScriptUrlWire")]
pub fn is_java_script_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::is_java_script_url(url)?;
        wire::write_value(w, out, &S_IS_JAVA_SCRIPT_URL_OUT)?;
        Ok(())
    })
}

/// `withoutJavaScriptScheme` over the binary wire.
#[napi(js_name = "withoutJavaScriptSchemeWire")]
pub fn without_java_script_scheme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::without_java_script_scheme(text)?;
        w.string16(out);
        Ok(())
    })
}

/// `fixupUrl` over the binary wire.
#[napi(js_name = "fixupUrlWire")]
pub fn fixup_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::fixup_url(raw)?;
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

static S_SAME_ADDRESS_OUT: wire::Schema = wire::Schema::Dyn;

/// `sameAddress` over the binary wire.
#[napi(js_name = "sameAddressWire")]
pub fn same_address_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: String = r.string16()?;
        let b: String = r.string16()?;
        let out = &port::same_address(a, b)?;
        wire::write_value(w, out, &S_SAME_ADDRESS_OUT)?;
        Ok(())
    })
}

/// `spacedAddressGuess` over the binary wire.
#[napi(js_name = "spacedAddressGuessWire")]
pub fn spaced_address_guess_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::spaced_address_guess(raw)?;
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

/// `searchUrlFor` over the binary wire.
#[napi(js_name = "searchUrlForWire")]
pub fn search_url_for_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let template: String = r.string16()?;
        let query: String = r.string16()?;
        let out = &port::search_url_for(template, query)?;
        w.string16(out);
        Ok(())
    })
}

/// `resolveInput` over the binary wire.
#[napi(js_name = "resolveInputWire")]
pub fn resolve_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let search: String = r.string16()?;
        let out = &port::resolve_input(raw, search)?;
        w.string16(out);
        Ok(())
    })
}

/// `classifyPaste` over the binary wire.
#[napi(js_name = "classifyPasteWire")]
pub fn classify_paste_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::classify_paste(text)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `sanitizePaste` over the binary wire.
#[napi(js_name = "sanitizePasteWire")]
pub fn sanitize_paste_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::sanitize_paste(text)?;
        w.string16(out);
        Ok(())
    })
}

static S_BREADCRUMB_OUT: wire::Schema = wire::Schema::Obj(&[
    ("host", wire::Schema::Str16),
    ("trail", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `breadcrumb` over the binary wire.
#[napi(js_name = "breadcrumbWire")]
pub fn breadcrumb_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let title: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::breadcrumb(url, title)?;
        wire::write_value(w, out, &S_BREADCRUMB_OUT)?;
        Ok(())
    })
}

/// `urlForDisplay` over the binary wire.
#[napi(js_name = "urlForDisplayWire")]
pub fn url_for_display_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::url_for_display(url)?;
        w.string16(out);
        Ok(())
    })
}
