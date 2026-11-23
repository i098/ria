// Ported from ardia packages/ardiacore/ios/ArdiaExtensionPackage.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Reads the manifest of an unpacked extension folder.

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{Mode, OFlags, openat};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::text::{base64_decode_lenient, base64_encode};
use crate::call::CallError;

/// An extension id: 32 letters from `a` to `p`.
pub fn is_extension_id(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|c| (b'a'..=b'p').contains(&c))
}

/// The id of the key in a manifest `key` field: the first 16 bytes of its SHA-256, as letters from `a` to `p`.
pub fn id_for_key(base64_key: &str) -> Option<String> {
    let key = base64_decode_lenient(base64_key)?;
    if key.is_empty() {
        return None;
    }
    let hash = Sha256::digest(key);
    Some(
        hash[..16]
            .iter()
            .flat_map(|b| [(b'a' + (b >> 4)) as char, (b'a' + (b & 15)) as char])
            .collect(),
    )
}

fn load_json(package: &Package<'_>, path: &Path) -> Option<Map<String, Value>> {
    let mut bytes = Vec::new();
    package.open_file(path)?.read_to_end(&mut bytes).ok()?;
    match bytes.as_slice() {
        [0, 0, 0xfe, 0xff, rest @ ..] => load_utf32(rest, u32::from_be_bytes),
        [0xff, 0xfe, 0, 0, rest @ ..] => load_utf32(rest, u32::from_le_bytes),
        [0xfe, 0xff, rest @ ..] => load_utf16(rest, u16::from_be_bytes),
        [0xff, 0xfe, rest @ ..] => load_utf16(rest, u16::from_le_bytes),
        [0, 0, 0, _, ..] => load_utf32(&bytes, u32::from_be_bytes),
        [_, 0, 0, 0, ..] => load_utf32(&bytes, u32::from_le_bytes),
        [0, _, ..] => load_utf16(&bytes, u16::from_be_bytes),
        [_, 0, ..] => load_utf16(&bytes, u16::from_le_bytes),
        _ => {
            let text = std::str::from_utf8(&bytes).ok()?;
            parse_json(text.trim_start_matches('\u{feff}'))
        }
    }
}

fn load_utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> Option<Map<String, Value>> {
    let (chunks, rest) = bytes.as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    let text: String = char::decode_utf16(chunks.iter().copied().map(unit))
        .collect::<Result<_, _>>()
        .ok()?;
    parse_json(&text)
}

fn load_utf32(bytes: &[u8], unit: fn([u8; 4]) -> u32) -> Option<Map<String, Value>> {
    let (chunks, rest) = bytes.as_chunks::<4>();
    if !rest.is_empty() {
        return None;
    }
    let text: String = chunks
        .iter()
        .map(|b| char::from_u32(unit(*b)))
        .collect::<Option<_>>()?;
    parse_json(&text)
}

fn parse_json(text: &str) -> Option<Map<String, Value>> {
    let mut chars = text.chars().peekable();
    let mut depth = 0usize;
    while let Some(ch) = chars.next() {
        match ch {
            '"' | '\'' => {
                while let Some(c) = chars.next() {
                    if c == ch {
                        break;
                    }
                    if c == '\\' {
                        chars.next();
                    }
                }
            }
            '/' if chars.next_if_eq(&'/').is_some() => {
                for c in chars.by_ref() {
                    if matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                        break;
                    }
                }
            }
            '/' if chars.next_if_eq(&'*').is_some() => {
                while let Some(c) = chars.next() {
                    if c == '*' && chars.next_if_eq(&'/').is_some() {
                        break;
                    }
                }
            }
            '{' | '[' => {
                depth += 1;
                if depth > 128 {
                    return None;
                }
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    json5::from_str(text).ok()
}

struct Package<'a> {
    path: &'a Path,
    root: File,
}

impl Package<'_> {
    fn open_file(&self, path: &Path) -> Option<File> {
        let mut normalized = PathBuf::new();
        let path = if path.components().any(|part| part == Component::ParentDir) {
            for part in path.components() {
                match part {
                    Component::Normal(name) => normalized.push(name),
                    Component::CurDir => {}
                    Component::ParentDir if normalized.pop() => {}
                    _ => return None,
                }
            }
            normalized.as_path()
        } else {
            path
        };
        if path.is_absolute() {
            return None;
        }
        open_without_links(&self.root, path).or_else(|| {
            let root = self.path.canonicalize().ok()?;
            let target = root.join(path).canonicalize().ok()?;
            open_without_links(&self.root, target.strip_prefix(root).ok()?)
        })
    }
}

fn open_without_links(folder: &File, path: &Path) -> Option<File> {
    let mut components = path
        .components()
        .filter(|part| *part != Component::CurDir)
        .peekable();
    let mut directory = None;
    while let Some(Component::Normal(name)) = components.next() {
        let parent = directory.as_ref().unwrap_or(folder);
        let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        if components.peek().is_none() {
            let fd = openat(parent, name, flags | OFlags::NONBLOCK, Mode::empty()).ok()?;
            let file = File::from(fd);
            return file.metadata().ok()?.is_file().then_some(file);
        }
        let fd = openat(parent, name, flags | OFlags::DIRECTORY, Mode::empty()).ok()?;
        directory = Some(File::from(fd));
    }
    None
}

/// A `data:` URL for an image file inside `folder`, or `None` for a path outside it, an empty file or an unknown type.
fn data_url(package: &Package<'_>, relative_path: &str) -> Option<String> {
    let path = Path::new(relative_path.strip_prefix('/').unwrap_or(relative_path));
    let kind = match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        _ => return None,
    };
    let mut data = Vec::new();
    if package.open_file(path)?.read_to_end(&mut data).ok()? == 0 {
        return None;
    }
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
    messages.get(&key).map_or_else(
        || Some(s.to_owned()),
        |v| Some(v["message"].as_str()?.to_owned()),
    )
}

fn valid_messages(messages: &Messages) -> bool {
    messages
        .values()
        .all(|entry| entry.get("message").is_some_and(Value::is_string))
}

/// The merged messages of the locales in order: the UI locale, its language, the manifest default, `en`, `en_US`.
fn messages(
    package: &Package<'_>,
    manifest: &Messages,
    ui_locale: &str,
) -> Result<Messages, CallError> {
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
        if locale.is_empty()
            || !locale
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
            || loaded.contains(&locale)
        {
            continue;
        }
        let file = Path::new("_locales").join(&locale).join("messages.json");
        loaded.push(locale.clone());
        let Some(found) = load_json(package, &file) else {
            continue;
        };
        if !valid_messages(&found) {
            return Err(CallError::Engine(format!(
                "The localization file _locales/{locale}/messages.json is invalid"
            )));
        }
        for (key, value) in found {
            merged.entry(key.to_lowercase()).or_insert(value);
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

fn strings(list: Option<&Value>) -> impl Iterator<Item = &str> {
    list.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

fn is_host_pattern(s: &str) -> bool {
    s == "<all_urls>" || s.contains("://")
}

fn text_or_null(value: Option<String>) -> Value {
    value.map_or(Value::Null, Value::String)
}

/// The summary of the manifest in `folder`. A field with no value is left out.
/// `ui_locale` is the first preferred language of the app (for example `en`, `ja` or `pt-BR`).
pub fn read_manifest(folder: &Path, ui_locale: &str) -> Result<Value, CallError> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let missing = || CallError::Engine("Manifest file is missing or unreadable".into());
    let package = Package {
        path: folder,
        root: File::from(rustix::fs::open(folder, flags, Mode::empty()).map_err(|_| missing())?),
    };
    let m = load_json(&package, Path::new("manifest.json")).ok_or_else(missing)?;
    let messages = messages(&package, &m, ui_locale)?;
    let action = ["action", "browser_action", "page_action"]
        .iter()
        .find_map(|k| m.get(*k).filter(|v| v.is_object()));
    let mut hosts = Vec::new();
    let mut seen = HashSet::new();
    let mut add_host = |host| {
        if seen.insert(host) {
            hosts.push(host);
        }
    };
    let mut permissions = Vec::new();
    for p in strings(m.get("permissions")) {
        if is_host_pattern(p) {
            add_host(p);
        } else {
            permissions.push(p);
        }
    }
    strings(m.get("host_permissions")).for_each(&mut add_host);
    for script in m
        .get("content_scripts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        strings(script.get("matches")).for_each(&mut add_host);
    }

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
        text_or_null(icon_path(m.get("icons"), 128).and_then(|p| data_url(&package, &p))),
    );
    put("permissions", json!(permissions));
    put(
        "optionalPermissions",
        json!(strings(m.get("optional_permissions")).collect::<Vec<_>>()),
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
        .and_then(|p| data_url(&package, &p));
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
    Ok(Value::Object(out))
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
    fn package_bounds_json_nesting_but_ignores_brackets_in_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        let brackets = "[".repeat(128);
        let name = format!("Escaped '\u{2028}{brackets}");
        fs::write(&path, format!("/* {brackets} */ {{name: \"{name}\"}}")).unwrap();
        assert_eq!(read_manifest(dir.path(), "en").unwrap()["name"], name);
        fs::write(
            path,
            format!("{{name: 'test', extra: {brackets}0{}}}", "]".repeat(128)),
        )
        .unwrap();
        assert_eq!(
            read_manifest(dir.path(), "en"),
            Err(CallError::Engine(
                "Manifest file is missing or unreadable".into()
            ))
        );
    }

    #[test]
    fn extension_ids_are_32_letters_a_to_p() {
        assert!(is_extension_id("abcdefghijklmnopabcdefghijklmnop"));
        assert!(!is_extension_id("abcdefghijklmnopabcdefghijklmnoq"));
        assert!(!is_extension_id("abc"));
    }

    #[test]
    fn id_for_key_requires_valid_padding_and_uses_the_sha256_prefix() {
        assert_eq!(
            id_for_key("YWJj").as_deref(),
            Some("lkhibglpipabmpokebebeanofnkocccd")
        );
        assert_eq!(
            id_for_key("Zg==").as_deref(),
            Some("cfcpbamidgbaolmkbkafjmalkoicffol")
        );
        assert_eq!(id_for_key("Zg"), None);
        assert_eq!(id_for_key(""), None);
    }

    #[test]
    fn manifest_and_locale_use_foundation_json_encodings() {
        let dir = tempfile::tempdir().unwrap();
        let locale = dir.path().join("_locales/fr");
        fs::create_dir_all(&locale).unwrap();
        for (width, big_endian, bom) in [
            (2, true, &[0xfe, 0xff][..]),
            (2, false, &[0xff, 0xfe][..]),
            (4, true, &[0, 0, 0xfe, 0xff][..]),
            (4, false, &[0xff, 0xfe, 0, 0][..]),
        ] {
            for prefix in [bom, &[]] {
                let encode = |text: &str| {
                    let mut bytes = prefix.to_vec();
                    if width == 2 {
                        for unit in text.encode_utf16() {
                            bytes.extend(if big_endian {
                                unit.to_be_bytes()
                            } else {
                                unit.to_le_bytes()
                            });
                        }
                    } else {
                        for unit in text.chars().map(u32::from) {
                            bytes.extend(if big_endian {
                                unit.to_be_bytes()
                            } else {
                                unit.to_le_bytes()
                            });
                        }
                    }
                    bytes
                };
                fs::write(
                    dir.path().join("manifest.json"),
                    encode(
                        "{name: '__MSG_title__', description: 'Line\u{2028}Paragraph\u{2029}End'}",
                    ),
                )
                .unwrap();
                fs::write(
                    locale.join("messages.json"),
                    encode("{title: {message: 'Bon\u{2028}été\u{2029}🐝'}}"),
                )
                .unwrap();
                let result = read_manifest(dir.path(), "fr").unwrap();
                assert_eq!(result["name"], "Bon\u{2028}été\u{2029}🐝");
                assert_eq!(result["description"], "Line\u{2028}Paragraph\u{2029}End");
            }
        }
    }

    #[test]
    fn manifest_is_localized_and_sorted_into_permissions_and_hosts() {
        let dir = folder("manifest");
        fs::create_dir_all(dir.join("_locales/en")).unwrap();
        fs::create_dir_all(dir.join("_locales/en_US")).unwrap();
        fs::write(
            dir.join("_locales/en_US/messages.json"),
            "{ title: { message: 'Hello', /* before close */ }, }",
        )
        .unwrap();
        fs::write(
            dir.join("_locales/en/messages.json"),
            r#"{"TITLE":{"message":"Fallback"}}"#,
        )
        .unwrap();
        fs::write(dir.join("i.png"), [1u8, 2, 3]).unwrap();
        fs::write(
            dir.join("manifest.json"),
            r#"{
              // comment
              name: '__MSG_title__', version: '1.0', manifest_version: 3,
              "permissions": ["tabs", "https://z.com/*"],
              "host_permissions": ["https://b.com/*", "https://z.com/*"],
              "content_scripts": [{"matches": ["https://b.com/*", "https://a.com/*"]}],
              "action": {"default_popup": "p.html", "default_icon": "./i.png"},
              "icons": {"16": "i.png", "128": "../out.png"},
              "incognito": "split",
            }"#,
        )
        .unwrap();
        let v = read_manifest(&dir, "en-US").unwrap();
        assert_eq!(v["name"], "Hello");
        assert_eq!(v["permissions"], json!(["tabs"]));
        assert_eq!(
            v["hostPermissions"],
            json!(["https://z.com/*", "https://b.com/*", "https://a.com/*"])
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
            read_manifest(&dir, "en"),
            Err(CallError::Engine(
                "Manifest file is missing or unreadable".into()
            ))
        );
        fs::write(dir.join("manifest.json"), "{}").unwrap();
        fs::create_dir_all(dir.join("_locales/en")).unwrap();
        fs::write(dir.join("_locales/en/messages.json"), r#"{"a":1}"#).unwrap();
        assert_eq!(
            read_manifest(&dir, "en"),
            Err(CallError::Engine(
                "The localization file _locales/en/messages.json is invalid".into()
            ))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn package_reads_follow_links_within_the_folder() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("build")).unwrap();
        fs::create_dir(root.join("_locales")).unwrap();
        fs::write(
            root.join("build/manifest.json"),
            "{name: '__MSG_title__', icons: {'128': 'icon.png'}, action: {default_icon: 'icon.png'}}",
        )
        .unwrap();
        fs::write(root.join("build/icon.png"), [1, 2, 3]).unwrap();
        fs::write(
            root.join("build/messages.json"),
            "{title: {message: 'One\u{2028}Two\u{2029}Three'}}",
        )
        .unwrap();
        symlink("build/manifest.json", root.join("manifest.json")).unwrap();
        symlink("../build", root.join("_locales/en")).unwrap();
        symlink(root.join("build/icon.png"), root.join("icon.png")).unwrap();

        let result = read_manifest(root, "en").unwrap();
        assert_eq!(result["name"], "One\u{2028}Two\u{2029}Three");
        assert_eq!(result["icon"], "data:image/png;base64,AQID");
        assert_eq!(result["actionIcon"], "data:image/png;base64,AQID");
    }

    #[test]
    fn package_reads_do_not_follow_links_outside_the_folder() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        fs::create_dir(&package).unwrap();
        let manifest = package.join("manifest.json");
        fs::write(dir.path().join("outside.json"), "{}").unwrap();
        symlink(dir.path().join("outside.json"), &manifest).unwrap();
        assert!(read_manifest(&package, "en").is_err());
        fs::remove_file(&manifest).unwrap();

        fs::write(dir.path().join("icon.png"), [1, 2, 3]).unwrap();
        fs::write(
            dir.path().join("messages.json"),
            "{title: {message: 'outside'}}",
        )
        .unwrap();
        symlink(dir.path().join("icon.png"), package.join("icon.png")).unwrap();
        fs::create_dir(package.join("_locales")).unwrap();
        symlink(dir.path(), package.join("_locales/en")).unwrap();
        fs::write(
            manifest,
            "{name: '__MSG_title__', icons: {'32': 'icon.png'}, action: {default_icon: 'icon.png'}}",
        )
        .unwrap();
        let result = read_manifest(&package, "en").unwrap();
        assert_eq!(result["name"], "__MSG_title__");
        assert!(result.get("icon").is_none());
        assert!(result.get("actionIcon").is_none());
    }

    #[test]
    fn icon_path_picks_the_nearest_size() {
        let icons = json!({"16": "a", "48": "b", "128": "c"});
        assert_eq!(icon_path(Some(&icons), 32).as_deref(), Some("b"));
        assert_eq!(icon_path(Some(&icons), 200).as_deref(), Some("c"));
        assert_eq!(icon_path(Some(&icons), 128).as_deref(), Some("c"));
    }
}
