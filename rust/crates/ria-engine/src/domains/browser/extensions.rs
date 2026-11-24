// Ported from ardia packages/ardiacore/ios/ExtensionsModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `extensions` domain. The shapes of the calls that `ArdiaServices.mm` makes (`listExtensions`, `installExtension`,
//! `setExtension`, `configureExtension` and the others) are here too.
//!
//! # Changes from Ardia
//!
//! - `store_folder` takes the folder with the highest numeric suffix (`1.2_10` is newer than `1.2_2`) and ignores
//!   non-numeric suffixes. Ardia takes the first entry of `contentsOfDirectory`, whose order is not defined.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::extension_package::{is_extension_id, read_list_manifest, read_manifest};
use super::{CallContext, EngineRoute, Route, passing, string_arg};
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
    for key in [
        "pinned",
        "incognito",
        "fileAccess",
        "userScripts",
        "siteAccess",
    ] {
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

/// Port of `StoreFolder` (`ArdiaServices.mm` line 114): the newest folder `<profile>/Extensions/<id>/<version>_<N>`.
fn store_folder(profile_dir: &str, id: Option<&str>, version: Option<&str>) -> Option<PathBuf> {
    let (id, version) = (
        id.filter(|id| is_extension_id(id))?,
        version.filter(|v| !v.is_empty())?,
    );
    let dir = Path::new(profile_dir).join("Extensions").join(id);
    let prefix = format!("{version}_");
    std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|n| {
            let suffix = n.strip_prefix(&prefix)?.parse::<u64>().ok()?;
            Some((suffix, n))
        })
        .max()
        .map(|(_, n)| dir.join(n))
}

/// Reads manifest fields only from a path in the engine reply, or else from the newest store install of the listed
/// version. A path from profile preferences can refer to a different installation of the same extension version.
/// Port of `listExtensions:profile:reply:` (`ArdiaServices.mm` lines 280-300).
fn list_reply(reply: Value, ctx: &CallContext) -> Result<Value, CallError> {
    let infos = reply["extensions"]
        .as_array()
        .ok_or_else(|| CallError::Engine("Unexpected extension list".into()))?;
    let list: Vec<Value> = infos
        .iter()
        .filter_map(Value::as_object)
        .map(|info| list_item(info, ctx))
        .collect();
    Ok(json!({"extensions": list}))
}

fn list_item(info: &Map<String, Value>, ctx: &CallContext) -> Value {
    let mut item: Map<String, Value> = info
        .iter()
        .filter(|(_, v)| !v.is_null())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let str_of = |key| info.get(key).and_then(Value::as_str);
    let folder = str_of("path")
        .map(PathBuf::from)
        .or_else(|| store_folder(&ctx.profile_dir, str_of("id"), str_of("version")));
    let manifest = folder
        .and_then(|folder| read_list_manifest(&folder, &ctx.ui_locale).ok())
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
        manifest["incognitoAllowed"].clone(),
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
            assert_eq!(
                configure_args(&args),
                Err(CallError::BadArguments),
                "{args}"
            );
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
            r#"{"name":"n","version":"1","icons":{"32":"icon.png"},"action":{"default_popup":"p.html"},"incognito":"split"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("icon.png"), [1, 2, 3]).unwrap();
        let reply = json!({"extensions": [{
            "id": "a", "path": dir, "location": "FROM_STORE", "siteAccess": null,
            "icon": "data:image/png;base64,BAUG"
        }]});
        let out = list_reply(reply, &ctx(&dir)).unwrap();
        let item = &out["extensions"][0];
        assert_eq!(item["popup"], "p.html");
        assert_eq!(item["icon"], "data:image/png;base64,BAUG");
        assert_eq!(item["actionIcon"], "data:image/png;base64,AQID");
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
    fn pathless_list_reads_the_store_folder_of_the_version() {
        let dir = tempfile::tempdir().unwrap();
        let id = "abcdefghijklmnopabcdefghijklmnop";
        let store = |sub: &str, popup: &str, incognito: &str| {
            let d = dir.path().join("Extensions").join(id).join(sub);
            std::fs::create_dir_all(&d).unwrap();
            let m = format!(
                r#"{{"action":{{"default_popup":"{popup}","default_title":"T"}},"incognito":"{incognito}",
                "side_panel":{{"default_path":"panel.html"}},"icons":{{"32":"i.png"}}}}"#
            );
            std::fs::write(d.join("manifest.json"), m).unwrap();
            std::fs::write(d.join("i.png"), [1, 2, 3]).unwrap();
        };
        store("1.2_0", "a.html", "split");
        store("1.20_0", "wrong.html", "not_allowed");
        let live = json!({"id": id, "version": "1.2", "location": "FROM_STORE", "path": null,
            "popup": "live.html", "incognitoAllowed": false});
        let out = list_reply(json!({"extensions": [live]}), &ctx(dir.path())).unwrap();
        let item = &out["extensions"][0];
        assert_eq!(item["popup"], "a.html");
        assert_eq!(item["actionTitle"], "T");
        assert_eq!(item["actionIcon"], "data:image/png;base64,AQID");
        assert_eq!(item["sidePanel"], "panel.html");
        assert_eq!(item["hasAction"], true);
        assert_eq!(item["incognitoSplit"], true);
        assert_eq!(item["incognitoAllowed"], true);
        // No folder for the version: the live fields stay, the incognito fields follow Ardia.
        let live = json!({"id": id, "version": "9", "path": null, "popup": "live.html"});
        let out = list_reply(json!({"extensions": [live]}), &ctx(dir.path())).unwrap();
        assert_eq!(out["extensions"][0]["popup"], "live.html");
        assert_eq!(out["extensions"][0]["incognitoSplit"], false);
        assert_eq!(out["extensions"][0]["incognitoAllowed"], Value::Null);
    }

    #[test]
    fn pathless_list_preserves_live_metadata_after_same_version_reinstall() {
        let dir = tempfile::tempdir().unwrap();
        let id = "abcdefghijklmnopabcdefghijklmnop";
        let store = |sub: &str, popup: &str| {
            let d = dir.path().join("Extensions").join(id).join(sub);
            std::fs::create_dir_all(&d).unwrap();
            let m = format!(r#"{{"action":{{"default_popup":"{popup}"}}}}"#);
            std::fs::write(d.join("manifest.json"), m).unwrap();
        };
        let popup_of = || {
            let live = json!({"id": id, "version": "1.2", "path": null, "popup": "live.html"});
            let out = list_reply(json!({"extensions": [live]}), &ctx(dir.path())).unwrap();
            out["extensions"][0]["popup"].clone()
        };
        store("1.2_0", "old.html");
        store("1.2_1", "new.html");
        assert_eq!(popup_of(), "new.html");
        store("1.2_2", "two.html");
        store("1.2_10", "ten.html");
        store("1.2_x", "junk.html");
        assert_eq!(popup_of(), "ten.html");
    }
}
