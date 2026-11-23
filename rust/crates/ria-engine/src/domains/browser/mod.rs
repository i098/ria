// Ria original, no Ardia source.

//! Engine domains B: extensions, media, tabs and windows (plus the `webView` settings).
//!
//! [`route`] maps each call name of `api.ts` in these domains to what `ExtensionsModule.swift`, `MediaModule.swift`,
//! `TabsModule.swift`, `WindowsModule.swift` and `WebViewModule.swift` of Ardia do with it: a call that goes to an engine
//! symbol (`ac_*`), with the change of the argument shape and of the reply, or a call that the host answers with AppKit
//! code (`ria-host-mac`). [`call_engine`] runs the first kind.

use std::path::Path;
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
    /// The folder of the profile, as `Engine::call` takes it.
    pub profile_dir: String,
    pub private: bool,
    /// The first preferred language of the app, for extension locale files.
    pub ui_locale: String,
}

/// Sends an event to the handlers of the app (`ArdiaEvents emit:` of Ardia).
pub type Emit = Rc<dyn Fn(&str, Value)>;

/// The route of a call, for example `extensions.list`. `None` when the name is not a call of these domains.
pub fn route(name: &str) -> Option<Route> {
    let (domain, call) = name.split_once('.')?;
    match domain {
        "extensions" => extensions::route(call),
        "media" => media::route(call),
        "tabs" => tabs::route(call),
        "webView" => web_view::route(call),
        "windows" => windows::route(call),
        _ => None,
    }
}

/// The name in `api.ts` of an engine event topic that these domains handle. The C name `tabs.strip` is `tabs.strips`.
pub fn event_name(topic: &str) -> Option<&'static str> {
    match topic {
        "tabs.strip" => Some("tabs.strips"),
        _ => None,
    }
}

/// Runs an engine route: shapes the arguments, calls the symbol, shapes the reply, and emits `extensions.changed`.
/// `completion` runs on a drain, once. When the arguments do not fit the call, it returns the error and does not run
/// `completion`.
pub fn call_engine(
    engine: &Engine,
    route: &EngineRoute,
    ctx: &CallContext,
    args: &Value,
    emit: Emit,
    completion: impl FnOnce(CallResult) + 'static,
) -> Result<(), CallError> {
    let shaped = (route.shape)(args)?;
    let (reply_fn, changed_fn) = (route.reply, route.changed);
    let (ctx, args) = (ctx.clone(), args.clone());
    let profile_dir = ctx.profile_dir.clone();
    engine.call(
        route.symbol,
        &profile_dir,
        ctx.private,
        shaped,
        move |result| {
            let result = result.and_then(|reply| {
                if let Some((id, event)) = changed_fn(&args, &reply) {
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
    Ok(())
}

/// An engine route that passes its arguments and its reply as they are.
pub(crate) const fn passing(symbol: &'static str) -> Route {
    Route::Engine(EngineRoute {
        symbol,
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

pub(crate) fn dir_of(ctx: &CallContext) -> &Path {
    Path::new(&ctx.profile_dir)
}

/// Each call of `domain` in the generated API file `api_source` has a route, and each route name is in the file.
#[cfg(test)]
pub(crate) fn assert_routes_cover(domain: &str, api_source: &str) {
    let marker = format!("const NAME: &'static str = \"{domain}.");
    let names: Vec<&str> = api_source
        .lines()
        .filter_map(|l| l.trim().strip_prefix(&marker)?.strip_suffix("\";"))
        .collect();
    assert!(!names.is_empty(), "no call of {domain} in the API file");
    for name in names {
        assert!(
            route(&format!("{domain}.{name}")).is_some(),
            "{domain}.{name} has no route"
        );
    }
    assert!(route(&format!("{domain}.noSuchCall")).is_none());
}
