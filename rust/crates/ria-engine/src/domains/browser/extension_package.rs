// Ported from ardia packages/ardiacore/ios/ArdiaExtensionPackage.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Reads the manifest of an unpacked extension folder.

use std::path::{Component, Path, PathBuf};

use serde_json::{Map, Value, json};

use super::text::{base64_decode_lenient, base64_encode, parse_json5_subset, sha256};

/// An extension id: 32 letters from `a` to `p`.
pub fn is_extension_id(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|c| (b'a'..=b'p').contains(&c))
}

/// The id of the key in a manifest `key` field: the first 16 bytes of its SHA-256, as letters from `a` to `p`.
pub fn id_for_key(base64_key: &str) -> Option<String> {
    let key = base64_decode_lenient(base64_key);
    if key.is_empty() {
        return None;
    }
    let hash = sha256(&key);
    Some(
        hash[..16]
            .iter()
            .flat_map(|b| [(b'a' + (b >> 4)) as char, (b'a' + (b & 15)) as char])
            .collect(),
    )
}

fn load_json(path: &Path) -> Option<Map<String, Value>> {
    let text = std::fs::read_to_string(path).ok()?;
    match parse_json5_subset(&text)? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

/// `path` with `.` and `..` resolved by name. `NSString.stringByStandardizingPath` also follows symlinks.
// shortcut: symbolic links are not resolved, so a link inside the folder can point outside it; resolve them when links in extensions matter.
fn standardize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// A `data:` URL for an image file inside `folder`, or `None` for a path outside it, an empty file or an unknown type.
pub fn data_url(folder: &Path, relative_path: &str) -> Option<String> {
    if relative_path.is_empty() {
        return None;
    }
    let rel = relative_path.strip_prefix('/').unwrap_or(relative_path);
    let root = standardize(folder);
    let path = standardize(&folder.join(rel));
    if path.components().count() <= root.components().count() || !path.starts_with(&root) {
        return None;
    }
    let data = std::fs::read(&path).ok().filter(|d| !d.is_empty())?;
    let kind = match path.extension()?.to_str()?.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        _ => return None,
    };
    Some(format!("data:{kind};base64,{}", base64_encode(&data)))
}

type Messages = Map<String, Value>;

/// The text of `value`, with a `__MSG_name__` reference replaced from the messages (names match without case).
fn localize(value: Option<&Value>, messages: &Messages) -> Option<String> {
    let s = value?.as_str()?;
    if !(s.starts_with("__MSG_") && s.ends_with("__") && s.encode_utf16().count() > 8) {
        return Some(s.to_owned());
    }
    let key = s[6..s.len() - 2].to_lowercase();
    messages
        .iter()
        .find(|(k, _)| k.to_lowercase() == key)
        .map_or_else(
            || Some(s.to_owned()),
            |(_, v)| Some(v["message"].as_str()?.to_owned()),
        )
}

fn valid_messages(messages: &Messages) -> bool {
    messages
        .values()
        .all(|entry| entry.get("message").is_some_and(Value::is_string))
}

/// The merged messages of the locales in order: the UI locale, its language, the manifest default, `en`, `en_US`.
fn messages(folder: &Path, manifest: &Messages, ui_locale: &str) -> Result<Messages, String> {
    let mut ui = ui_locale.replace('-', "_");
    if ui == "en" {
        ui = "en_US".into();
    }
    let mut candidates = Vec::new();
    if !ui.is_empty() {
        if let Some((language, _)) = ui.split_once('_') {
            let language = language.to_owned();
            candidates.push(ui);
            candidates.push(language);
        } else {
            candidates.push(ui);
        }
    }
    if let Some(Value::String(default)) = manifest.get("default_locale") {
        candidates.push(default.clone());
    }
    candidates.extend(["en".to_owned(), "en_US".to_owned()]);
    let mut merged = Messages::new();
    let mut loaded: Vec<String> = Vec::new();
    for locale in candidates {
        if loaded.contains(&locale) {
            continue;
        }
        let file = folder.join("_locales").join(&locale).join("messages.json");
        loaded.push(locale.clone());
        let Some(found) = load_json(&file) else {
            continue;
        };
        if !valid_messages(&found) {
            return Err(format!(
                "The localization file _locales/{locale}/messages.json is invalid"
            ));
        }
        for (key, value) in found {
            merged.entry(key).or_insert(value);
        }
    }
    Ok(merged)
}

/// The icon path nearest to `size`: the smallest one that is not smaller, else the largest.
fn icon_path(icons: Option<&Value>, size: i64) -> Option<String> {
    let icons = match icons? {
        Value::String(s) => return Some(s.clone()),
        Value::Object(map) => map,
        _ => return None,
    };
    let mut best: Option<(&str, i64)> = None;
    for (key, value) in icons {
        let (Some(path), s) = (value.as_str(), leading_int(key)) else {
            continue;
        };
        if s <= 0 {
            continue;
        }
        let better = match best {
            None => true,
            Some((_, best_size)) if best_size < size => s > best_size,
            Some((_, best_size)) => s >= size && s < best_size,
        };
        if better {
            best = Some((path, s));
        }
    }
    best.map(|(path, _)| path.to_owned())
}

/// `NSString.intValue`: the integer at the start of the text, else 0.
fn leading_int(s: &str) -> i64 {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && matches!(c, '-' | '+')))
        .count();
    s[..end].parse().unwrap_or(0)
}

fn strings(list: Option<&Value>) -> Vec<String> {
    list.and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn is_host_pattern(s: &str) -> bool {
    s == "<all_urls>" || s.contains("://")
}

fn text_or_null(value: Option<String>) -> Value {
    value.map_or(Value::Null, Value::String)
}

/// The summary of the manifest in `folder`, or `{"error": ...}`. A field with no value is left out.
/// `ui_locale` is the first preferred language of the app (for example `en`, `ja` or `pt-BR`).
pub fn read_manifest(folder: &Path, ui_locale: &str) -> Value {
    let Some(m) = load_json(&folder.join("manifest.json")) else {
        return json!({"error": "Manifest file is missing or unreadable"});
    };
    let messages = match messages(folder, &m, ui_locale) {
        Ok(messages) => messages,
        Err(error) => return json!({"error": error}),
    };
    let action = ["action", "browser_action", "page_action"]
        .iter()
        .find_map(|k| m.get(*k).filter(|v| v.is_object()));
    let mut hosts: Vec<String> = Vec::new();
    let mut permissions = Vec::new();
    for p in strings(m.get("permissions")) {
        if is_host_pattern(&p) {
            hosts.push(p);
        } else {
            permissions.push(p);
        }
    }
    hosts.extend(strings(m.get("host_permissions")));
    for script in m
        .get("content_scripts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        hosts.extend(strings(script.get("matches")));
    }
    let mut seen = Vec::new();
    hosts.retain(|h| {
        let new = !seen.contains(h);
        if new {
            seen.push(h.clone());
        }
        new
    });

    let action_get = |key: &str| action.and_then(|a| a.get(key));
    let options = match m.get("options_ui") {
        Some(Value::Object(ui)) => ui.get("page"),
        _ => m.get("options_page"),
    };
    let incognito = m.get("incognito").and_then(Value::as_str);
    let mut out = Map::new();
    let mut put = |key: &str, value: Value| {
        if !value.is_null() {
            out.insert(key.to_owned(), value);
        }
    };
    put(
        "name",
        json!(localize(m.get("name"), &messages).unwrap_or_default()),
    );
    put(
        "shortName",
        text_or_null(localize(m.get("short_name"), &messages)),
    );
    put(
        "version",
        json!(m.get("version").and_then(Value::as_str).unwrap_or("")),
    );
    put(
        "description",
        json!(localize(m.get("description"), &messages).unwrap_or_default()),
    );
    put(
        "manifestVersion",
        m.get("manifest_version")
            .filter(|v| v.is_number())
            .cloned()
            .unwrap_or(json!(2)),
    );
    put(
        "icon",
        text_or_null(icon_path(m.get("icons"), 128).and_then(|p| data_url(folder, &p))),
    );
    put("permissions", json!(permissions));
    put(
        "optionalPermissions",
        json!(strings(m.get("optional_permissions"))),
    );
    put("hostPermissions", json!(hosts));
    put("hasAction", json!(action.is_some()));
    put(
        "popup",
        action_get("default_popup")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map_or(Value::Null, |s| json!(s)),
    );
    put(
        "actionTitle",
        text_or_null(localize(action_get("default_title"), &messages)),
    );
    let action_icon = icon_path(action_get("default_icon"), 32)
        .or_else(|| icon_path(m.get("icons"), 32))
        .and_then(|p| data_url(folder, &p));
    put("actionIcon", text_or_null(action_icon));
    put(
        "optionsPage",
        options
            .filter(|v| v.is_string())
            .cloned()
            .unwrap_or(Value::Null),
    );
    let panel = m.get("side_panel").and_then(|p| p.get("default_path"));
    put(
        "sidePanel",
        panel
            .filter(|v| v.is_string())
            .cloned()
            .unwrap_or(Value::Null),
    );
    put("incognitoSplit", json!(incognito == Some("split")));
    put("incognitoAllowed", json!(incognito != Some("not_allowed")));
    put(
        "homepageUrl",
        m.get("homepage_url")
            .filter(|v| v.is_string())
            .cloned()
            .unwrap_or(Value::Null),
    );
    put(
        "id",
        text_or_null(m.get("key").and_then(Value::as_str).and_then(id_for_key)),
    );
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ria-ext-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn extension_ids_are_32_letters_a_to_p() {
        assert!(is_extension_id("abcdefghijklmnopabcdefghijklmnop"));
        assert!(!is_extension_id("abcdefghijklmnopabcdefghijklmnoq"));
        assert!(!is_extension_id("abc"));
    }

    #[test]
    fn id_for_key_hashes_the_decoded_key() {
        // SHA-256("abc") starts ba 78 16 bf; each nibble n becomes the letter 'a' + n.
        let id = id_for_key("YWJj").unwrap();
        assert_eq!(id.len(), 32);
        assert!(id.starts_with("lkhibglp"), "{id}");
        assert_eq!(id_for_key(""), None);
    }

    #[test]
    fn manifest_is_localized_and_sorted_into_permissions_and_hosts() {
        let dir = folder("manifest");
        fs::create_dir_all(dir.join("_locales/en")).unwrap();
        fs::write(
            dir.join("_locales/en/messages.json"),
            r#"{"Title":{"message":"Hello"}}"#,
        )
        .unwrap();
        fs::write(dir.join("i.png"), [1u8, 2, 3]).unwrap();
        fs::write(
            dir.join("manifest.json"),
            r#"{
              // comment
              "name": "__MSG_title__", "version": "1.0", "manifest_version": 3,
              "permissions": ["tabs", "https://a.com/*"], "host_permissions": ["https://b.com/*"],
              "content_scripts": [{"matches": ["https://a.com/*"]}],
              "action": {"default_popup": "p.html", "default_icon": "i.png"},
              "icons": {"16": "i.png", "128": "../out.png"},
              "incognito": "split",
            }"#,
        )
        .unwrap();
        let v = read_manifest(&dir, "en-US");
        assert_eq!(v["name"], "Hello");
        assert_eq!(v["permissions"], json!(["tabs"]));
        assert_eq!(
            v["hostPermissions"],
            json!(["https://a.com/*", "https://b.com/*"])
        );
        assert_eq!(v["hasAction"], true);
        assert_eq!(v["popup"], "p.html");
        assert_eq!(v["incognitoSplit"], true);
        assert!(
            v.get("icon").is_none(),
            "icon outside the folder is dropped"
        );
        assert_eq!(v["actionIcon"], "data:image/png;base64,AQID");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_manifest_and_bad_locale_report_an_error() {
        let dir = folder("errors");
        assert_eq!(
            read_manifest(&dir, "en")["error"],
            "Manifest file is missing or unreadable"
        );
        fs::write(dir.join("manifest.json"), "{}").unwrap();
        fs::create_dir_all(dir.join("_locales/en")).unwrap();
        fs::write(dir.join("_locales/en/messages.json"), r#"{"a":1}"#).unwrap();
        assert_eq!(
            read_manifest(&dir, "en")["error"],
            "The localization file _locales/en/messages.json is invalid"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn icon_path_picks_the_nearest_size() {
        let icons = json!({"16": "a", "48": "b", "128": "c"});
        assert_eq!(icon_path(Some(&icons), 32).as_deref(), Some("b"));
        assert_eq!(icon_path(Some(&icons), 200).as_deref(), Some("c"));
        assert_eq!(icon_path(Some(&icons), 128).as_deref(), Some("c"));
    }
}
