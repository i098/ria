//! JS bindings for `apps/browser/src/lib/pluginShortcuts.harness.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parsePluginShortcutCase`.
#[napi(js_name = "parsePluginShortcutCase")]
pub fn parse_plugin_shortcut_case(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::parse_plugin_shortcut_case(raw)?;
        crate::js::to_json(&out)
    })
}

/// `pluginShortcutTextCase`.
#[napi(js_name = "pluginShortcutTextCase")]
pub fn plugin_shortcut_text_case(shortcut: String) -> napi::Result<String> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let out = port::plugin_shortcut_text_case(shortcut)?;
        Ok(out)
    })
}

/// `isRefusedShortcutCase`.
#[napi(js_name = "isRefusedShortcutCase")]
pub fn is_refused_shortcut_case(shortcut: String) -> napi::Result<bool> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let out = port::is_refused_shortcut_case(shortcut)?;
        Ok(out)
    })
}

/// `pluginShortcutOfCase`.
#[napi(js_name = "pluginShortcutOfCase")]
pub fn plugin_shortcut_of_case(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::plugin_shortcut_of_case(raw)?;
        crate::js::to_json(&out)
    })
}

/// `pluginShortcutModifiersCase`.
#[napi(js_name = "pluginShortcutModifiersCase")]
pub fn plugin_shortcut_modifiers_case(shortcut: String) -> napi::Result<Vec<String>> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let out = port::plugin_shortcut_modifiers_case(shortcut)?;
        Ok(out)
    })
}

/// `pluginShortcutKeyCharCase`.
#[napi(js_name = "pluginShortcutKeyCharCase")]
pub fn plugin_shortcut_key_char_case(shortcut: String) -> napi::Result<String> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let out = port::plugin_shortcut_key_char_case(shortcut)?;
        Ok(out)
    })
}

/// `pluginShortcutHandledKeysCase`.
#[napi(js_name = "pluginShortcutHandledKeysCase")]
pub fn plugin_shortcut_handled_keys_case(
    shortcut: String,
    command_flag: String,
) -> napi::Result<String> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let command_flag = crate::js::from_json(&command_flag)?;
        let out = port::plugin_shortcut_handled_keys_case(shortcut, command_flag)?;
        crate::js::to_json(&out)
    })
}

/// `pluginShortcutMatchesCase`.
#[napi(js_name = "pluginShortcutMatchesCase")]
pub fn plugin_shortcut_matches_case(
    shortcut: String,
    e: String,
    command_flag: String,
) -> napi::Result<bool> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let e = crate::js::from_json(&e)?;
        let command_flag = crate::js::from_json(&command_flag)?;
        let out = port::plugin_shortcut_matches_case(shortcut, e, command_flag)?;
        Ok(out)
    })
}

/// `bindingsOfCase`.
#[napi(js_name = "bindingsOfCase")]
pub fn bindings_of_case(actions: String) -> napi::Result<String> {
    run(move || {
        let actions = crate::js::from_json(&actions)?;
        let out = port::bindings_of_case(actions)?;
        crate::js::to_json(&out)
    })
}

/// `mergeShortcutBindingsCase`.
#[napi(js_name = "mergeShortcutBindingsCase")]
pub fn merge_shortcut_bindings_case(registrations: String, held: String) -> napi::Result<String> {
    run(move || {
        let registrations = crate::js::from_json(&registrations)?;
        let held = crate::js::from_json(&held)?;
        let out = port::merge_shortcut_bindings_case(registrations, held)?;
        crate::js::to_json(&out)
    })
}

/// `pressedBindingCase`.
#[napi(js_name = "pressedBindingCase")]
pub fn pressed_binding_case(
    bindings: String,
    e: String,
    command_flag: String,
) -> napi::Result<String> {
    run(move || {
        let bindings = crate::js::from_json(&bindings)?;
        let e = crate::js::from_json(&e)?;
        let command_flag = crate::js::from_json(&command_flag)?;
        let out = port::pressed_binding_case(bindings, e, command_flag)?;
        crate::js::to_json(&out)
    })
}

/// `handledKeysOfCase`.
#[napi(js_name = "handledKeysOfCase")]
pub fn handled_keys_of_case(bindings: String, command_flag: String) -> napi::Result<String> {
    run(move || {
        let bindings = crate::js::from_json(&bindings)?;
        let command_flag = crate::js::from_json(&command_flag)?;
        let out = port::handled_keys_of_case(bindings, command_flag)?;
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

static S_PARSE_PLUGIN_SHORTCUT_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_PLUGIN_SHORTCUT_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parsePluginShortcutCase` over the binary wire.
#[napi(js_name = "parsePluginShortcutCaseWire")]
pub fn parse_plugin_shortcut_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Value = wire::read_value(r, &S_PARSE_PLUGIN_SHORTCUT_CASE_0)?;
        let out = &port::parse_plugin_shortcut_case(raw)?;
        wire::write_value(w, out, &S_PARSE_PLUGIN_SHORTCUT_CASE_OUT)?;
        Ok(())
    })
}

static S_PLUGIN_SHORTCUT_TEXT_CASE_0: wire::Schema = wire::Schema::Dyn;

/// `pluginShortcutTextCase` over the binary wire.
#[napi(js_name = "pluginShortcutTextCaseWire")]
pub fn plugin_shortcut_text_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_TEXT_CASE_0)?;
        let out = &port::plugin_shortcut_text_case(shortcut)?;
        w.string16(out);
        Ok(())
    })
}

static S_IS_REFUSED_SHORTCUT_CASE_0: wire::Schema = wire::Schema::Dyn;

/// `isRefusedShortcutCase` over the binary wire.
#[napi(js_name = "isRefusedShortcutCaseWire")]
pub fn is_refused_shortcut_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: Value = wire::read_value(r, &S_IS_REFUSED_SHORTCUT_CASE_0)?;
        let out = &port::is_refused_shortcut_case(shortcut)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PLUGIN_SHORTCUT_OF_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_PLUGIN_SHORTCUT_OF_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `pluginShortcutOfCase` over the binary wire.
#[napi(js_name = "pluginShortcutOfCaseWire")]
pub fn plugin_shortcut_of_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_OF_CASE_0)?;
        let out = &port::plugin_shortcut_of_case(raw)?;
        wire::write_value(w, out, &S_PLUGIN_SHORTCUT_OF_CASE_OUT)?;
        Ok(())
    })
}

static S_PLUGIN_SHORTCUT_MODIFIERS_CASE_0: wire::Schema = wire::Schema::Dyn;

/// `pluginShortcutModifiersCase` over the binary wire.
#[napi(js_name = "pluginShortcutModifiersCaseWire")]
pub fn plugin_shortcut_modifiers_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_MODIFIERS_CASE_0)?;
        let out = &port::plugin_shortcut_modifiers_case(shortcut)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_PLUGIN_SHORTCUT_KEY_CHAR_CASE_0: wire::Schema = wire::Schema::Dyn;

/// `pluginShortcutKeyCharCase` over the binary wire.
#[napi(js_name = "pluginShortcutKeyCharCaseWire")]
pub fn plugin_shortcut_key_char_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_KEY_CHAR_CASE_0)?;
        let out = &port::plugin_shortcut_key_char_case(shortcut)?;
        w.string16(out);
        Ok(())
    })
}

static S_PLUGIN_SHORTCUT_HANDLED_KEYS_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_PLUGIN_SHORTCUT_HANDLED_KEYS_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_PLUGIN_SHORTCUT_HANDLED_KEYS_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `pluginShortcutHandledKeysCase` over the binary wire.
#[napi(js_name = "pluginShortcutHandledKeysCaseWire")]
pub fn plugin_shortcut_handled_keys_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_HANDLED_KEYS_CASE_0)?;
        let command_flag: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_HANDLED_KEYS_CASE_1)?;
        let out = &port::plugin_shortcut_handled_keys_case(shortcut, command_flag)?;
        wire::write_value(w, out, &S_PLUGIN_SHORTCUT_HANDLED_KEYS_CASE_OUT)?;
        Ok(())
    })
}

static S_PLUGIN_SHORTCUT_MATCHES_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_PLUGIN_SHORTCUT_MATCHES_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_PLUGIN_SHORTCUT_MATCHES_CASE_2: wire::Schema = wire::Schema::Dyn;

/// `pluginShortcutMatchesCase` over the binary wire.
#[napi(js_name = "pluginShortcutMatchesCaseWire")]
pub fn plugin_shortcut_matches_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_MATCHES_CASE_0)?;
        let e: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_MATCHES_CASE_1)?;
        let command_flag: Value = wire::read_value(r, &S_PLUGIN_SHORTCUT_MATCHES_CASE_2)?;
        let out = &port::plugin_shortcut_matches_case(shortcut, e, command_flag)?;
        w.bool(*out);
        Ok(())
    })
}

static S_BINDINGS_OF_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_BINDINGS_OF_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `bindingsOfCase` over the binary wire.
#[napi(js_name = "bindingsOfCaseWire")]
pub fn bindings_of_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let actions: Value = wire::read_value(r, &S_BINDINGS_OF_CASE_0)?;
        let out = &port::bindings_of_case(actions)?;
        wire::write_value(w, out, &S_BINDINGS_OF_CASE_OUT)?;
        Ok(())
    })
}

static S_MERGE_SHORTCUT_BINDINGS_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_MERGE_SHORTCUT_BINDINGS_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_MERGE_SHORTCUT_BINDINGS_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `mergeShortcutBindingsCase` over the binary wire.
#[napi(js_name = "mergeShortcutBindingsCaseWire")]
pub fn merge_shortcut_bindings_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let registrations: Value = wire::read_value(r, &S_MERGE_SHORTCUT_BINDINGS_CASE_0)?;
        let held: Value = wire::read_value(r, &S_MERGE_SHORTCUT_BINDINGS_CASE_1)?;
        let out = &port::merge_shortcut_bindings_case(registrations, held)?;
        wire::write_value(w, out, &S_MERGE_SHORTCUT_BINDINGS_CASE_OUT)?;
        Ok(())
    })
}

static S_PRESSED_BINDING_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_PRESSED_BINDING_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_PRESSED_BINDING_CASE_2: wire::Schema = wire::Schema::Dyn;
static S_PRESSED_BINDING_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `pressedBindingCase` over the binary wire.
#[napi(js_name = "pressedBindingCaseWire")]
pub fn pressed_binding_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let bindings: Value = wire::read_value(r, &S_PRESSED_BINDING_CASE_0)?;
        let e: Value = wire::read_value(r, &S_PRESSED_BINDING_CASE_1)?;
        let command_flag: Value = wire::read_value(r, &S_PRESSED_BINDING_CASE_2)?;
        let out = &port::pressed_binding_case(bindings, e, command_flag)?;
        wire::write_value(w, out, &S_PRESSED_BINDING_CASE_OUT)?;
        Ok(())
    })
}

static S_HANDLED_KEYS_OF_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_HANDLED_KEYS_OF_CASE_1: wire::Schema = wire::Schema::Dyn;
static S_HANDLED_KEYS_OF_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `handledKeysOfCase` over the binary wire.
#[napi(js_name = "handledKeysOfCaseWire")]
pub fn handled_keys_of_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let bindings: Value = wire::read_value(r, &S_HANDLED_KEYS_OF_CASE_0)?;
        let command_flag: Value = wire::read_value(r, &S_HANDLED_KEYS_OF_CASE_1)?;
        let out = &port::handled_keys_of_case(bindings, command_flag)?;
        wire::write_value(w, out, &S_HANDLED_KEYS_OF_CASE_OUT)?;
        Ok(())
    })
}
