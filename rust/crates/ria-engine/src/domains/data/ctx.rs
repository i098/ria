// Ported from ardia packages/ardiacore/ios/ArdiaEngineBridge.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The context of the data call tables: the engine call with the profile rules of `ArdiaEngineBridge.call`, the
//! native services, and the two page engines.

use std::rc::Rc;
use std::time::Duration;

use serde_json::{Value, json};

use crate::call::{CallError, Engine};

use super::call_table::CallReply;
use super::favicons::Favicons;
use super::host::{PageEngine, Services};
use super::profiles::{ProfileRegistry, original_profile_name};

/// The reply of an engine call as Ardia sees it: the engine reply, or `{"error": ...}`.
pub fn reply_value(result: Result<Value, CallError>) -> Value {
    match result {
        Ok(value) => value,
        Err(CallError::Engine(message)) => json!({ "error": message }),
        Err(CallError::UnknownCall(name)) => {
            json!({ "error": format!("the engine has no {name}") })
        }
        Err(CallError::BadArguments) => json!({ "error": "bad arguments" }),
        Err(other) => json!({ "error": format!("{other:?}") }),
    }
}

/// Runs work on the main queue after a delay: `call::main_queue::after` on a Mac, a fake in tests. A delay of zero is
/// `main_queue::post`.
pub type After = fn(Duration, Box<dyn FnOnce()>);

/// Everything that a data call needs. Cheap to clone. Main thread only.
#[derive(Clone)]
pub struct DataCtx {
    pub engine: Engine,
    pub registry: ProfileRegistry,
    pub services: Rc<dyn Services>,
    pub chromium: Rc<dyn PageEngine>,
    pub webkit: Rc<dyn PageEngine>,
    pub favicons: Rc<Favicons>,
    pub after: After,
}

impl DataCtx {
    /// `ArdiaEngineBridge.call:profile:args:completion:`. A private profile adds `offTheRecord`; the call runs in the
    /// folder of the original profile, which loads first when it is not loaded. `done` gets the reply of the engine
    /// or `{"error": ...}`.
    pub fn engine_call(
        &self,
        name: &str,
        profile: &str,
        args: Value,
        done: impl FnOnce(Value) + 'static,
    ) {
        let private = self.registry.is_private_profile(profile);
        if private && !args.is_object() {
            return done(json!({ "error": "bad arguments" }));
        }
        let engine = self.engine.clone();
        let name = name.to_owned();
        self.registry
            .with_profile(&original_profile_name(profile), move |loaded| {
                let Some(loaded) = loaded else {
                    return done(json!({ "error": "profile not loaded" }));
                };
                engine.call(&name, &loaded.path(), private, args, move |result| {
                    done(reply_value(result))
                });
            });
    }

    /// `EngineCall.passing(name)`: sends the arguments as they are and answers with the engine reply.
    pub fn pass(&self, name: &str, profile: &str, args: Value, reply: CallReply) {
        self.engine_call(name, profile, args, move |value| reply.engine_value(value));
    }

    /// `reply.service(value, error)` for a native service.
    pub fn service(&self, op: &str, profile: &str, args: Value, reply: CallReply) {
        self.services.run(
            op,
            profile,
            args,
            Box::new(move |result| match result {
                Ok(value) => reply.ok(Some(value)),
                Err(error) => reply.fail("ERR_ENGINE", error),
            }),
        );
    }

    /// `PageEngines.owner(of:)`: WebKit owns the ids with the prefix `wk-`, Chromium owns the rest.
    pub fn owner(&self, id: &str) -> &Rc<dyn PageEngine> {
        [&self.webkit, &self.chromium]
            .into_iter()
            .find(|engine| engine.owns(id))
            .unwrap_or(&self.chromium)
    }
}

impl CallReply {
    /// `engine`, for a reply that is already a value.
    pub fn engine_value(self, value: Value) {
        match value.get("error") {
            Some(Value::String(message)) => self.fail("ERR_ENGINE", message.clone()),
            Some(other) => self.fail("ERR_ENGINE", other.to_string()),
            None => self.settle(Ok(value.to_string())),
        }
    }
}
