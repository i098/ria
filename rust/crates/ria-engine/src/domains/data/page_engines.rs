// Ported from ardia packages/ardiacore/ios/PageEngine.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `PageEngines`: the calls that go to both page engines, and the Chromium part of clearing browsing data
//! (`ArdiaServices.clearBrowsingData`). The engine `PageEngine` is in `host.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::{Value, json};

use std::time::Duration;

use super::ctx::DataCtx;

/// `ArdiaServices.clearBrowsingData`: the engine call `ac_browsing_data_clear`. `done` gets `None` when the engine
/// answers a boolean `true`, and an error text otherwise. When the types hold `history`, the pictures of the early
/// window are forgotten first.
pub fn clear_chromium_browsing_data(
    ctx: &DataCtx,
    profile: &str,
    types: &[String],
    since: f64,
    done: Box<dyn FnOnce(Option<String>)>,
) {
    if types.iter().any(|t| t == "history") {
        ctx.services
            .run("forgetPictures", "", json!({}), Box::new(|_| {}));
    }
    ctx.engine_call(
        "ac_browsing_data_clear",
        profile,
        json!({ "types": types, "since": since }),
        move |result| {
            if let Some(error) = result.get("error") {
                return done(Some(match error {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                }));
            }
            done(if result.get("ok") == Some(&Value::Bool(true)) {
                None
            } else {
                Some("Some browsing data could not be cleared.".to_owned())
            });
        },
    );
}

/// `PageEngines.clearBrowsingData`: runs on both engines; `done` runs once, after both answered, with the first error.
pub fn clear_browsing_data(
    ctx: &DataCtx,
    profile: &str,
    types: &[String],
    since: f64,
    done: Box<dyn FnOnce(Option<String>)>,
) {
    let after = ctx.after;
    let state = Rc::new(RefCell::new((2_usize, None::<String>, Some(done))));
    for engine in [&ctx.webkit, &ctx.chromium] {
        let state = state.clone();
        engine.clear_browsing_data(
            profile,
            types,
            since,
            // Ardia hops to the main queue before it counts, so the answer is never synchronous.
            Box::new(move |error| {
                after(
                    Duration::ZERO,
                    Box::new(move || {
                        let mut state = state.borrow_mut();
                        if state.1.is_none() {
                            state.1 = error;
                        }
                        state.0 -= 1;
                        if state.0 == 0
                            && let Some(done) = state.2.take()
                        {
                            let failure = state.1.take();
                            drop(state);
                            done(failure);
                        }
                    }),
                );
            }),
        );
    }
}

type DeleteState = RefCell<(
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<Box<dyn FnOnce(Vec<String>)>>,
)>;

/// `PageEngines.deleteProfileData`: both engines; `done` gets the data that is left, Chromium first.
pub fn delete_profile_data(ctx: &DataCtx, profile: &str, done: Box<dyn FnOnce(Vec<String>)>) {
    let state = Rc::new(RefCell::new((
        None::<Vec<String>>,
        None::<Vec<String>>,
        Some(done),
    )));
    let finish = |state: &Rc<DeleteState>| {
        let mut s = state.borrow_mut();
        if let (Some(c), Some(w)) = (&s.0, &s.1) {
            let all = [c.clone(), w.clone()].concat();
            if let Some(done) = s.2.take() {
                drop(s);
                done(all);
            }
        }
    };
    let (a, b) = (state.clone(), state.clone());
    ctx.chromium.delete_profile_data(
        profile,
        Box::new(move |left| {
            a.borrow_mut().0 = Some(left);
            finish(&a);
        }),
    );
    ctx.webkit.delete_profile_data(
        profile,
        Box::new(move |left| {
            b.borrow_mut().1 = Some(left);
            finish(&b);
        }),
    );
}

/// `PageEngines.externalAppAllowances`: Chromium first, then the WebKit ones that Chromium does not have
/// (same `origin` and `scheme`).
pub fn external_app_allowances(ctx: &DataCtx, profile: &str, done: Box<dyn FnOnce(Vec<Value>)>) {
    let webkit = ctx.webkit.clone();
    let profile_owned = profile.to_owned();
    ctx.chromium.external_app_allowances(
        profile,
        Box::new(move |chromium| {
            webkit.external_app_allowances(
                &profile_owned,
                Box::new(move |webkit| {
                    let key = |a: &Value| {
                        let part = |k: &str| {
                            a.get(k)
                                .map(|v| v.as_str().map_or(v.to_string(), str::to_owned))
                                .unwrap_or_default()
                        };
                        format!("{}|{}", part("origin"), part("scheme"))
                    };
                    let seen: Vec<String> = chromium.iter().map(key).collect();
                    let mut all = chromium;
                    all.extend(webkit.into_iter().filter(|a| !seen.contains(&key(a))));
                    done(all);
                }),
            );
        }),
    );
}

pub fn remove_external_app_allowance(ctx: &DataCtx, profile: &str, origin: &str, scheme: &str) {
    for engine in [&ctx.webkit, &ctx.chromium] {
        engine.remove_external_app_allowance(profile, origin, scheme);
    }
}
