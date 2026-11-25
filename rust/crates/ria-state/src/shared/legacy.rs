// Ported from ardia apps/browser/src/lib/legacy.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Moves saved values from the old `netnyahoo` names to the Ardia names: URL schemes and setting keys.
//!
//! Changes from Ardia:
//! - `fromLegacy` takes a JSON value and gives `None` where the original gives back the same object
//!   (nothing changed). JavaScript values that JSON cannot hold (functions, class instances) are out of scope.
//! - `LEGACY_KEYS` is a function. The original looks keys up in an object literal, so a key that names
//!   an `Object.prototype` member (`toString`, `__proto__`) maps to that member as a string key; the port keeps this.

use serde_json::{Map, Value};

/// `LEGACY_BOOKMARK_SEED` (`legacy.ts:1`).
pub const LEGACY_BOOKMARK_SEED: &str = "netnyahoo-bookmark:";

/// `LEGACY_URLS` (`legacy.ts:3`): old prefix and new prefix.
pub const LEGACY_URLS: [(&str, &str); 3] = [
    ("netnyahoo://yahu", "ardia://game"),
    ("chrome://yahu", "chrome://game"),
    ("netnyahoo://", "ardia://"),
];

/// `LEGACY_KEYS` (`legacy.ts:9`): old setting key to new setting key.
pub const LEGACY_KEYS: [(&str, &str); 4] = [
    ("openLinksInSmallYahu", "openLinksInLittleArdia"),
    ("smallYahuSize", "littleArdiaSize"),
    ("newSmallYahu", "newLittleArdia"),
    ("toggleOpenLinksInSmallYahu", "toggleOpenLinksInLittleArdia"),
];

/// `LEGACY_KEYS[key]` as a property key: own entries, then the `Object.prototype` members.
fn legacy_key(key: &str) -> Option<String> {
    if let Some((_, now)) = LEGACY_KEYS.iter().find(|(old, _)| *old == key) {
        return Some(now.to_string());
    }
    const PROTO: [&str; 11] = [
        "constructor:Object",
        "__defineGetter__",
        "__defineSetter__",
        "hasOwnProperty",
        "__lookupGetter__",
        "__lookupSetter__",
        "isPrototypeOf",
        "propertyIsEnumerable",
        "toString",
        "valueOf",
        "toLocaleString",
    ];
    if key == "__proto__" {
        return Some("[object Object]".to_string());
    }
    PROTO.iter().find_map(|entry| {
        let (member, name) = entry.split_once(':').unwrap_or((entry, entry));
        (member == key).then(|| format!("function {name}() {{ [native code] }}"))
    })
}

/// `fromLegacyUrl` (`legacy.ts:16`): the URL with a legacy prefix replaced, also after `view-source:`.
pub fn from_legacy_url(url: &str) -> String {
    // `toLowerCase` keeps the ASCII prefixes at the same index: none of their letters has a
    // non-ASCII capital (`K`, `İ`) that lowercases to ASCII.
    for prefix in ["", "view-source:"] {
        for (old, now) in LEGACY_URLS {
            let len = prefix.len() + old.len();
            if !url
                .get(..len)
                .is_some_and(|head| head.eq_ignore_ascii_case(&format!("{prefix}{old}")))
            {
                continue;
            }
            let rest = &url[len..];
            if !old.ends_with("://") && !rest.is_empty() && !rest.starts_with(['/', '?', '#']) {
                continue;
            }
            return format!("{}{now}{rest}", &url[..prefix.len()]);
        }
    }
    url.to_string()
}

/// `fromLegacy` (`legacy.ts:29`): every string and setting key moved to the new names, or `None`
/// when nothing changed.
pub fn from_legacy(value: &Value) -> Option<Value> {
    match value {
        Value::String(s) => {
            let now = from_legacy_url(s);
            (now != *s).then_some(Value::String(now))
        }
        Value::Array(items) => {
            let out: Vec<Option<Value>> = items.iter().map(from_legacy).collect();
            out.iter().any(Option::is_some).then(|| {
                Value::Array(
                    out.into_iter()
                        .zip(items)
                        .map(|(now, was)| now.unwrap_or_else(|| was.clone()))
                        .collect(),
                )
            })
        }
        Value::Object(source) => {
            let mut changed = false;
            let mut out = Map::new();
            for (key, v) in source {
                let name = legacy_key(key).unwrap_or_else(|| key.clone());
                if name != *key && source.contains_key(&name) {
                    changed = true;
                    continue;
                }
                let now = from_legacy(v);
                changed |= name != *key || now.is_some();
                out.insert(name, now.unwrap_or_else(|| v.clone()));
            }
            changed.then_some(Value::Object(out))
        }
        _ => None,
    }
}
