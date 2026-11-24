// Ported from ardia packages/ardiacore/ios/EngineCall.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The call table of a domain: a call name, the profile name and a JSON object of arguments go in, one JSON text or one
//! error code and message come out. The JavaScript side of Ardia sees the same codes (`ERR_UNKNOWN_CALL`, `ERR_BAD_ARGS`,
//! `ERR_ENGINE`, `ERR_BAD_REPLY`), so they stay as they are.

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

/// The error of a call that the host rejects: a code and a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    pub code: String,
    pub message: String,
}

impl Rejection {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

/// The settled result of a call: the reply as JSON text (`promise.resolve`) or a rejection (`promise.reject`).
pub type Settled = Result<String, Rejection>;

/// `BadCallArgs`: the arguments of a call are not what the handler needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadCallArgs(pub String);

/// `CallArgs`: the profile name and the JSON object of a call, with typed readers.
#[derive(Debug, Clone)]
pub struct CallArgs {
    pub profile: String,
    values: Map<String, Value>,
}

impl CallArgs {
    pub fn new(profile: &str, json: &str) -> Result<Self, BadCallArgs> {
        match serde_json::from_str::<Value>(json) {
            Ok(Value::Object(values)) => Ok(Self {
                profile: profile.to_owned(),
                values,
            }),
            _ => Err(BadCallArgs("the arguments aren't a JSON object".into())),
        }
    }

    /// The whole object of the call, as the engine takes it (`json`). Moves the arguments: no copy of the payload.
    pub fn into_object(self) -> Value {
        Value::Object(self.values)
    }

    /// The value of `key`, `None` when it is missing or null. `kind` names the type in the message.
    fn optional<T>(
        &self,
        key: &str,
        kind: &str,
        read: impl FnOnce(&Value) -> Option<T>,
    ) -> Result<Option<T>, BadCallArgs> {
        match self.values.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(raw) => read(raw)
                .map(Some)
                .ok_or_else(|| BadCallArgs(format!("{key} isn't {kind}"))),
        }
    }

    fn required<T>(
        &self,
        key: &str,
        kind: &str,
        read: impl FnOnce(&Value) -> Option<T>,
    ) -> Result<T, BadCallArgs> {
        self.optional(key, kind, read)?
            .ok_or_else(|| BadCallArgs(format!("{key} is missing")))
    }

    pub fn text(&self, key: &str) -> Result<String, BadCallArgs> {
        self.required(key, "a string", |v| v.as_str().map(str::to_owned))
    }

    pub fn bool(&self, key: &str) -> Result<bool, BadCallArgs> {
        self.required(key, "a boolean", Value::as_bool)
    }

    pub fn optional_bool(&self, key: &str) -> Result<Option<bool>, BadCallArgs> {
        self.optional(key, "a boolean", Value::as_bool)
    }

    pub fn double(&self, key: &str) -> Result<f64, BadCallArgs> {
        self.required(key, "a number", Value::as_f64)
    }

    pub fn optional_double(&self, key: &str) -> Result<Option<f64>, BadCallArgs> {
        self.optional(key, "a number", Value::as_f64)
    }

    pub fn dict(&self, key: &str) -> Result<Map<String, Value>, BadCallArgs> {
        self.required(key, "an object", |v| v.as_object().cloned())
    }

    pub fn dicts(&self, key: &str) -> Result<Vec<Map<String, Value>>, BadCallArgs> {
        self.required(key, "a list of objects", |v| {
            v.as_array()?
                .iter()
                .map(|item| item.as_object().cloned())
                .collect()
        })
    }

    pub fn strings(&self, key: &str) -> Result<Vec<String>, BadCallArgs> {
        self.required(key, "a list of strings", |v| {
            v.as_array()?
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
    }

    /// A string that is one of the values of `T` (`choice`).
    pub fn choice<T: DeserializeOwned>(&self, key: &str) -> Result<T, BadCallArgs> {
        let raw = self.text(key)?;
        serde_json::from_value(Value::String(raw))
            .map_err(|_| BadCallArgs(format!("{key} isn't one of the values it takes")))
    }
}

/// `CallReply`: answers one call, once.
pub struct CallReply(Box<dyn FnOnce(Settled)>);

impl CallReply {
    pub fn new(settle: impl FnOnce(Settled) + 'static) -> Self {
        Self(Box::new(settle))
    }

    /// `ok`: the value as JSON; `None` is `null`. A number that is not finite becomes `null`.
    pub fn ok(self, value: Option<Value>) {
        let value = value.unwrap_or(Value::Null);
        match serde_json::to_string(&value) {
            Ok(json) => (self.0)(Ok(json)),
            Err(_) => self.fail("ERR_BAD_REPLY", "the reply isn't JSON"),
        }
    }

    pub fn settle(self, settled: Settled) {
        (self.0)(settled);
    }

    pub fn fail(self, code: &str, message: impl Into<String>) {
        (self.0)(Err(Rejection::new(code, message)));
    }
}

/// One handler of a table. `ctx` is whatever the domain needs (the shared context of the data domains).
pub type Handler<C> = fn(&C, CallArgs, CallReply) -> Result<(), BadCallArgs>;

/// `CallTable`: the handlers of one domain.
pub struct CallTable<C: 'static> {
    pub domain: &'static str,
    pub handlers: &'static [(&'static str, Handler<C>)],
}

impl<C: 'static> CallTable<C> {
    /// Runs the call `name`. `reply` settles exactly once, also when the arguments are wrong.
    pub fn run(&self, ctx: &C, name: &str, profile: &str, args: &str, reply: CallReply) {
        let Some((_, handler)) = self.handlers.iter().find(|(n, _)| *n == name) else {
            return reply.fail(
                "ERR_UNKNOWN_CALL",
                format!("{}.{name} isn't a call", self.domain),
            );
        };
        let parsed = CallArgs::new(profile, args);
        // The handler takes the reply, so a rejection for bad arguments needs its own copy of the settle function.
        let shared = std::rc::Rc::new(std::cell::RefCell::new(Some(reply)));
        let given = CallReply::new({
            let shared = shared.clone();
            move |settled| {
                if let Some(reply) = shared.borrow_mut().take() {
                    (reply.0)(settled)
                }
            }
        });
        let outcome = parsed.and_then(|parsed| handler(ctx, parsed, given));
        if let Err(BadCallArgs(message)) = outcome
            && let Some(reply) = shared.borrow_mut().take()
        {
            reply.fail("ERR_BAD_ARGS", format!("{}.{name}: {message}", self.domain));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use serde_json::json;

    use super::*;

    fn run(handlers: &'static [(&'static str, Handler<()>)], name: &str, args: &str) -> Settled {
        let table = CallTable {
            domain: "demo",
            handlers,
        };
        let out = Rc::new(RefCell::new(None));
        let sink = out.clone();
        table.run(
            &(),
            name,
            "p",
            args,
            CallReply::new(move |s| *sink.borrow_mut() = Some(s)),
        );
        out.take().expect("the call settles")
    }

    fn echo(_: &(), args: CallArgs, reply: CallReply) -> Result<(), BadCallArgs> {
        let text = args.text("text")?;
        reply.ok(Some(json!({ "echo": text })));
        Ok(())
    }

    static HANDLERS: &[(&str, Handler<()>)] = &[("echo", echo)];

    #[test]
    fn a_call_gives_json_and_bad_input_gives_codes() {
        assert_eq!(
            run(HANDLERS, "echo", r#"{"text":"hi"}"#),
            Ok(r#"{"echo":"hi"}"#.to_owned())
        );
        assert_eq!(
            run(HANDLERS, "echo", "{}"),
            Err(Rejection::new("ERR_BAD_ARGS", "demo.echo: text is missing"))
        );
        assert_eq!(
            run(HANDLERS, "echo", r#"{"text":3}"#),
            Err(Rejection::new(
                "ERR_BAD_ARGS",
                "demo.echo: text isn't a string"
            ))
        );
        assert_eq!(
            run(HANDLERS, "echo", "[1]"),
            Err(Rejection::new(
                "ERR_BAD_ARGS",
                "demo.echo: the arguments aren't a JSON object"
            ))
        );
        assert_eq!(
            run(HANDLERS, "nope", "{}"),
            Err(Rejection::new("ERR_UNKNOWN_CALL", "demo.nope isn't a call"))
        );
    }
}
