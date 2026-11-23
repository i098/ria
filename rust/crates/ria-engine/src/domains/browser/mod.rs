// Ria original, no Ardia source.

//! Engine domains B: extensions, media, tabs and windows (plus the `webView` settings).
//!
//! [`route`] maps each call name of `api.ts` in these domains to what `ExtensionsModule.swift`, `MediaModule.swift`,
//! `TabsModule.swift`, `WindowsModule.swift` and `WebViewModule.swift` of Ardia do with it: a call that goes to an engine
//! symbol (`ac_*`), with the change of the argument shape and of the reply, or a call that the host answers with AppKit
//! code (`ria-host-mac`). [`call_engine`] runs the first kind.

use std::rc::Rc;

use serde_json::Value;

use crate::call::{CallError, CallResult, Engine};

pub mod extension_package;
pub mod extensions;
pub mod input_source_keys;
pub mod media;
pub mod native_messaging;
pub mod now_playing;
pub mod swipe;
pub mod system_state;
pub mod tab_cycling;
pub mod tabs;
mod text;
pub mod web_view;
pub mod windows;

/// What one call of `api.ts` does.
pub enum Route {
    /// The host answers the call.
    Host,
    Engine(EngineRoute),
}

/// A call that goes to an engine symbol.
pub struct EngineRoute {
    pub symbol: &'static str,
    base_profile: bool,
    /// The arguments of the symbol from the arguments of the call (without `profile`).
    pub shape: fn(&Value) -> Result<Value, CallError>,
    /// The reply of the call from the reply of the symbol.
    pub reply: fn(Value, &CallContext) -> Result<Value, CallError>,
    /// The `extensions.changed` event after a good reply, from the arguments and the reply:
    /// the extension id and the event name.
    pub changed: fn(&Value, &Value) -> Option<(String, &'static str)>,
}

/// The profile of a call.
#[derive(Debug, Clone)]
pub struct CallContext {
    /// The name of the original (not private) profile, for events.
    pub profile_name: String,
    /// The folder of the original profile, as `Engine::call` takes it.
    pub profile_dir: String,
    pub private: bool,
    /// The first preferred language of the app, for extension locale files.
    pub ui_locale: String,
}

/// Sends an event to the handlers of the app (`ArdiaEvents emit:` of Ardia).
pub type Emit = Rc<dyn Fn(&str, Value)>;

/// The route of a call, or `None` if it is not registered for this instance.
pub fn route(name: &str, test_instance: bool) -> Option<Route> {
    let (domain, call) = name.split_once('.')?;
    match domain {
        "extensions" => extensions::route(call),
        "media" => media::route(call),
        "tabs" => tabs::route(call, test_instance),
        "webView" => web_view::route(call),
        "windows" => windows::route(call),
        _ => None,
    }
}

/// Runs an engine route: shapes the arguments, calls the symbol, shapes the reply, and emits `extensions.changed`.
/// `completion` runs once: immediately for invalid arguments, or on a drain after dispatch.
pub fn call_engine(
    engine: &Engine,
    route: &EngineRoute,
    ctx: &CallContext,
    args: Value,
    emit: Emit,
    completion: impl FnOnce(CallResult) + 'static,
) {
    let shaped = match (route.shape)(&args) {
        Ok(shaped) => shaped,
        Err(error) => return completion(Err(error)),
    };
    let (reply_fn, changed_fn) = (route.reply, route.changed);
    let profile_dir = ctx.profile_dir.as_str();
    let private = ctx.private && !route.base_profile;
    let ctx = ctx.clone();
    engine.call(
        route.symbol,
        profile_dir,
        private,
        shaped,
        move |result| {
            let result = result.and_then(|reply| {
                if let Some((id, event)) = changed_fn(&args, &reply).filter(|(id, _)| !id.is_empty()) {
                    emit(
                        "extensions.changed",
                        serde_json::json!({"profile": ctx.profile_name, "id": id, "event": event}),
                    );
                }
                reply_fn(reply, &ctx)
            });
            completion(result)
        },
    );
}

/// An engine route that passes its arguments and its reply as they are.
pub(crate) const fn passing(symbol: &'static str) -> Route {
    Route::Engine(EngineRoute {
        symbol,
        base_profile: false,
        shape: |args| Ok(args.clone()),
        reply: |reply, _| Ok(reply),
        changed: |_, _| None,
    })
}

pub(crate) fn string_arg(args: &Value, key: &str) -> Result<String, CallError> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(CallError::BadArguments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Call, extensions::*};
    use crate::call::ActionQueue;
    use ria_engine_sys::ReplyFn;
    use std::cell::RefCell;
    use std::ffi::{CStr, c_char, c_void};

    thread_local! {
        static WIRED: RefCell<Option<(String, Value)>> = const { RefCell::new(None) };
    }

    unsafe extern "C" fn capture(
        profile: *const c_char,
        args: *const c_char,
        reply: ReplyFn,
        context: *mut c_void,
    ) {
        // SAFETY: Engine::call keeps the strings and reply context live for this call.
        unsafe {
            let profile = CStr::from_ptr(profile).to_string_lossy().into_owned();
            let args = serde_json::from_slice(CStr::from_ptr(args).to_bytes()).ok();
            WIRED.set(args.map(|args| (profile, args)));
            reply(context, c"{\"id\":\"ext\",\"extensions\":[]}".as_ptr());
        }
    }

    #[test]
    fn private_extension_calls_use_the_service_or_bridge_profile() {
        use serde_json::json;

        let queue = ActionQueue::new();
        let engine = Engine::with(queue.clone(), |_| Some(capture), |_, _| {});
        engine.did_start();
        let ctx = CallContext {
            profile_name: "base".into(),
            profile_dir: "/profiles/base".into(),
            private: true,
            ui_locale: "en".into(),
        };
        for (name, args, expected_private) in [
            (List::NAME, json!({}), false),
            (Install::NAME, json!({"path": "/ext"}), false),
            (InstallCrx::NAME, json!({"path": "/ext.crx"}), false),
            (SetEnabled::NAME, json!({"id": "ext", "enabled": true}), false),
            (Uninstall::NAME, json!({"id": "ext"}), false),
            (Reload::NAME, json!({"id": "ext"}), false),
            (
                Configure::NAME,
                json!({"id": "ext", "options": {"fileAccess": true}}),
                false,
            ),
            (WatchActions::NAME, json!({}), true),
            (SyncState::NAME, json!({}), true),
            (SyncApply::NAME, json!({}), true),
            (LoadHealth::NAME, json!({}), true),
        ] {
            let Some(Route::Engine(route)) = route(name, false) else {
                panic!("{name} does not have an engine route");
            };
            call_engine(&engine, &route, &ctx, args, Rc::new(|_, _| {}), |result| {
                assert!(result.is_ok(), "{result:?}");
            });
            let (profile, wired) = WIRED.take().expect("engine did not receive arguments");
            assert_eq!(profile, "/profiles/base");
            assert_eq!(
                wired.get("offTheRecord"),
                expected_private.then_some(&Value::Bool(true)),
                "{name}"
            );
            assert_eq!(queue.drain(), 1);
        }
        engine.shutdown();
    }
}
