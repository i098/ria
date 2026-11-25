//! JS bindings for `apps/browser/src/lib/pluginActivation.harness.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `constantsCase`.
#[napi(js_name = "constantsCase")]
pub fn constants_case() -> napi::Result<String> {
    run(move || {
        let out = port::constants_case()?;
        crate::js::to_json(&out)
    })
}

/// `activationEventProblemCase`.
#[napi(js_name = "activationEventProblemCase")]
pub fn activation_event_problem_case(
    event: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::activation_event_problem_case(event)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `activationEventsOfCase`.
#[napi(js_name = "activationEventsOfCase")]
pub fn activation_events_of_case(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::activation_events_of_case(value)?;
        crate::js::to_json(&out)
    })
}

/// `derivedActivationEventsCase`.
#[napi(js_name = "derivedActivationEventsCase")]
pub fn derived_activation_events_case(
    contributes: String,
    main: bool,
    sidecar: bool,
) -> napi::Result<Vec<String>> {
    run(move || {
        let contributes = crate::js::from_json(&contributes)?;
        let out = port::derived_activation_events_case(contributes, main, sidecar)?;
        Ok(out)
    })
}

/// `startsEagerlyCase`.
#[napi(js_name = "startsEagerlyCase")]
pub fn starts_eagerly_case(events: Vec<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::starts_eagerly_case(events)?;
        Ok(out)
    })
}

/// `urlActivationHostsCase`.
#[napi(js_name = "urlActivationHostsCase")]
pub fn url_activation_hosts_case(events: Vec<String>) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::url_activation_hosts_case(events)?;
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

static S_CONSTANTS_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `constantsCase` over the binary wire.
#[napi(js_name = "constantsCaseWire")]
pub fn constants_case_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::constants_case()?;
        wire::write_value(w, out, &S_CONSTANTS_CASE_OUT)?;
        Ok(())
    })
}

static S_ACTIVATION_EVENT_PROBLEM_CASE_0: wire::Schema = wire::Schema::Dyn;

/// `activationEventProblemCase` over the binary wire.
#[napi(js_name = "activationEventProblemCaseWire")]
pub fn activation_event_problem_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_ACTIVATION_EVENT_PROBLEM_CASE_0)?;
        let out = &port::activation_event_problem_case(event)?;
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

static S_ACTIVATION_EVENTS_OF_CASE_0: wire::Schema = wire::Schema::Dyn;
static S_ACTIVATION_EVENTS_OF_CASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `activationEventsOfCase` over the binary wire.
#[napi(js_name = "activationEventsOfCaseWire")]
pub fn activation_events_of_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_ACTIVATION_EVENTS_OF_CASE_0)?;
        let out = &port::activation_events_of_case(value)?;
        wire::write_value(w, out, &S_ACTIVATION_EVENTS_OF_CASE_OUT)?;
        Ok(())
    })
}

static S_DERIVED_ACTIVATION_EVENTS_CASE_0: wire::Schema = wire::Schema::Dyn;

/// `derivedActivationEventsCase` over the binary wire.
#[napi(js_name = "derivedActivationEventsCaseWire")]
pub fn derived_activation_events_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let contributes: serde_json::Value =
            wire::read_value(r, &S_DERIVED_ACTIVATION_EVENTS_CASE_0)?;
        let main: bool = r.bool()?;
        let sidecar: bool = r.bool()?;
        let out = &port::derived_activation_events_case(contributes, main, sidecar)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

/// `startsEagerlyCase` over the binary wire.
#[napi(js_name = "startsEagerlyCaseWire")]
pub fn starts_eagerly_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let events: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::starts_eagerly_case(events)?;
        w.bool(*out);
        Ok(())
    })
}

/// `urlActivationHostsCase` over the binary wire.
#[napi(js_name = "urlActivationHostsCaseWire")]
pub fn url_activation_hosts_case_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let events: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::url_activation_hosts_case(events)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}
