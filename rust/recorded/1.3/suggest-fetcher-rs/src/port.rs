// Ported from ardia packages/core/src/suggestFetcher.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `omnibox::suggest_fetcher`, through the recording harness
//! `suggestFetcher.harness.ts`: it plays the same script with the I/O the core leaves to its
//! caller (the timer, the fetches and their aborts) and logs the same events. Each step of the
//! harness ends after a macrotask, so the promise work of a step is done before the next one.

#![forbid(unsafe_code)]

use std::collections::VecDeque;

use ria_core::omnibox::suggest_fetcher::{DEFAULT_CACHE_SIZE, Next, SuggestFetcher, Ticket};
use ria_core::search::engines::{EngineExtension, RemoteSuggestions, SearchEngine};
use serde_json::{Value, json};

use crate::js;

/// A JS string value, lone surrogates replaced.
fn string(v: &Value) -> Option<String> {
    js::string_units(v).map(|u| String::from_utf16_lossy(&u))
}

/// `!!v`.
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(x) => match js::as_number(x) {
            Some(n) => n != 0.0 && !n.is_nan(),
            None => !js::is_undefined(x),
        },
    }
}

/// The number of replies kept for a `cacheSize`: the original evicts while `size > cacheSize`.
fn cache_size(size: Option<f64>) -> usize {
    match size {
        None => DEFAULT_CACHE_SIZE,
        Some(n) if n.is_nan() => usize::MAX,
        Some(n) => n.max(0.0).floor() as usize,
    }
}

/// The engine of a request step, as `suggestRequestUrl` reads it.
fn engine(v: Option<&Value>) -> Result<SearchEngine, String> {
    let v = v.unwrap_or(&Value::Null);
    if v.is_null() || js::is_undefined(v) {
        return Err(format!(
            "TypeError: Cannot read properties of {v} (reading 'suggestUrl')"
        ));
    }
    let text = |key: &str| v.get(key).and_then(string).unwrap_or_default();
    let suggest_url = match v.get("suggestUrl") {
        Some(s) if truthy(Some(s)) => Some(
            string(s).ok_or_else(|| "TypeError: template.replace is not a function".to_string())?,
        ),
        _ => None,
    };
    Ok(SearchEngine {
        id: text("id"),
        name: text("name"),
        keyword: text("keyword"),
        url: text("url"),
        suggest_url,
        custom: truthy(v.get("custom")),
        extension: v
            .get("extension")
            .filter(|e| e.is_object())
            .map(|e| EngineExtension {
                id: e.get("id").and_then(string).unwrap_or_default(),
                name: e.get("name").and_then(string).unwrap_or_default(),
            }),
    })
}

fn result(s: &RemoteSuggestions) -> Value {
    let sites: Vec<Value> = s
        .sites
        .iter()
        .map(|x| json!({ "url": x.url, "title": x.title }))
        .collect();
    json!({ "result": { "queries": s.queries, "sites": sites } })
}

/// One fetch the fake `fetch` got.
struct Fetch {
    ticket: Ticket,
    aborted: bool,
}

/// The fetcher with its I/O: the pending timer, the fetches and the one in flight.
struct Player {
    fetcher: SuggestFetcher,
    timer: Option<(Ticket, Value)>,
    fetches: Vec<Fetch>,
    open: VecDeque<usize>,
    in_flight: Option<usize>,
    log: Vec<Value>,
}

impl Player {
    /// The I/O part of `cancel`: clear the timer and abort the fetch in flight.
    fn cancel_io(&mut self) {
        self.timer = None;
        if let Some(i) = self.in_flight.take() {
            self.fetches[i].aborted = true;
            self.log
                .push(json!({ "abort": self.fetches[i].ticket.url }));
        }
    }

    fn request(&mut self, step: &Value) -> Result<(), String> {
        self.cancel_io();
        let engine_value = step.get("engine");
        let engine = engine(engine_value)?;
        let query = step.get("query");
        if engine.suggest_url.is_some() && !engine.custom {
            let Some(query) = query.and_then(string) else {
                self.fetcher.cancel();
                return Err("TypeError: query.trim is not a function".into());
            };
            let id = engine_value
                .and_then(|e| e.get("id"))
                .filter(|id| !js::is_undefined(id))
                .cloned()
                .unwrap_or(Value::Null);
            match self.fetcher.request(&engine, &query) {
                Next::Nothing => {}
                Next::Cached(_, s) => self.log.push(result(&s)),
                Next::Fetch(ticket) => self.timer = Some((ticket, id)),
            }
        } else {
            self.fetcher.cancel();
        }
        Ok(())
    }

    fn fire(&mut self) {
        if let Some((ticket, id)) = self.timer.take() {
            self.log.push(json!({ "fetch": ticket.url, "engine": id }));
            self.in_flight = Some(self.fetches.len());
            self.open.push_back(self.fetches.len());
            self.fetches.push(Fetch {
                ticket,
                aborted: false,
            });
        }
    }

    /// The oldest open fetch answers; `reply`: `None` when it fails outright, `Some(None)` for
    /// a reply without suggestions, `Some(Some(body))` for an ok reply with a body.
    fn answer(&mut self, reply: Option<Option<String>>) {
        let Some(i) = self.open.pop_front() else {
            return;
        };
        let Some(body) = reply else {
            return;
        };
        if self.fetches[i].aborted {
            return;
        }
        if let Some(s) = self
            .fetcher
            .finish(&self.fetches[i].ticket, body.as_deref())
        {
            self.in_flight = None;
            self.log.push(result(&s));
        }
    }
}

/// Port of `runFetcher` (`packages/core/src/suggestFetcher.harness.ts:20`).
pub async fn run_fetcher(steps: Value, cache_size: Option<f64>) -> Result<Vec<Value>, String> {
    let mut p = Player {
        fetcher: SuggestFetcher::new(self::cache_size(cache_size)),
        timer: None,
        fetches: Vec::new(),
        open: VecDeque::new(),
        in_flight: None,
        log: Vec::new(),
    };
    // `for...of` over a string gives its code points: steps of no `op`, with no fetch to answer.
    if js::string_units(&steps).is_some() {
        return Ok(p.log);
    }
    let Value::Array(steps) = steps else {
        return Err("TypeError: steps is not iterable".into());
    };
    for step in &steps {
        if step.is_null() || js::is_undefined(step) {
            return Err(format!(
                "TypeError: Cannot read properties of {step} (reading 'op')"
            ));
        }
        match step.get("op").and_then(Value::as_str) {
            Some("request") => p.request(step)?,
            Some("cancel") => {
                p.cancel_io();
                p.fetcher.cancel();
            }
            Some("fire") => p.fire(),
            Some("fail") => p.answer(None),
            Some("unreadable") => p.answer(Some(None)),
            Some("reply") => {
                let body = truthy(step.get("ok"))
                    .then(|| step.get("body").and_then(string))
                    .flatten();
                p.answer(Some(body));
            }
            _ => p.answer(None),
        }
    }
    Ok(p.log)
}
