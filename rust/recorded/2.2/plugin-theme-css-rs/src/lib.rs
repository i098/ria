//! JS bindings for `apps/browser/src/lib/pluginThemeCss.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `cssRoleName`.
#[napi(js_name = "cssRoleName")]
pub fn css_role_name(role: String) -> napi::Result<String> {
    run(move || {
        let out = port::css_role_name(role)?;
        crate::js::to_json(&out)
    })
}

/// `cssRoleValue`.
#[napi(js_name = "cssRoleValue")]
pub fn css_role_value(value: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::css_role_value(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `pluginThemeVariables`.
#[napi(js_name = "pluginThemeVariables")]
pub fn plugin_theme_variables(roles: String) -> napi::Result<String> {
    run(move || {
        let roles = crate::js::from_json(&roles)?;
        let out = port::plugin_theme_variables(roles)?;
        crate::js::to_json(&out)
    })
}

/// `buildPluginThemeCss`.
#[napi(js_name = "buildPluginThemeCss")]
pub fn build_plugin_theme_css(theme: String) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::build_plugin_theme_css(theme)?;
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

static S_CSS_ROLE_NAME_OUT: wire::Schema = wire::Schema::Dyn;

/// `cssRoleName` over the binary wire.
#[napi(js_name = "cssRoleNameWire")]
pub fn css_role_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let role: String = r.string16()?;
        let out = &port::css_role_name(role)?;
        wire::write_value(w, out, &S_CSS_ROLE_NAME_OUT)?;
        Ok(())
    })
}

static S_CSS_ROLE_VALUE_0: wire::Schema = wire::Schema::Dyn;

/// `cssRoleValue` over the binary wire.
#[napi(js_name = "cssRoleValueWire")]
pub fn css_role_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_CSS_ROLE_VALUE_0)?;
        let out = &port::css_role_value(value)?;
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

static S_PLUGIN_THEME_VARIABLES_0: wire::Schema = wire::Schema::Dyn;
static S_PLUGIN_THEME_VARIABLES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `pluginThemeVariables` over the binary wire.
#[napi(js_name = "pluginThemeVariablesWire")]
pub fn plugin_theme_variables_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let roles: serde_json::Value = wire::read_value(r, &S_PLUGIN_THEME_VARIABLES_0)?;
        let out = &port::plugin_theme_variables(roles)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PLUGIN_THEME_VARIABLES_OUT.inner())?;
        }
        Ok(())
    })
}

static S_BUILD_PLUGIN_THEME_CSS_0: wire::Schema = wire::Schema::Dyn;

/// `buildPluginThemeCss` over the binary wire.
#[napi(js_name = "buildPluginThemeCssWire")]
pub fn build_plugin_theme_css_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_BUILD_PLUGIN_THEME_CSS_0)?;
        let out = &port::build_plugin_theme_css(theme)?;
        w.string16(out);
        Ok(())
    })
}
