//! JS bindings for `apps/browser/src/lib/pluginMarkdown.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `markdownLink`.
#[napi(js_name = "markdownLink")]
pub fn markdown_link(raw: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::markdown_link(raw)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `parseInline`.
#[napi(js_name = "parseInline")]
pub fn parse_inline(source: String, style: String, depth: Option<f64>) -> napi::Result<String> {
    run(move || {
        let style = crate::js::from_json(&style)?;
        let out = port::parse_inline(source, style, depth)?;
        crate::js::to_json(&out)
    })
}

/// `parseMarkdown`.
#[napi(js_name = "parseMarkdown")]
pub fn parse_markdown(source: String, depth: Option<f64>) -> napi::Result<String> {
    run(move || {
        let out = port::parse_markdown(source, depth)?;
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

/// `markdownLink` over the binary wire.
#[napi(js_name = "markdownLinkWire")]
pub fn markdown_link_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::markdown_link(raw)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_PARSE_INLINE_1: wire::Schema = wire::Schema::Dyn;
static S_PARSE_INLINE_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("text", wire::Schema::Str16),
    ("bold", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("italic", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("code", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("url", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `parseInline` over the binary wire.
#[napi(js_name = "parseInlineWire")]
pub fn parse_inline_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: String = r.string16()?;
        let style: Value = wire::read_value(r, &S_PARSE_INLINE_1)?;
        let depth: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::parse_inline(source, style, depth)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_INLINE_OUT.inner())?;
        }
        Ok(())
    })
}

static S_PARSE_MARKDOWN_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `parseMarkdown` over the binary wire.
#[napi(js_name = "parseMarkdownWire")]
pub fn parse_markdown_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: String = r.string16()?;
        let depth: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::parse_markdown(source, depth)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_MARKDOWN_OUT.inner())?;
        }
        Ok(())
    })
}
