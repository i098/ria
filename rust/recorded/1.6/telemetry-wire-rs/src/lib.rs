//! JS bindings for `packages/core/src/telemetryWire.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

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

/// `eventRecord`.
#[napi(js_name = "eventRecord")]
pub fn event_record(e: String) -> napi::Result<String> {
    run(move || {
        let e = crate::js::from_json(&e)?;
        let out = port::event_record(e)?;
        crate::js::to_json(&out)
    })
}

/// `logRecord`.
#[napi(js_name = "logRecord")]
pub fn log_record(
    level: String,
    message: String,
    attributes: String,
    at: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let attributes = crate::js::from_json(&attributes)?;
        let out = port::log_record(level, message, attributes, at)?;
        crate::js::to_json(&out)
    })
}

/// `logsRequest`.
#[napi(js_name = "logsRequest")]
pub fn logs_request(resource: String, scope: String, records: String) -> napi::Result<String> {
    run(move || {
        let resource = crate::js::from_json(&resource)?;
        let records: Vec<Value> = crate::js::from_json(&records)?;
        let out = port::logs_request(resource, scope, records)?;
        Ok(out)
    })
}

/// `outcome`.
#[napi(js_name = "outcome")]
pub fn outcome(status: f64, count: f64) -> napi::Result<String> {
    run(move || {
        let out = port::outcome(status, count)?;
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

static S_EVENT_RECORD_0: wire::Schema = wire::Schema::Dyn;
static S_EVENT_RECORD_OUT: wire::Schema = wire::Schema::Obj(&[
    ("timeUnixNano", wire::Schema::Str16),
    ("severityNumber", wire::Schema::Num),
    ("severityText", wire::Schema::Str16),
    (
        "body",
        wire::Schema::Obj(&[("stringValue", wire::Schema::Str16)]),
    ),
    (
        "attributes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("key", wire::Schema::Str16),
            ("value", wire::Schema::Dyn),
        ])),
    ),
]);

/// `eventRecord` over the binary wire.
#[napi(js_name = "eventRecordWire")]
pub fn event_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let e: serde_json::Value = wire::read_value(r, &S_EVENT_RECORD_0)?;
        let out = &port::event_record(e)?;
        wire::write_value(w, out, &S_EVENT_RECORD_OUT)?;
        Ok(())
    })
}

static S_LOG_RECORD_2: wire::Schema = wire::Schema::Dyn;
static S_LOG_RECORD_OUT: wire::Schema = wire::Schema::Obj(&[
    ("timeUnixNano", wire::Schema::Str16),
    ("severityNumber", wire::Schema::Num),
    ("severityText", wire::Schema::Str16),
    (
        "body",
        wire::Schema::Obj(&[("stringValue", wire::Schema::Str16)]),
    ),
    (
        "attributes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("key", wire::Schema::Str16),
            ("value", wire::Schema::Dyn),
        ])),
    ),
]);

/// `logRecord` over the binary wire.
#[napi(js_name = "logRecordWire")]
pub fn log_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let level: String = r.string16()?;
        let message: String = r.string16()?;
        let attributes: serde_json::Value = wire::read_value(r, &S_LOG_RECORD_2)?;
        let at = r.opt(|r| r.f64())?;
        let out = &port::log_record(level, message, attributes, at)?;
        wire::write_value(w, out, &S_LOG_RECORD_OUT)?;
        Ok(())
    })
}

static S_LOGS_REQUEST_0: wire::Schema = wire::Schema::Dyn;
static S_LOGS_REQUEST_2: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("timeUnixNano", wire::Schema::Str16),
    ("severityNumber", wire::Schema::Num),
    ("severityText", wire::Schema::Str16),
    (
        "body",
        wire::Schema::Obj(&[("stringValue", wire::Schema::Str16)]),
    ),
    (
        "attributes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("key", wire::Schema::Str16),
            ("value", wire::Schema::Dyn),
        ])),
    ),
]));

/// `logsRequest` over the binary wire.
#[napi(js_name = "logsRequestWire")]
pub fn logs_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let resource: serde_json::Value = wire::read_value(r, &S_LOGS_REQUEST_0)?;
        let scope: String = r.string16()?;
        let records = r.list(|r| wire::read_value(r, S_LOGS_REQUEST_2.inner()))?;
        let out = &port::logs_request(resource, scope, records)?;
        w.string16(out);
        Ok(())
    })
}

/// `outcome` over the binary wire.
#[napi(js_name = "outcomeWire")]
pub fn outcome_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let status: f64 = r.f64()?;
        let count: f64 = r.f64()?;
        let out = &port::outcome(status, count)?;
        w.string16(out);
        Ok(())
    })
}
