// Ported from ardia packages/ardiacore/ios/ProfileData.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
//
// Deviation from Ardia (issue #53, approved by the order to resolve every open issue): Ardia drops the error of every
// service call (`{ value, _ in` at ProfileData.swift:16, 19, 40, 41, 53, 60, 64, 70, 73) and reads a missing or
// wrong-shaped answer as empty (`?? []` at lines 42, 44, 61, 64; `?? [:]` at lines 54, 71, 74, 95), so a failed read
// can report a step as done while data remains. Here a step is complete only after a read that succeeded shows that
// nothing remains; a failed read, a read of the wrong shape or a failed check puts the step label in `remaining`.

//! Deletes the data of the default profile, step by step. A step that does not finish leaves its label in the list
//! that `done` gets (`remaining`).

use std::rc::Rc;
use std::time::Duration;

use serde_json::{Map, Value, json};

use super::ctx::DataCtx;
use super::page_engines::clear_chromium_browsing_data;

type Then = Box<dyn FnOnce()>;
type Ok = Box<dyn FnOnce(bool)>;
type Body<T> = Rc<dyn Fn(T, Then)>;
type Lister = Rc<dyn Fn(Box<dyn FnOnce(Option<Map<String, Value>>)>)>;

/// A native service of the default profile. `then` gets `None` when the service fails, so a failed read is
/// never taken for an empty list.
fn service(ctx: &DataCtx, op: &str, args: Value, then: impl FnOnce(Option<Value>) + 'static) {
    ctx.services
        .run(op, "", args, Box::new(move |result| then(result.ok())));
}

fn listed(value: Option<Value>) -> Option<Map<String, Value>> {
    match value {
        Some(Value::Object(map)) => Some(map),
        _ => None,
    }
}

fn strings(value: Option<Value>) -> Option<Vec<String>> {
    match value {
        Some(Value::Array(a)) => Some(
            a.iter()
                .filter_map(|o| o.as_str().map(str::to_owned))
                .collect(),
        ),
        _ => None,
    }
}

fn for_each<T: 'static>(mut items: impl Iterator<Item = T> + 'static, body: Body<T>, then: Then) {
    let Some(first) = items.next() else {
        return then();
    };
    let rest_body = body.clone();
    body(first, Box::new(move || for_each(items, rest_body, then)));
}

fn text(item: &Map<String, Value>, key: &str) -> String {
    item.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn clear_each(list: Lister, key: &'static str, done: Ok, clear: Body<Map<String, Value>>) {
    let again = list.clone();
    list(Box::new(move |result| {
        let Some(Value::Array(items)) = result.as_ref().and_then(|r| r.get(key)) else {
            return done(false);
        };
        let items: Vec<_> = items
            .iter()
            .filter_map(|i| i.as_object().cloned())
            .collect();
        for_each(
            items.into_iter(),
            clear,
            Box::new(move || {
                again(Box::new(move |after| {
                    done(
                        matches!(after.as_ref().and_then(|a| a.get(key)), Some(Value::Array(a)) if a.is_empty()),
                    )
                }))
            }),
        );
    }));
}

fn service_list(ctx: &DataCtx, op: &'static str) -> Lister {
    let ctx = ctx.clone();
    Rc::new(move |done| service(&ctx, op, json!({}), move |v| done(listed(v))))
}

fn delete_entries(ctx: &DataCtx, list_op: &'static str, key: &'static str, done: Ok) {
    let delete = ctx.clone();
    clear_each(
        service_list(ctx, list_op),
        key,
        done,
        Rc::new(move |item, next| {
            service(
                &delete,
                "deleteAutofillEntry",
                json!({ "id": text(&item, "id") }),
                move |_| next(),
            )
        }),
    );
}

fn delete_passwords(ctx: DataCtx, ok: Ok) {
    let (never, delete) = (ctx.clone(), ctx.clone());
    clear_each(
        service_list(&ctx, "listPasswords"),
        "passwords",
        Box::new(move |deleted| {
            let again = never.clone();
            service(&never, "neverSaveOrigins", json!({}), move |result| {
                let Some(origins) = listed(result).and_then(|mut r| strings(r.remove("origins")))
                else {
                    return ok(false);
                };
                let allow = again.clone();
                for_each(
                    origins.into_iter(),
                    Rc::new(move |origin, next| {
                        service(
                            &allow,
                            "allowSaving",
                            json!({ "origin": origin }),
                            move |_| next(),
                        )
                    }),
                    Box::new(move || {
                        let check = again.clone();
                        service(&check, "neverSaveOrigins", json!({}), move |left| {
                            let empty = listed(left)
                                .and_then(|mut l| strings(l.remove("origins")))
                                .is_some_and(|o| o.is_empty());
                            ok(deleted && empty)
                        })
                    }),
                );
            });
        }),
        Rc::new(move |item, next| {
            service(
                &delete,
                "deletePassword",
                json!({ "origin": text(&item, "origin"), "username": text(&item, "username") }),
                move |_| next(),
            )
        }),
    );
}

fn delete_extensions(ctx: DataCtx, ok: Ok) {
    let (listing, disable) = (ctx.clone(), ctx.clone());
    let list: Lister = Rc::new(move |done| {
        service(&listing, "listExtensions", json!({}), move |result| {
            let all = listed(result).and_then(|mut r| r.remove("extensions"));
            let Some(Value::Array(all)) = all else {
                return done(None);
            };
            let running = all
                .into_iter()
                .filter(|e| {
                    e.get("mayModify").and_then(Value::as_bool) != Some(false)
                        && e.get("enabled").and_then(Value::as_bool) == Some(true)
                })
                .collect();
            done(Some(Map::from_iter([(
                "extensions".to_owned(),
                Value::Array(running),
            )])));
        })
    });
    clear_each(
        list,
        "extensions",
        ok,
        Rc::new(move |item, next| {
            service(
                &disable,
                "setExtension",
                json!({ "id": text(&item, "id"), "enabled": false }),
                move |_| next(),
            )
        }),
    );
}

fn reset_site_settings(ctx: DataCtx, ok: Ok) {
    let (first, reset, last) = (ctx.clone(), ctx.clone(), ctx);
    service(&first, "siteSettingsOrigins", json!({}), move |origins| {
        let Some(origins) = strings(origins) else {
            return ok(false);
        };
        for_each(
            origins.into_iter(),
            Rc::new(move |origin, next| {
                reset.engine_call(
                    "ac_site_settings_reset",
                    "",
                    json!({ "origin": origin }),
                    move |_| next(),
                )
            }),
            Box::new(move || {
                let check = last.clone();
                service(&check, "siteSettingsOrigins", json!({}), move |left| {
                    ok(strings(left).is_some_and(|o| o.is_empty()))
                })
            }),
        );
    });
}

fn reset_zoom(ctx: DataCtx, ok: Ok) {
    let again = ctx.clone();
    service(&ctx, "zoomLevels", json!({}), move |levels| {
        let Some(levels) = listed(levels) else {
            return ok(false);
        };
        for host in levels.keys() {
            service(
                &again,
                "setZoom",
                json!({ "zoom": 1, "host": host }),
                |_| {},
            );
        }
        check_zoom(again, 0, ok);
    });
}

fn check_zoom(ctx: DataCtx, attempt: u32, ok: Ok) {
    let next = ctx.clone();
    service(&ctx, "zoomLevels", json!({}), move |left| {
        let Some(remaining) = listed(left) else {
            return ok(false);
        };
        if remaining.is_empty() || attempt >= 20 {
            return ok(remaining.is_empty());
        }
        (next.after)(
            Duration::from_millis(100),
            Box::new(move || check_zoom(next, attempt + 1, ok)),
        );
    });
}

type StepRun = fn(DataCtx, Ok);

fn steps() -> Vec<(&'static str, StepRun)> {
    vec![
        ("history, cookies and cache", |ctx, ok| {
            let types = ["history", "siteData", "cache", "downloads"].map(str::to_owned);
            clear_chromium_browsing_data(&ctx, "", &types, 0.0, Box::new(move |e| ok(e.is_none())));
        }),
        ("autocomplete entries", |ctx, ok| {
            ctx.engine_call(
                "ac_browsing_data_clear",
                "",
                json!({ "types": ["formData", "siteSettings"] }),
                move |result| ok(result.get("ok") == Some(&Value::Bool(true))),
            );
        }),
        ("passwords", delete_passwords),
        ("addresses", |ctx, ok| {
            delete_entries(&ctx, "addresses", "addresses", ok)
        }),
        ("cards", |ctx, ok| {
            delete_entries(&ctx, "cards", "cards", ok)
        }),
        ("extensions", delete_extensions),
        ("site settings", reset_site_settings),
        ("zoom levels", reset_zoom),
    ]
}

/// `ProfileData.deleteDefault`: `done` gets the labels of the steps that did not finish.
pub fn delete_default(ctx: DataCtx, done: Box<dyn FnOnce(Vec<String>)>) {
    run(ctx, Rc::new(steps()), 0, Vec::new(), done);
}

fn run(
    ctx: DataCtx,
    steps: Rc<Vec<(&'static str, StepRun)>>,
    index: usize,
    mut remaining: Vec<String>,
    done: Box<dyn FnOnce(Vec<String>)>,
) {
    let Some(&(label, step)) = steps.get(index) else {
        return done(remaining);
    };
    let next = ctx.clone();
    step(
        ctx,
        Box::new(move |ok| {
            if !ok {
                remaining.push(label.to_owned());
            }
            run(next, steps, index + 1, remaining, done);
        }),
    );
}
