// Ported from ardia packages/ardiacore/ios/AutofillModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The call tables of the data domains: autofill, bookmarks, downloads, favicons, history, passwords, permissions,
//! profiles, search and sites (`AutofillModule.swift` to `SitesModule.swift`). A handler of an engine call sends the
//! arguments as they are to the engine symbol (`EngineCall.passing`); a handler of a native call goes through
//! `Services`. The name of each call and its symbol are the same as in Ardia.

use serde_json::{Map, Value, json};

use crate::types::{PageEngine, PermissionResult};

use super::call_table::{BadCallArgs, CallArgs, CallReply, CallTable, Handler};
use super::content_blocker::ContentBlocker;
use super::content_blocker_module::CALLS;
use super::ctx::DataCtx;
use super::page_engines;

type Done = Result<(), BadCallArgs>;

/// `EngineCall.passing(symbol)`.
macro_rules! passing {
    ($symbol:literal) => {
        |ctx: &DataCtx, args: CallArgs, reply: CallReply| -> Done {
            ctx.pass($symbol, &args.profile, args.object(), reply);
            Ok(())
        }
    };
}

/// A native service with the same arguments as the call has: `args` is the whole object.
macro_rules! service {
    ($op:literal) => {
        |ctx: &DataCtx, args: CallArgs, reply: CallReply| -> Done {
            ctx.service($op, &args.profile, json!({}), reply);
            Ok(())
        }
    };
    ($op:literal, $($key:literal : $read:ident),+) => {
        |ctx: &DataCtx, args: CallArgs, reply: CallReply| -> Done {
            let payload = json!({ $($key: args.$read($key)?),+ });
            ctx.service($op, &args.profile, payload, reply);
            Ok(())
        }
    };
}

fn toggles_error(error: String) -> String {
    error
}

/// Reads the `profileToggles` answer: an object of booleans.
fn toggle(value: &Value, key: &str) -> Option<bool> {
    value.as_object()?.get(key)?.as_bool()
}

fn settings(ctx: &DataCtx, args: CallArgs, reply: CallReply) -> Done {
    ctx.services.run(
        "profileToggles",
        &args.profile,
        json!({ "set": null }),
        Box::new(move |result| match result {
            Ok(toggles) if toggles.is_object() => reply.ok(Some(json!({
                "addresses": toggle(&toggles, "addresses").unwrap_or(true),
                "cards": toggle(&toggles, "cards").unwrap_or(true),
            }))),
            Ok(_) => reply.fail("ERR_ENGINE", "no autofill settings"),
            Err(error) => reply.fail("ERR_ENGINE", toggles_error(error)),
        }),
    );
    Ok(())
}

fn set_toggles(ctx: &DataCtx, profile: &str, values: Map<String, Value>, reply: CallReply) {
    ctx.services.run(
        "profileToggles",
        profile,
        json!({ "set": values }),
        Box::new(move |result| match result {
            Ok(_) => reply.ok(None),
            Err(error) => reply.fail("ERR_ENGINE", error),
        }),
    );
}

fn set_settings(ctx: &DataCtx, args: CallArgs, reply: CallReply) -> Done {
    let mut values = Map::new();
    for key in ["addresses", "cards"] {
        if let Some(on) = args.optional_bool(key)? {
            values.insert(key.to_owned(), Value::Bool(on));
        }
    }
    set_toggles(ctx, &args.profile, values, reply);
    Ok(())
}

/// `AsyncFunction("saveCard")` of `AutofillModule`.
pub fn save_card(
    ctx: &DataCtx,
    profile: &str,
    card: Value,
    number: Option<String>,
    reply: CallReply,
) {
    ctx.service(
        "saveCard",
        profile,
        json!({ "card": card, "number": number }),
        reply,
    );
}

fn autofill_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("settings", settings),
        ("setSettings", set_settings),
        ("addresses", service!("addresses")),
        ("saveAddress", |ctx, args, reply| {
            let payload = json!({ "address": args.dict("address")? });
            ctx.service("saveAddress", &args.profile, payload, reply);
            Ok(())
        }),
        ("cards", service!("cards")),
        ("remove", service!("deleteAutofillEntry", "id": text)),
        ("cardNumber", service!("revealCardNumber", "id": text)),
        (
            "showSuggestions",
            service!("showAutofillSuggestions", "tabId": text, "passwords": bool),
        ),
    ]
}

fn export_html(ctx: &DataCtx, args: CallArgs, reply: CallReply) -> Done {
    let ctx = ctx.clone();
    let profile = args.profile.clone();
    ctx.services.clone().run(
        "savePanel",
        &args.profile,
        json!({ "name": "Ardia Bookmarks.html", "type": "html" }),
        Box::new(move |chosen| {
            let Ok(Value::String(path)) = chosen else {
                return reply.ok(None);
            };
            let sent = path.clone();
            ctx.engine_call(
                "ac_bookmarks_export_html",
                &profile,
                json!({ "path": sent }),
                move |result| match result.get("error") {
                    Some(Value::String(message)) => reply.fail("ERR_ENGINE", message.clone()),
                    Some(other) => reply.fail("ERR_ENGINE", other.to_string()),
                    None => reply.ok(Some(Value::String(path))),
                },
            );
        }),
    );
    Ok(())
}

fn bookmarks_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("tree", passing!("ac_bookmarks_tree")),
        ("apply", passing!("ac_bookmarks_apply")),
        ("watch", passing!("ac_bookmarks_watch")),
        ("exportHtml", export_html),
    ]
}

fn run_command(ctx: &DataCtx, action: &str, args: &CallArgs, reply: CallReply) -> Done {
    let id = args.text("id")?;
    ctx.owner(&id).download_command(action, &id);
    reply.ok(None);
    Ok(())
}

fn downloads_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("cancel", |ctx, args, reply| {
            run_command(ctx, "cancel", &args, reply)
        }),
        ("pause", |ctx, args, reply| {
            run_command(ctx, "pause", &args, reply)
        }),
        ("resume", |ctx, args, reply| {
            run_command(ctx, "resume", &args, reply)
        }),
        ("keep", |ctx, args, reply| {
            run_command(ctx, "keep", &args, reply)
        }),
        ("settings", passing!("ac_downloads_settings")),
        ("setSettings", passing!("ac_downloads_set_settings")),
    ]
}

fn favicons_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("get", passing!("ac_favicons_get")),
        ("set", passing!("ac_favicons_set")),
        ("fetch", |ctx, args, reply| {
            let url = args.text("url")?;
            ctx.favicons
                .fetch(&url, &args.profile, None, move |icon| reply.ok(icon));
            Ok(())
        }),
    ]
}

fn history_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("query", passing!("ac_history_query")),
        ("add", passing!("ac_history_add")),
        ("import", passing!("ac_history_import")),
        ("deleteUrls", passing!("ac_history_delete_urls")),
        ("watch", passing!("ac_history_watch")),
    ]
}

fn password_toggle(ctx: &DataCtx, args: CallArgs, reply: CallReply) -> Done {
    ctx.services.run(
        "profileToggles",
        &args.profile,
        json!({ "set": null }),
        Box::new(move |result| match result {
            Ok(toggles) => match toggle(&toggles, "passwords") {
                Some(on) => reply.ok(Some(Value::Bool(on))),
                None => reply.fail("ERR_ENGINE", "no password setting"),
            },
            Err(error) => reply.fail("ERR_ENGINE", error),
        }),
    );
    Ok(())
}

/// `AsyncFunction("save")` of `PasswordsModule`.
pub fn save_password(
    ctx: &DataCtx,
    profile: &str,
    origin: &str,
    username: &str,
    password: &str,
    reply: CallReply,
) {
    let payload = json!({ "origin": origin, "username": username, "password": password });
    ctx.service("savePassword", profile, payload, reply);
}

/// `AsyncFunction("update")` of `PasswordsModule`.
pub fn update_password(
    ctx: &DataCtx,
    profile: &str,
    origin: &str,
    username: &str,
    new_username: Option<&str>,
    new_password: Option<&str>,
    reply: CallReply,
) {
    let payload = json!({
        "origin": origin, "username": username, "newUsername": new_username, "newPassword": new_password,
    });
    ctx.service("updatePassword", profile, payload, reply);
}

fn passwords_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("list", service!("listPasswords")),
        ("unlock", service!("unlockPasswords")),
        (
            "reveal",
            service!("password", "origin": text, "username": text),
        ),
        (
            "remove",
            service!("deletePassword", "origin": text, "username": text),
        ),
        ("neverSaveOrigins", service!("neverSaveOrigins")),
        ("allowSaving", service!("allowSaving", "origin": text)),
        ("export", service!("exportPasswords")),
        ("autofillEnabled", password_toggle),
        ("setAutofillEnabled", |ctx, args, reply| {
            let enabled = args.bool("enabled")?;
            set_toggles(
                ctx,
                &args.profile,
                Map::from_iter([("passwords".to_owned(), Value::Bool(enabled))]),
                reply,
            );
            Ok(())
        }),
        ("setFieldProviders", |ctx, args, reply| {
            let payload = json!({
                "generation": args.double("generation")?,
                "session": args.text("session")?,
                "profiles": args.dicts("profiles")?,
            });
            ctx.service("setFieldProviders", &args.profile, payload, reply);
            Ok(())
        }),
        ("showSystemPanel", |ctx, args, reply| {
            let payload = json!({
                "tab": args.text("tab")?,
                "generation": args.optional_double("generation")?.unwrap_or(-1.0),
            });
            ctx.service("showSystemPanel", &args.profile, payload, reply);
            Ok(())
        }),
        (
            "showPanelHint",
            service!("showPanelHint", "tab": text, "label": text, "side": text),
        ),
        ("systemPanelAvailable", service!("systemPanelAvailability")),
        ("otpauthHandler", service!("otpauthHandler")),
        ("setUpVerificationCode", |ctx, args, reply| {
            let payload = json!({
                "tab": args.text("tab")?,
                "token": args.text("token")?,
                "generation": args.optional_double("generation")?.unwrap_or(-1.0),
            });
            ctx.service("setUpVerificationCode", &args.profile, payload, reply);
            Ok(())
        }),
    ]
}

fn permissions_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("resolve", |ctx, args, reply| {
            let id = args.text("id")?;
            let result = match args.choice::<PermissionResult>("result")? {
                PermissionResult::Accept => "accept",
                PermissionResult::Deny => "deny",
                PermissionResult::Dismiss => "dismiss",
            };
            ctx.owner(&id)
                .resolve_permission(&id, result, args.bool("remember")?);
            reply.ok(None);
            Ok(())
        }),
        ("resolveExternalApp", |ctx, args, reply| {
            let id = args.text("id")?;
            ctx.owner(&id)
                .resolve_external_app(&id, args.bool("open")?, args.bool("remember")?);
            reply.ok(None);
            Ok(())
        }),
        ("externalAppAllowances", |ctx, args, reply| {
            page_engines::external_app_allowances(
                ctx,
                &args.profile,
                Box::new(move |all| reply.ok(Some(Value::Array(all)))),
            );
            Ok(())
        }),
        ("removeExternalAppAllowance", |ctx, args, reply| {
            page_engines::remove_external_app_allowance(
                ctx,
                &args.profile,
                &args.text("origin")?,
                &args.text("scheme")?,
            );
            reply.ok(None);
            Ok(())
        }),
        ("devExternalLaunches", service!("testExternalLaunches")),
    ]
}

fn profiles_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("loaded", |ctx, _, reply| {
            reply.ok(Some(
                json!({ "profiles": ctx.registry.loaded_data_profile_names() }),
            ));
            Ok(())
        }),
        ("ctrlTabMru", service!("ctrlTabMru")),
        ("release", |ctx, args, reply| {
            ctx.services.run(
                "releaseProfile",
                &args.profile,
                json!({}),
                Box::new(move |_| reply.ok(None)),
            );
            Ok(())
        }),
        ("unloadIdle", service!("unloadIdleProfile")),
        ("deleteData", |ctx, args, reply| {
            page_engines::delete_profile_data(
                ctx,
                &args.profile,
                Box::new(move |left| reply.ok(Some(json!({ "remaining": left })))),
            );
            Ok(())
        }),
        ("clearBrowsingData", |ctx, args, reply| {
            let types = args.strings("types")?;
            let since = args.optional_double("since")?.unwrap_or(0.0);
            page_engines::clear_browsing_data(
                ctx,
                &args.profile,
                &types,
                since,
                Box::new(move |error| match error {
                    Some(error) => reply.fail("ERR_ENGINE", error),
                    None => reply.ok(None),
                }),
            );
            Ok(())
        }),
    ]
}

fn search_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("suggest", passing!("ac_search_suggest")),
        ("stopSuggest", passing!("ac_search_suggest_stop")),
        ("engines", service!("searchEngineList")),
        ("setEngineName", |ctx, args, reply| {
            let name = args.text("name")?;
            ctx.services.run(
                "setSearchEngineName",
                &args.profile,
                json!({ "name": name }),
                Box::new(move |_| reply.ok(None)),
            );
            Ok(())
        }),
    ]
}

fn engine<'a>(
    ctx: &'a DataCtx,
    args: &CallArgs,
) -> Result<&'a std::rc::Rc<dyn super::host::PageEngine>, BadCallArgs> {
    Ok(match args.choice::<PageEngine>("engine")? {
        PageEngine::Chromium => &ctx.chromium,
        PageEngine::Webkit => &ctx.webkit,
    })
}

fn done_service(reply: CallReply) -> super::host::ServiceDone {
    Box::new(move |result| match result {
        Ok(value) => reply.ok(Some(value)),
        Err(error) => reply.fail("ERR_ENGINE", error),
    })
}

fn sites_table() -> &'static [(&'static str, Handler<DataCtx>)] {
    &[
        ("settings", |ctx, args, reply| {
            engine(ctx, &args)?.site_settings(
                &args.profile,
                &args.text("origin")?,
                done_service(reply),
            );
            Ok(())
        }),
        ("setSetting", |ctx, args, reply| {
            let (origin, kind) = (args.text("origin")?, args.text("type")?);
            engine(ctx, &args)?.set_site_setting(
                &args.profile,
                &origin,
                &kind,
                &args.text("value")?,
            );
            if kind == "notifications" {
                ctx.services.run(
                    "refreshNotifications",
                    &args.profile,
                    json!({ "origin": origin }),
                    Box::new(|_| {}),
                );
            }
            reply.ok(None);
            Ok(())
        }),
        ("origins", service!("siteSettingsOrigins")),
        ("reset", |ctx, args, reply| {
            let origin = args.text("origin")?;
            ctx.services.run(
                "resetSiteSettings",
                &args.profile,
                json!({ "origin": origin }),
                Box::new(|_| {}),
            );
            ctx.services.run(
                "refreshNotifications",
                &args.profile,
                json!({ "origin": origin }),
                Box::new(|_| {}),
            );
            reply.ok(None);
            Ok(())
        }),
        ("clearData", |ctx, args, reply| {
            engine(ctx, &args)?.clear_site_data(
                &args.profile,
                &args.text("origin")?,
                done_service(reply),
            );
            Ok(())
        }),
        ("readStorage", passing!("ac_local_storage_read")),
        ("writeStorage", passing!("ac_local_storage_write")),
        ("registrableDomains", passing!("ac_registrable_domains")),
        ("zoomLevels", service!("zoomLevels")),
        ("setZoom", |ctx, args, reply| {
            let payload = json!({ "zoom": args.double("zoom")?, "host": args.text("host")? });
            ctx.services
                .run("setZoom", &args.profile, payload, Box::new(|_| {}));
            reply.ok(None);
            Ok(())
        }),
        ("preloadPages", service!("preloadPages")),
        ("setPreloadPages", service!("setPreloadPages", "on": bool)),
    ]
}

/// Runs call `name` of `domain` for `profile`. The domain `contentBlocker` goes to `blocker`; the others use the
/// tables below. `reply` settles once, also for an unknown domain.
pub fn run_call(
    ctx: &DataCtx,
    blocker: &ContentBlocker,
    domain: &str,
    name: &str,
    profile: &str,
    args: &str,
    reply: CallReply,
) {
    if domain == "contentBlocker" {
        return CALLS.run(blocker, name, profile, args, reply);
    }
    run_data_call(ctx, domain, name, profile, args, reply)
}

/// `run_call` without the content blocker: the live test has no extension page.
pub fn run_data_call(
    ctx: &DataCtx,
    domain: &str,
    name: &str,
    profile: &str,
    args: &str,
    reply: CallReply,
) {
    match tables().iter().find(|table| table.domain == domain) {
        Some(table) => table.run(ctx, name, profile, args, reply),
        None => reply.fail("ERR_UNKNOWN_CALL", format!("{domain} isn't a domain")),
    }
}

/// The tables of the data domains, in the order of the modules. The content blocker table is in
/// `content_blocker_module`.
pub fn tables() -> Vec<CallTable<DataCtx>> {
    [
        ("autofill", autofill_table()),
        ("bookmarks", bookmarks_table()),
        ("downloads", downloads_table()),
        ("favicons", favicons_table()),
        ("history", history_table()),
        ("passwords", passwords_table()),
        ("permissions", permissions_table()),
        ("profiles", profiles_table()),
        ("search", search_table()),
        ("sites", sites_table()),
    ]
    .into_iter()
    .map(|(domain, handlers)| CallTable { domain, handlers })
    .collect()
}
