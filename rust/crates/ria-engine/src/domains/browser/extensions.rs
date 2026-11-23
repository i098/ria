// Ported from ardia packages/ardiacore/ios/ExtensionsModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `extensions` domain. The shapes of the calls that `ArdiaServices.mm` makes (`listExtensions`, `installExtension`,
//! `setExtension`, `configureExtension` and the others) are here too.

use std::cell::LazyCell;
use std::path::{Component, Path, PathBuf};

use serde_json::{Map, Value, json};

use super::extension_package::{is_extension_id, read_manifest};
use super::{CallContext, EngineRoute, Route, dir_of, passing, string_arg};
use crate::api::extensions::InspectUnpackedArgs;
use crate::call::CallError;

pub fn inspect_unpacked(args: &InspectUnpackedArgs, ui_locale: &str) -> Result<Value, CallError> {
    let mut manifest = read_manifest(Path::new(&args.path), ui_locale)?;
    manifest["path"] = json!(args.path);
    Ok(manifest)
}

/// Calls that the host answers: the file panel, the install prompt and the action state of the tab.
pub const HOST_CALLS: &[&str] = &[
    "inspectUnpacked",
    "chooseFolder",
    "resolveInstallPrompt",
    "actionStates",
    "sidePanelUrl",
];

pub fn route(call: &str) -> Option<Route> {
    let engine = |symbol, shape, reply, changed| {
        Some(Route::Engine(EngineRoute {
            symbol,
            base_profile: true,
            shape,
            reply,
            changed,
        }))
    };
    match call {
        "list" => engine(
            "ac_extensions_list",
            |_| Ok(json!({})),
            list_reply,
            |_, _| None,
        ),
        "install" => engine(
            "ac_extensions_install",
            path_args,
            |r, _| Ok(r),
            |_, reply| installed(reply),
        ),
        "installCrx" => engine(
            "ac_extensions_install_crx",
            path_args,
            |r, _| Ok(r),
            |_, reply| installed(reply),
        ),
        "setEnabled" => engine(
            "ac_extensions_set_enabled",
            set_enabled_args,
            |r, _| Ok(r),
            |args, _| {
                let event = if args["enabled"] == true {
                    "enabled"
                } else {
                    "disabled"
                };
                Some((string_arg(args, "id").ok()?, event))
            },
        ),
        "uninstall" => engine(
            "ac_extensions_uninstall",
            id_args,
            |r, _| Ok(r),
            |args, _| Some((string_arg(args, "id").ok()?, "uninstalled")),
        ),
        "reload" => engine(
            "ac_extensions_reload",
            id_args,
            |r, _| Ok(r),
            |args, _| Some((string_arg(args, "id").ok()?, "reloaded")),
        ),
        "configure" => engine(
            "ac_extensions_configure",
            configure_args,
            |r, _| Ok(r),
            |args, _| Some((string_arg(args, "id").ok()?, "configured")),
        ),
        "watchActions" => Some(passing("ac_extensions_watch_actions")),
        "syncState" => Some(passing("ac_extensions_sync_state")),
        "syncApply" => Some(passing("ac_extensions_sync_apply")),
        "loadHealth" => Some(passing("ac_extensions_load_health")),
        c if HOST_CALLS.contains(&c) => Some(Route::Host),
        _ => None,
    }
}

fn installed(reply: &Value) -> Option<(String, &'static str)> {
    Some((reply["id"].as_str()?.to_owned(), "installed"))
}

fn path_args(args: &Value) -> Result<Value, CallError> {
    Ok(json!({"path": string_arg(args, "path")?}))
}

fn id_args(args: &Value) -> Result<Value, CallError> {
    Ok(json!({"id": string_arg(args, "id")?}))
}

fn set_enabled_args(args: &Value) -> Result<Value, CallError> {
    let enabled = args["enabled"].as_bool().ok_or(CallError::BadArguments)?;
    Ok(json!({"id": string_arg(args, "id")?, "enabled": enabled}))
}

/// `id` and the options that the engine knows; `pinnedIndex` as an integer.
fn configure_args(args: &Value) -> Result<Value, CallError> {
    let options = args["options"].as_object().ok_or(CallError::BadArguments)?;
    let mut out = Map::new();
    out.insert("id".into(), json!(string_arg(args, "id")?));
    for key in ["pinned", "incognito", "fileAccess", "userScripts", "siteAccess"] {
        if let Some(value) = options.get(key).filter(|v| !v.is_null()) {
            let valid = if key == "siteAccess" {
                matches!(
                    value.as_str(),
                    Some("onClick" | "specificSites" | "allSites")
                )
            } else {
                value.is_boolean()
            };
            if !valid {
                return Err(CallError::BadArguments);
            }
            out.insert(key.into(), value.clone());
        }
    }
    if let Some(value) = options.get("pinnedIndex").filter(|v| !v.is_null()) {
        let index = value.as_f64().ok_or(CallError::BadArguments)?.trunc();
        if !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&index) {
            return Err(CallError::BadArguments);
        }
        out.insert("pinnedIndex".into(), json!(index as i32));
    }
    Ok(Value::Object(out))
}

fn store_registrations(profile_dir: &Path) -> Option<Map<String, Value>> {
    ["Secure Preferences", "Preferences"]
        .into_iter()
        .find_map(|name| {
            let reader = std::io::BufReader::new(std::fs::File::open(profile_dir.join(name)).ok()?);
            let mut prefs: Value = serde_json::from_reader(reader).ok()?;
            match prefs.get_mut("extensions")?.get_mut("settings")?.take() {
                Value::Object(settings) => Some(settings),
                _ => None,
            }
        })
}

fn store_folder(profile_dir: &Path, registration: &Value, id: &str, version: &str) -> Option<PathBuf> {
    if !is_extension_id(id) || version.is_empty() {
        return None;
    }
    let path = Path::new(registration.get("path")?.as_str()?);
    let extensions = profile_dir.join("Extensions");
    let relative = if path.is_absolute() {
        path.strip_prefix(&extensions).ok()?
    } else {
        path
    };
    let mut parts = relative.components();
    let (Some(Component::Normal(owner)), Some(Component::Normal(folder)), None) =
        (parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let (registered_version, suffix) = folder.to_str()?.rsplit_once('_')?;
    if owner != id
        || registered_version != version
        || suffix.is_empty()
        || !suffix.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some(extensions.join(relative))
}

/// Adds the manifest fields to each extension of the engine list.
fn list_reply(reply: Value, ctx: &CallContext) -> Result<Value, CallError> {
    let infos = reply["extensions"]
        .as_array()
        .ok_or_else(|| CallError::Engine("Unexpected extension list".into()))?;
    let registrations = LazyCell::new(|| store_registrations(dir_of(ctx)));
    let list: Vec<Value> = infos
        .iter()
        .filter_map(Value::as_object)
        .map(|info| list_item(info, ctx, || registrations.as_ref()))
        .collect();
    Ok(json!({"extensions": list}))
}

fn list_item<'a>(
    info: &Map<String, Value>,
    ctx: &CallContext,
    registrations: impl FnOnce() -> Option<&'a Map<String, Value>>,
) -> Value {
    let mut item: Map<String, Value> = info
        .iter()
        .filter(|(_, v)| !v.is_null())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let path = info.get("path").and_then(Value::as_str);
    let folder = path.map(PathBuf::from).or_else(|| {
        let id = info.get("id")?.as_str()?;
        let version = info.get("version")?.as_str()?;
        store_folder(dir_of(ctx), registrations()?.get(id)?, id, version)
    });
    let manifest = folder
        .and_then(|f| read_manifest(&f, &ctx.ui_locale).ok())
        .unwrap_or(Value::Null);
    for key in [
        "popup",
        "actionTitle",
        "actionIcon",
        "sidePanel",
        "hasAction",
    ] {
        if let Some(value) = manifest.get(key) {
            item.insert(key.into(), value.clone());
        }
    }
    item.insert(
        "incognitoSplit".into(),
        json!(manifest["incognitoSplit"] == true),
    );
    item.insert(
        "incognitoAllowed".into(),
        manifest
            .get("incognitoAllowed")
            .cloned()
            .unwrap_or(Value::Null),
    );
    for key in ["siteAccess", "optionsUrl", "path", "homepageUrl"] {
        item.entry(key).or_insert(Value::Null);
    }
    let from_store = info.get("location").and_then(Value::as_str) == Some("FROM_STORE");
    item.insert("fromWebStore".into(), json!(from_store));
    let url = from_store
        .then(|| info.get("id").and_then(Value::as_str))
        .flatten()
        .map(|id| format!("https://chromewebstore.google.com/detail/{id}"));
    item.insert("webStoreUrl".into(), json!(url));
    Value::Object(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dir: &Path) -> CallContext {
        CallContext {
            profile_name: "p".into(),
            profile_dir: dir.to_string_lossy().into(),
            private: false,
            ui_locale: "en".into(),
        }
    }

    #[test]
    fn configure_keeps_known_options_and_truncates_the_pin_index() {
        let args = json!({"id": "x", "options": {"pinned": true, "pinnedIndex": 2.9, "other": 1, "siteAccess": null}});
        assert_eq!(
            configure_args(&args).unwrap(),
            json!({"id": "x", "pinned": true, "pinnedIndex": 2})
        );
        assert_eq!(
            configure_args(&json!({"id": "x"})),
            Err(CallError::BadArguments)
        );
        for (key, value) in [
            ("pinned", json!(0)),
            ("incognito", json!("true")),
            ("fileAccess", json!("false")),
            ("userScripts", json!([])),
            ("siteAccess", json!(false)),
            ("siteAccess", json!("unknown")),
            ("siteAccess", json!("ON_CLICK")),
            ("pinnedIndex", json!("1")),
            ("pinnedIndex", json!(2147483648_i64)),
            ("pinnedIndex", json!(-2147483649_i64)),
        ] {
            let mut args = json!({"id": "x", "options": {}});
            args["options"][key] = value;
            assert_eq!(configure_args(&args), Err(CallError::BadArguments), "{args}");
        }
    }

    #[test]
    fn configure_forwards_the_site_access_commands() {
        use crate::api::{Call, extensions::Configure};

        let Some(Route::Engine(route)) = super::super::route(Configure::NAME, false) else {
            panic!("extensions.configure has no engine route");
        };
        for mode in ["onClick", "specificSites", "allSites"] {
            let args = json!({"id": "ext", "options": {"siteAccess": mode}});
            assert_eq!(
                (route.shape)(&args),
                Ok(json!({"id": "ext", "siteAccess": mode}))
            );
        }
    }

    #[test]
    fn inspection_returns_a_typed_package_or_a_call_error() {
        use crate::api::{Call, extensions::InspectUnpacked};

        let dir = tempfile::tempdir().unwrap();
        let args = InspectUnpackedArgs {
            path: dir.path().to_str().unwrap().to_owned(),
        };
        assert!(matches!(
            inspect_unpacked(&args, "en"),
            Err(CallError::Engine(_))
        ));
        std::fs::write(dir.path().join("manifest.json"), "{name: 'Example'}").unwrap();
        let value = inspect_unpacked(&args, "en").unwrap();
        let package: <InspectUnpacked as Call>::Reply = serde_json::from_value(value).unwrap();
        assert_eq!(package.path, args.path);
        assert_eq!(package.name, "Example");
    }

    #[test]
    fn list_reply_merges_the_manifest_of_a_path_extension() {
        let dir = std::env::temp_dir().join(format!("ria-extlist-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            r#"{"name":"n","version":"1","action":{"default_popup":"p.html"},"incognito":"split"}"#,
        )
        .unwrap();
        let reply = json!({"extensions": [{"id": "a", "path": dir, "location": "FROM_STORE", "siteAccess": null}]});
        let out = list_reply(reply, &ctx(&dir)).unwrap();
        let item = &out["extensions"][0];
        assert_eq!(item["popup"], "p.html");
        assert_eq!(item["hasAction"], true);
        assert_eq!(item["incognitoSplit"], true);
        assert_eq!(item["fromWebStore"], true);
        assert_eq!(
            item["webStoreUrl"],
            "https://chromewebstore.google.com/detail/a"
        );
        assert_eq!(item["siteAccess"], Value::Null);
        assert_eq!(item["homepageUrl"], Value::Null);
        assert_eq!(
            list_reply(json!({}), &ctx(&dir)),
            Err(CallError::Engine("Unexpected extension list".into()))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_uses_the_registered_store_directory() {
        let dir = tempfile::tempdir().unwrap();
        let id = "abcdefghijklmnopabcdefghijklmnop";
        let reply = json!({"extensions": [{"id": id, "version": "1.2", "location": "FROM_STORE"}]});
        for suffix in ["0", "1", "2"] {
            let folder = dir
                .path()
                .join("Extensions")
                .join(id)
                .join(format!("1.2_{suffix}"));
            std::fs::create_dir_all(&folder).unwrap();
            let manifest = json!({"action": {"default_popup": format!("{suffix}.html")}});
            std::fs::write(folder.join("manifest.json"), manifest.to_string()).unwrap();
        }
        let prefs = dir.path().join("Secure Preferences");
        let settings = json!({"extensions": {"settings": {(id): {"path": format!("{id}/1.2_1")}}}});
        std::fs::write(&prefs, settings.to_string()).unwrap();
        let ctx = ctx(dir.path());
        let out = list_reply(reply.clone(), &ctx).unwrap();
        assert_eq!(out["extensions"][0]["popup"], "1.html");

        std::fs::remove_file(prefs).unwrap();
        let out = list_reply(reply, &ctx).unwrap();
        assert!(out["extensions"][0].get("popup").is_none());
    }
}
