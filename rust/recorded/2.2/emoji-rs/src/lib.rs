//! JS bindings for `apps/browser/src/lib/emoji.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `emojiSections`.
#[napi(js_name = "emojiSections")]
pub fn emoji_sections() -> napi::Result<String> {
    run(move || {
        let out = port::emoji_sections()?;
        crate::js::to_json(&out)
    })
}

/// `searchTerms`.
#[napi(js_name = "searchTerms")]
pub fn search_terms(query: String) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::search_terms(query)?;
        Ok(out)
    })
}

/// `matchesTerms`.
#[napi(js_name = "matchesTerms")]
pub fn matches_terms(text: String, terms: Vec<String>) -> napi::Result<String> {
    run(move || {
        let out = port::matches_terms(text, terms)?;
        crate::js::to_json(&out)
    })
}

/// `searchEmoji`.
#[napi(js_name = "searchEmoji")]
pub fn search_emoji(terms: Vec<String>) -> napi::Result<String> {
    run(move || {
        let out = port::search_emoji(terms)?;
        crate::js::to_json(&out)
    })
}

/// `sameIcon`.
#[napi(js_name = "sameIcon")]
pub fn same_icon(a: String, b: String) -> napi::Result<String> {
    run(move || {
        let a = crate::js::from_json(&a)?;
        let b = crate::js::from_json(&b)?;
        let out = port::same_icon(a, b)?;
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

static S_EMOJI_SECTIONS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("title", wire::Schema::Str16),
    (
        "emoji",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("emoji", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("nameWords", wire::Schema::Str16),
            ("words", wire::Schema::Str16),
        ])),
    ),
]));

/// `emojiSections` over the binary wire.
#[napi(js_name = "emojiSectionsWire")]
pub fn emoji_sections_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::emoji_sections()?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_EMOJI_SECTIONS_OUT.inner())?;
        }
        Ok(())
    })
}

/// `searchTerms` over the binary wire.
#[napi(js_name = "searchTermsWire")]
pub fn search_terms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let query: String = r.string16()?;
        let out = &port::search_terms(query)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_MATCHES_TERMS_OUT: wire::Schema = wire::Schema::Dyn;

/// `matchesTerms` over the binary wire.
#[napi(js_name = "matchesTermsWire")]
pub fn matches_terms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let terms: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::matches_terms(text, terms)?;
        wire::write_value(w, out, &S_MATCHES_TERMS_OUT)?;
        Ok(())
    })
}

static S_SEARCH_EMOJI_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("emoji", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("nameWords", wire::Schema::Str16),
    ("words", wire::Schema::Str16),
]));

/// `searchEmoji` over the binary wire.
#[napi(js_name = "searchEmojiWire")]
pub fn search_emoji_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let terms: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::search_emoji(terms)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_SEARCH_EMOJI_OUT.inner())?;
        }
        Ok(())
    })
}

static S_SAME_ICON_0: wire::Schema = wire::Schema::Dyn;
static S_SAME_ICON_1: wire::Schema = wire::Schema::Dyn;
static S_SAME_ICON_OUT: wire::Schema = wire::Schema::Dyn;

/// `sameIcon` over the binary wire.
#[napi(js_name = "sameIconWire")]
pub fn same_icon_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: Value = wire::read_value(r, &S_SAME_ICON_0)?;
        let b: Value = wire::read_value(r, &S_SAME_ICON_1)?;
        let out = &port::same_icon(a, b)?;
        wire::write_value(w, out, &S_SAME_ICON_OUT)?;
        Ok(())
    })
}
