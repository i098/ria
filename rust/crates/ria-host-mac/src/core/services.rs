// Ported from ardia packages/ardiacore/ios/ArdiaServices.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The engine services of the host: each call of `ArdiaServices` as a method of [`Services`]. A method resolves the
//! profile by name, makes the engine call through `ria_engine::call::Engine`, and shapes the reply as Ardia does.
//!
//! Changes from Ardia:
//! - A reply is `Result<Value, String>` (`Err` is the `error` text) instead of the `(value, error)` block pair.
//! - `exportPasswords` has no save panel: a missing `path` answers `{"status": "cancelled"}`, as a cancelled panel does.
//!   The window slice owns the panel.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2_foundation::{NSBundle, NSString};
use ria_engine::call::{CallError, CallResult, Engine};
use ria_engine::domains::browser::{self, CallContext, Route, extensions};
use serde_json::{Map, Value, json};
use zeroize::Zeroize;

use super::bindings::{Profile, Tab};

pub type Reply = Box<dyn FnOnce(Result<Value, String>)>;

/// What `ArdiaServices` reads from the rest of the host. Every method runs on the main thread.
pub trait ServiceHost {
    /// `ardia_host::IsPrivateProfile`.
    fn is_private(&self, profile: &str) -> bool;
    /// `ardia_host::OriginalProfileName`.
    fn original_name(&self, profile: &str) -> String;
    /// `ardia_host::WithProfile`: runs `done` with the loaded profile, or with `None` when it cannot load.
    fn with_profile(&self, name: &str, done: Box<dyn FnOnce(Option<Retained<Profile>>)>);
    /// `ArdiaEvents emit:data:`.
    fn emit(&self, topic: &str, data: Value);
    /// The app tab ids (JSON strings) of the tab that has this Chrome tab id, in any window and loaded profile.
    fn app_tab_ids(&self, chrome_tab_id: i64) -> Vec<Value>;
    /// `[ArdiaEarlyWindow forgetPictures]`.
    fn forget_early_window_pictures(&self);
}

/// The services. Cloning is cheap; each clone shares the same state.
#[derive(Clone)]
pub struct Services {
    engine: Engine,
    host: Rc<dyn ServiceHost>,
    download_profiles: Rc<RefCell<HashMap<String, String>>>,
    observing: Rc<Cell<bool>>,
}

impl Services {
    pub fn new(engine: Engine, host: Rc<dyn ServiceHost>) -> Self {
        Self {
            engine,
            host,
            download_profiles: Rc::default(),
            observing: Rc::default(),
        }
    }

    /// `+call:profile:args:completion:`. The completion gets the reply object; an engine failure is `{"error": text}`.
    pub fn call(
        &self,
        name: &str,
        profile: &str,
        args: Option<Value>,
        done: impl FnOnce(Value) + 'static,
    ) {
        let private = self.host.is_private(profile);
        let engine = self.engine.clone();
        let name = name.to_owned();
        let args = args.unwrap_or_else(|| json!({}));
        self.host.with_profile(
            &self.host.original_name(profile),
            Box::new(move |p| match p {
                None => done(error("profile not loaded")),
                Some(p) => engine.call(&name, &p.path().to_string(), private, args, move |r| {
                    done(parse(r))
                }),
            }),
        );
    }

    /// `+callBase:profile:args:completion:`: the call is never private; the base profile answers.
    pub fn call_base(
        &self,
        name: &str,
        profile: &str,
        args: Option<Value>,
        done: impl FnOnce(Value) + 'static,
    ) {
        self.call(name, &self.host.original_name(profile), args, done);
    }

    /// `+call:profile:args:secrets:completion:`. Each secret travels as `<key>Hex` and its buffer is zeroed.
    pub fn call_with_secrets(
        &self,
        name: &str,
        profile: &str,
        args: Value,
        secrets: &[(&str, &str)],
        done: impl FnOnce(Value) + 'static,
    ) {
        let mut buffer = with_secrets(&args, secrets);
        if self.host.is_private(profile) {
            buffer.zeroize();
            return done(error("private profile"));
        }
        let engine = self.engine.clone();
        let name = name.to_owned();
        let host = self.host.clone();
        host.with_profile(
            &self.host.original_name(profile),
            Box::new(move |p| match p {
                Some(p) if !p.off_the_record() => {
                    engine.call_secret(&name, &p.path().to_string(), false, buffer, move |r| {
                        done(parse(r))
                    })
                }
                _ => {
                    buffer.zeroize();
                    done(error("profile not loaded"))
                }
            }),
        );
    }

    fn base_answer(&self, name: &str, profile: &str, args: Option<Value>, reply: Reply) {
        self.call_base(name, profile, args, move |r| reply(answer(r)));
    }

    // ---- passwords ----

    pub fn list_passwords(&self, profile: &str, reply: Reply) {
        self.call_base("ac_passwords_list", profile, None, answering(reply, |result| {
            let Some(list) = result["passwords"].as_array() else { return result };
            let passwords: Vec<Value> = list
                .iter()
                .map(|e| json!({"origin": text(&e["origin"]), "username": text(&e["username"]), "created": 0, "modified": 0}))
                .collect();
            json!({"passwords": passwords})
        }));
    }

    pub fn unlock_passwords(&self, profile: &str, reply: Reply) {
        self.base_answer("ac_passwords_unlock", profile, None, reply);
    }

    pub fn password(&self, profile: &str, origin: &str, username: &str, reply: Reply) {
        self.base_answer(
            "ac_passwords_reveal",
            profile,
            Some(login(origin, username)),
            reply,
        );
    }

    pub fn save_password(
        &self,
        profile: &str,
        origin: &str,
        username: &str,
        password: &str,
        reply: Reply,
    ) {
        self.call_with_secrets(
            "ac_passwords_add",
            profile,
            login(origin, username),
            &[("password", password)],
            move |r| reply(answer(r)),
        );
    }

    pub fn update_password(
        &self,
        profile: &str,
        origin: &str,
        username: &str,
        new_username: Option<&str>,
        new_password: Option<&str>,
        reply: Reply,
    ) {
        let mut args = login(origin, username);
        if let Some(new_username) = new_username {
            args["newUsername"] = json!(new_username);
        }
        let secrets: Vec<(&str, &str)> = new_password
            .map(|p| ("newPassword", p))
            .into_iter()
            .collect();
        self.call_with_secrets("ac_passwords_update", profile, args, &secrets, move |r| {
            reply(answer(r))
        });
    }

    pub fn delete_password(&self, profile: &str, origin: &str, username: &str, reply: Reply) {
        self.base_answer(
            "ac_passwords_remove",
            profile,
            Some(login(origin, username)),
            reply,
        );
    }

    pub fn never_save_origins(&self, profile: &str, reply: Reply) {
        self.base_answer("ac_passwords_exceptions", profile, None, reply);
    }

    pub fn allow_saving(&self, profile: &str, origin: &str, reply: Reply) {
        self.base_answer(
            "ac_passwords_allow",
            profile,
            Some(json!({"origin": login_origin(origin)})),
            reply,
        );
    }

    pub fn export_passwords(&self, profile: &str, path: Option<&str>, reply: Reply) {
        let Some(target) = path.filter(|p| !p.is_empty()).map(str::to_owned) else {
            return reply(Ok(json!({"status": "cancelled"})));
        };
        let shown = target.clone();
        self.call_base(
            "ac_passwords_export",
            profile,
            Some(json!({"path": target})),
            answering(reply, move |mut result| {
                if result["status"] == "succeeded" {
                    result["path"] = json!(shown);
                }
                result
            }),
        );
    }

    // ---- profile switches ----

    pub fn profile_toggles(&self, profile: &str, set: Option<Map<String, Value>>, reply: Reply) {
        let wanted = set.unwrap_or_default();
        self.host.with_profile(
            &self.host.original_name(profile),
            Box::new(move |p| {
                let Some(p) = p.filter(|p| !p.destroyed()) else { return reply(Err("profile not loaded".into())) };
                if let Some(on) = wanted.get("passwords").map(truthy) {
                    p.set_passwords_offered(on);
                }
                if let Some(on) = wanted.get("addresses").map(truthy) {
                    p.set_addresses_autofilled(on);
                }
                if let Some(on) = wanted.get("cards").map(truthy) {
                    p.set_cards_autofilled(on);
                }
                reply(Ok(json!({"passwords": p.passwords_offered(), "addresses": p.addresses_autofilled(), "cards": p.cards_autofilled()})))
            }),
        );
    }

    pub fn preload_pages(&self, profile: &str, reply: Reply) {
        self.host.with_profile(
            &self.host.original_name(profile),
            Box::new(move |p| {
                reply(
                    p.map(|p| json!(p.preload_pages()))
                        .ok_or_else(|| "profile not loaded".into()),
                )
            }),
        );
    }

    pub fn set_preload_pages(&self, on: bool, profile: &str, reply: Reply) {
        self.host.with_profile(
            &self.host.original_name(profile),
            Box::new(move |p| match p {
                None => reply(Err("profile not loaded".into())),
                Some(p) => {
                    p.set_preload_pages(on);
                    reply(Ok(Value::Null));
                }
            }),
        );
    }

    // ---- autofill ----

    pub fn addresses(&self, profile: &str, reply: Reply) {
        self.call_base(
            "ac_autofill_addresses",
            profile,
            None,
            dated("addresses", reply),
        );
    }

    pub fn save_address(&self, address: &Map<String, Value>, profile: &str, reply: Reply) {
        // Only strings and numbers survive, as strings (`[value description]`).
        let fields: Map<String, Value> = address
            .iter()
            .filter_map(|(k, v)| match v {
                Value::String(s) => Some((k.clone(), json!(s))),
                Value::Number(n) => Some((k.clone(), json!(n.to_string()))),
                _ => None,
            })
            .collect();
        self.base_answer(
            "ac_autofill_save_address",
            profile,
            Some(json!({"address": fields})),
            reply,
        );
    }

    pub fn cards(&self, profile: &str, reply: Reply) {
        self.call_base("ac_autofill_cards", profile, None, dated("cards", reply));
    }

    pub fn save_card(&self, card: Value, number: Option<&str>, profile: &str, reply: Reply) {
        let secrets: Vec<(&str, &str)> = number.map(|n| ("number", n)).into_iter().collect();
        self.call_with_secrets(
            "ac_autofill_save_card",
            profile,
            json!({"card": card}),
            &secrets,
            move |r| reply(answer(r)),
        );
    }

    pub fn delete_autofill_entry(&self, entry_id: &str, profile: &str, reply: Reply) {
        self.base_answer(
            "ac_autofill_remove",
            profile,
            Some(json!({"id": entry_id})),
            reply,
        );
    }

    pub fn reveal_card_number(&self, card_id: &str, profile: &str, reply: Reply) {
        self.call_base(
            "ac_autofill_card_number",
            profile,
            Some(json!({"id": card_id})),
            answering(reply, |result| match result["number"].as_str() {
                Some(number) => json!({"number": number}),
                None => json!({}),
            }),
        );
    }

    // ---- zoom ----

    pub fn zoom_levels(&self, profile: &str, reply: Reply) {
        if self.host.is_private(profile) {
            return reply(Ok(json!({})));
        }
        self.call(
            "ac_zoom_list",
            profile,
            None,
            answering(reply, |result| {
                let levels: Map<String, Value> = result["levels"]
                    .as_object()
                    .map(|hosts| {
                        hosts
                            .iter()
                            .map(|(h, l)| {
                                (h.clone(), json!(zoom_factor(l.as_f64().unwrap_or(0.0))))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Value::Object(levels)
            }),
        );
    }

    pub fn set_zoom(&self, zoom: f64, profile: &str, host: &str) {
        let host = host.to_lowercase();
        if host.is_empty() || zoom <= 0.0 {
            return;
        }
        self.call(
            "ac_zoom_set",
            profile,
            Some(json!({"host": host, "level": zoom_level(zoom)})),
            |_| {},
        );
    }

    // ---- extensions ----

    pub fn inspect_unpacked(&self, path: &str, reply: Reply) {
        let args = ria_engine::api::extensions::InspectUnpackedArgs { path: path.into() };
        reply(answer(parse(extensions::inspect_unpacked(
            &args,
            &preferred_language(),
        ))));
    }

    pub fn list_extensions(&self, profile: &str, reply: Reply) {
        self.extension_call("list", profile, json!({}), reply);
    }

    pub fn install_extension(&self, path: &str, profile: &str, reply: Reply) {
        self.extension_call("install", profile, json!({"path": path}), reply);
    }

    pub fn install_crx(&self, path: &str, profile: &str, reply: Reply) {
        self.extension_call("installCrx", profile, json!({"path": path}), reply);
    }

    pub fn set_extension_enabled(
        &self,
        extension_id: &str,
        enabled: bool,
        profile: &str,
        reply: Reply,
    ) {
        self.extension_call(
            "setEnabled",
            profile,
            json!({"id": extension_id, "enabled": enabled}),
            reply,
        );
    }

    pub fn uninstall_extension(&self, extension_id: &str, profile: &str, reply: Reply) {
        self.extension_call("uninstall", profile, json!({"id": extension_id}), reply);
    }

    pub fn reload_extension(&self, extension_id: &str, profile: &str, reply: Reply) {
        self.extension_call("reload", profile, json!({"id": extension_id}), reply);
    }

    pub fn configure_extension(
        &self,
        extension_id: &str,
        profile: &str,
        options: &Map<String, Value>,
        reply: Reply,
    ) {
        self.extension_call(
            "configure",
            profile,
            json!({"id": extension_id, "options": options}),
            reply,
        );
    }

    fn extension_call(&self, name: &str, profile: &str, args: Value, reply: Reply) {
        let Some(Route::Engine(route)) = extensions::route(name) else {
            unreachable!("service calls have an extension engine route");
        };
        let engine = self.engine.clone();
        let host = self.host.clone();
        let profile_name = host.original_name(profile);
        let private = host.is_private(profile);
        let ui_locale = if name == "list" {
            preferred_language()
        } else {
            String::new()
        };
        self.host.with_profile(
            &profile_name.clone(),
            Box::new(move |profile| {
                let Some(profile) = profile else {
                    return reply(Err("profile not loaded".into()));
                };
                let ctx = CallContext {
                    profile_name,
                    profile_dir: profile.path().to_string(),
                    private,
                    ui_locale,
                };
                browser::call_engine(
                    &engine,
                    &route,
                    &ctx,
                    args,
                    Rc::new(move |topic, data| host.emit(topic, data)),
                    move |result| reply(answer(parse(result))),
                );
            }),
        );
    }

    pub fn search_engine_list(&self, profile: &str, reply: Reply) {
        self.call_base("ac_search_engines_list", profile, None, answering(reply, |result| {
            json!({"list": {"engines": result.get("engines").cloned().unwrap_or_else(|| json!([]))}})
        }));
    }

    // ---- downloads ----

    /// `+watchDownloadEvents`: forwards `downloads.changed` once, whatever the number of calls.
    pub fn watch_download_events(&self) {
        if self.observing.replace(true) {
            return;
        }
        let (host, profiles) = (self.host.clone(), self.download_profiles.clone());
        self.engine.on_event("downloads.changed", move |payload| {
            let Some(mut download) = payload["download"].as_object().cloned() else {
                return;
            };
            let profile = payload.get("profile").cloned().unwrap_or_else(|| json!(""));
            download.insert("profile".into(), profile.clone());
            if let Some(id) = download.get("id").and_then(Value::as_str) {
                profiles.borrow_mut().insert(id.to_owned(), text(&profile));
            }
            host.emit("downloads.changed", Value::Object(download));
        });
    }

    /// `+watchDownloads:`: emits one `downloads.changed` for each download that the profile lists now.
    pub fn watch_downloads(&self, profile: &str) {
        let (host, profiles, profile) = (
            self.host.clone(),
            self.download_profiles.clone(),
            profile.to_owned(),
        );
        self.call_base("ac_downloads_list", &profile.clone(), None, move |result| {
            for download in result["downloads"].as_array().into_iter().flatten() {
                let Some(mut d) = download.as_object().cloned() else {
                    continue;
                };
                d.insert("profile".into(), json!(profile));
                if let Some(id) = d.get("id").and_then(Value::as_str) {
                    profiles.borrow_mut().insert(id.to_owned(), profile.clone());
                }
                host.emit("downloads.changed", Value::Object(d));
            }
        });
    }

    /// `+downloadCommand:id:`.
    pub fn download_command(&self, command: &str, download_id: &str) {
        let profile = self
            .download_profiles
            .borrow()
            .get(download_id)
            .cloned()
            .unwrap_or_default();
        self.call_base(
            "ac_downloads_command",
            &profile,
            Some(json!({"id": download_id, "command": command})),
            |_| {},
        );
    }

    // ---- site settings ----

    pub fn site_settings(&self, profile: &str, origin: &str, reply: Reply) {
        self.call(
            "ac_site_settings_get",
            profile,
            Some(json!({"origin": origin})),
            answering(reply, |result| {
                if result["settings"].is_object() {
                    result["settings"].clone()
                } else {
                    json!({})
                }
            }),
        );
    }

    pub fn set_site_setting(&self, value: &str, profile: &str, origin: &str, kind: &str) {
        self.call(
            "ac_site_settings_set",
            profile,
            Some(json!({"origin": origin, "type": kind, "value": value})),
            |_| {},
        );
    }

    pub fn site_settings_origins(&self, profile: &str, reply: Reply) {
        self.call(
            "ac_site_settings_origins",
            profile,
            None,
            answering(reply, |result| {
                if result["origins"].is_array() {
                    result["origins"].clone()
                } else {
                    json!([])
                }
            }),
        );
    }

    pub fn reset_site_settings(&self, profile: &str, origin: &str) {
        self.call(
            "ac_site_settings_reset",
            profile,
            Some(json!({"origin": origin})),
            |_| {},
        );
    }

    pub fn clear_site_data(&self, profile: &str, origin: &str, reply: Reply) {
        self.call(
            "ac_site_data_clear",
            profile,
            Some(json!({"origin": origin})),
            move |r| reply(answer(r)),
        );
    }

    // ---- external apps, tasks, components ----

    pub fn external_app_allowances(&self, profile: &str, reply: Reply) {
        self.call_base(
            "ac_external_apps_allowances",
            profile,
            None,
            answering(reply, |result| {
                if result["allowances"].is_array() {
                    result["allowances"].clone()
                } else {
                    json!([])
                }
            }),
        );
    }

    pub fn remove_external_app_allowance(&self, profile: &str, origin: &str, scheme: &str) {
        self.call(
            "ac_external_apps_remove",
            profile,
            Some(json!({"origin": origin, "scheme": scheme})),
            |_| {},
        );
    }

    pub fn tasks(&self, reply: Reply) {
        let host = self.host.clone();
        self.call(
            "ac_tasks_list",
            "",
            None,
            answering(reply, move |result| {
                let tasks: Vec<Value> = result["tasks"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_object)
                    .map(|task| {
                        let mut t = task.clone();
                        let ids: Vec<Value> = task
                            .get("tabIds")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_i64)
                            .flat_map(|chrome_id| host.app_tab_ids(chrome_id))
                            .collect();
                        t.insert("tabIds".into(), Value::Array(ids));
                        Value::Object(t)
                    })
                    .collect();
                Value::Array(tasks)
            }),
        );
    }

    pub fn components(&self, reply: Reply) {
        self.call(
            "ac_components_list",
            "",
            None,
            answering(reply, |result| {
                if result["components"].is_array() {
                    result["components"].clone()
                } else {
                    json!([])
                }
            }),
        );
    }

    /// `+clearBrowsingData:types:since:completion:`. The completion gets the error text, or `None` when all was cleared.
    pub fn clear_browsing_data(
        &self,
        profile: &str,
        types: &[&str],
        since_ms: f64,
        completion: impl FnOnce(Option<String>) + 'static,
    ) {
        if types.contains(&"history") {
            self.host.forget_early_window_pictures();
        }
        self.call(
            "ac_browsing_data_clear",
            profile,
            Some(json!({"types": types, "since": since_ms})),
            move |result| {
                if let Some(e) = result.get("error") {
                    return completion(Some(text(e)));
                }
                completion(
                    (result["ok"] != json!(true))
                        .then(|| "Some browsing data could not be cleared.".to_owned()),
                );
            },
        );
    }

    /// `[tab originForURL:]`, used by the login calls: the origin of a URL, or the text itself when it has none.
    pub fn login_origin_of(url: &str) -> String {
        login_origin(url)
    }
}

// ---- helpers ----

fn error(message: &str) -> Value {
    json!({"error": message})
}

/// `Parse` of `ArdiaServices.mm`: the reply is an object, or an error object.
fn parse(result: CallResult) -> Value {
    match result {
        Ok(v) if v.is_object() => v,
        Ok(_) | Err(CallError::BadReply) => error("bad reply from the engine"),
        Err(CallError::Engine(message)) => error(&message),
        Err(CallError::NotStarted) => error("profile not loaded"),
        Err(CallError::UnknownCall(name)) => error(&format!("the engine has no {name}")),
        Err(CallError::PrivateProfile) => error("private profile"),
        Err(CallError::BadArguments) => error("bad arguments"),
        Err(CallError::Timeout) => error("timeout"),
        Err(CallError::Cancelled) => error("cancelled"),
    }
}

/// `Answer`: the error text, or the whole reply.
fn answer(result: Value) -> Result<Value, String> {
    match result.get("error") {
        Some(e) => Err(text(e)),
        None => Ok(result),
    }
}

/// `Answering`: like [`answer`], with the shape of the reply changed.
fn answering(reply: Reply, shape: impl FnOnce(Value) -> Value + 'static) -> impl FnOnce(Value) {
    move |result| reply(answer(result).map(shape))
}

/// `Dated`: each entry of the list under `key` gets `created` and `modified` set to 0.
fn dated(key: &'static str, reply: Reply) -> impl FnOnce(Value) {
    answering(reply, move |result| {
        let Some(list) = result[key].as_array() else {
            return result;
        };
        let list: Vec<Value> = list
            .iter()
            .filter_map(Value::as_object)
            .map(|e| {
                let mut item = e.clone();
                item.insert("created".into(), json!(0));
                item.insert("modified".into(), json!(0));
                Value::Object(item)
            })
            .collect();
        json!({ key: list })
    })
}

fn preferred_language() -> String {
    NSBundle::mainBundle()
        .preferredLocalizations()
        .firstObject()
        .map_or_else(String::new, |language| language.to_string())
}

/// `LoginOrigin`: the origin of the URL, or the URL itself.
fn login_origin(url: &str) -> String {
    let origin = Tab::origin_for_url(&NSString::from_str(url))
        .map(|o| o.to_string())
        .filter(|o| !o.is_empty());
    origin.unwrap_or_else(|| url.to_owned())
}

fn login(origin: &str, username: &str) -> Value {
    json!({"origin": login_origin(origin), "username": username})
}

fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// `[value boolValue]` for a JSON value.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0),
        Value::String(s) => matches!(s.chars().next(), Some('Y' | 'y' | 'T' | 't' | '1'..='9')),
        _ => false,
    }
}

/// The engine stores zoom as a level; the page uses a factor of 1.2 for each level.
fn zoom_factor(level: f64) -> f64 {
    (1.2f64.powf(level) * 100.0).round() / 100.0
}

fn zoom_level(zoom: f64) -> f64 {
    if (zoom - 1.0).abs() < 1e-6 {
        0.0
    } else {
        zoom.ln() / 1.2f64.ln()
    }
}

/// `WithSecrets`: the arguments as JSON, with each secret added as `"<key>Hex":"<hex of its UTF-8 bytes>"`, then a NUL byte.
fn with_secrets(plain: &Value, secrets: &[(&str, &str)]) -> Vec<u8> {
    let empty = plain.as_object().is_none_or(Map::is_empty);
    let mut out = if empty {
        b"{}".to_vec()
    } else {
        plain.to_string().into_bytes()
    };
    out.pop();
    for (i, (key, secret)) in secrets.iter().enumerate() {
        if !(empty && i == 0) {
            out.push(b',');
        }
        out.extend_from_slice(format!("\"{key}Hex\":\"").as_bytes());
        for byte in secret.as_bytes() {
            out.extend_from_slice(format!("{byte:02x}").as_bytes());
        }
        out.push(b'"');
    }
    out.extend_from_slice(b"}\0");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_added_as_hex_after_the_plain_arguments() {
        let bytes = with_secrets(
            &json!({"origin": "https://a.example", "username": "u"}),
            &[("password", "p\u{e9}")],
        );
        assert_eq!(bytes.last(), Some(&0));
        let parsed: Value = serde_json::from_slice(&bytes[..bytes.len() - 1]).unwrap();
        assert_eq!(parsed["passwordHex"], "70c3a9");
        assert_eq!(parsed["username"], "u");
    }

    #[test]
    fn secrets_alone_make_a_valid_object() {
        let bytes = with_secrets(&json!({}), &[("number", "4"), ("cvc", "12")]);
        let parsed: Value = serde_json::from_slice(&bytes[..bytes.len() - 1]).unwrap();
        assert_eq!(parsed, json!({"numberHex": "34", "cvcHex": "3132"}));
    }

    #[test]
    fn zoom_level_and_factor_are_inverse_at_two_decimals() {
        assert_eq!(zoom_level(1.0), 0.0);
        assert_eq!(zoom_factor(zoom_level(1.44)), 1.44);
    }

    #[test]
    fn dated_entries_get_zero_times_and_non_objects_are_dropped() {
        let out = std::cell::RefCell::new(None);
        let out = Rc::new(out);
        let sink = out.clone();
        dated("cards", Box::new(move |r| *sink.borrow_mut() = Some(r)))(
            json!({"cards": [{"id": "a"}, 3]}),
        );
        assert_eq!(
            out.borrow().clone().unwrap(),
            Ok(json!({"cards": [{"id": "a", "created": 0, "modified": 0}]}))
        );
    }
}
