// Ported from Ardia (https://github.com/ardia-browser/ardia), commit 551d3c13c:
// packages/ardiacore/ios/ArdiaEngineBridge.mm (`Symbol`, `InstallSink`) and
// engine/chromium/src/chrome/browser/ardia/public/ac_engine.h (the C types).
// Copyright 2026 Mantra Patel, Apache-2.0. This file is a Rust port with changes.

//! Raw access to the `ac_*` calls of the engine. The engine is already loaded into the
//! process (`dlopen` of `Chromium Framework`), so each call is found with `dlsym(RTLD_DEFAULT)`.
//! All calls must run on the main thread (`rust/research/engine-threads.md`, section 2.1).

#![allow(unsafe_code)]

use std::ffi::{CString, c_char, c_void};

mod names;

pub use names::CALLS;

/// `ac_engine_reply_t`: runs once per call, maybe before the call returns. `json` is valid only inside the callback.
pub type ReplyFn = unsafe extern "C" fn(context: *mut c_void, json: *const c_char);
/// `ac_engine_call_t`: the signature of every `ac_*` export.
pub type CallFn = unsafe extern "C" fn(
    profile_dir: *const c_char,
    args_json: *const c_char,
    reply: ReplyFn,
    context: *mut c_void,
);
/// `ac_engine_event_t`: `topic` and `json` are valid only inside the callback.
pub type EventFn =
    unsafe extern "C" fn(context: *mut c_void, topic: *const c_char, json: *const c_char);
type SetEventSinkFn = unsafe extern "C" fn(sink: Option<EventFn>, context: *mut c_void);

fn symbol(name: &str) -> Option<*mut c_void> {
    let name = CString::new(name).ok()?;
    // SAFETY: `name` is a valid NUL-terminated string; `dlsym` only reads it.
    let found = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
    (!found.is_null()).then_some(found)
}

/// Finds one of the [`CALLS`] by name. `None` for a name outside the list or a symbol the engine does not export.
pub fn call_symbol(name: &str) -> Option<CallFn> {
    CALLS.binary_search(&name).ok()?;
    // SAFETY: every name in `CALLS` is an `AC_*_CALL` export with the `ac_engine_call_t` signature.
    symbol(name).map(|p| unsafe { std::mem::transmute::<*mut c_void, CallFn>(p) })
}

/// The names in [`CALLS`] that the loaded engine does not export.
pub fn missing_calls() -> Vec<&'static str> {
    CALLS
        .iter()
        .copied()
        .filter(|n| call_symbol(n).is_none())
        .collect()
}

/// Sets the one global event sink of the engine, or clears it with `None`.
/// Returns false when the engine has no `ac_engine_set_event_sink`.
///
/// # Safety
/// `context` must stay valid until the sink is replaced or cleared, and `sink` must follow the callback rules above.
pub unsafe fn set_event_sink(sink: Option<EventFn>, context: *mut c_void) -> bool {
    let Some(p) = symbol("ac_engine_set_event_sink") else {
        return false;
    };
    // SAFETY: the export has the `ac_engine_set_event_sink_t` signature (public/ac_engine.h:27-28).
    let set = unsafe { std::mem::transmute::<*mut c_void, SetEventSinkFn>(p) };
    unsafe { set(sink, context) };
    true
}

#[cfg(test)]
mod tests {
    use super::CALLS;

    // `call_symbol` uses a binary search, so a wrong order would hide names.
    #[test]
    fn calls_are_sorted_and_unique() {
        assert!(CALLS.windows(2).all(|w| w[0] < w[1]));
    }
}
