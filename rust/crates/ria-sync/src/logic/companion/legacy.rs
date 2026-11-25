// Ported from ardia packages/sync/src/companion/legacy.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The addresses of the old app name, read as the addresses of Ardia.
//!
//! Changes from Ardia:
//! - `fromLegacyUrl` compares the prefix with an ASCII case fold, not with `toLowerCase`. The
//!   prefixes are ASCII and have no letter that a non-ASCII letter lowers to (only `K` (Kelvin)
//!   and `İ` lower to ASCII), so the result is the same.
//! - `fromLegacy` takes a JSON value; Ardia also accepts any JS value.

use serde_json::Value;

const LEGACY_URLS: [(&str, &str); 3] = [
    ("netnyahoo://yahu", "ardia://game"),
    ("chrome://yahu", "chrome://game"),
    ("netnyahoo://", "ardia://"),
];

/// Port of `fromLegacyUrl` (`packages/sync/src/companion/legacy.ts:7`).
pub fn from_legacy_url(url: &str) -> String {
    for prefix in ["", "view-source:"] {
        for (old, now) in LEGACY_URLS {
            let n = prefix.len() + old.len();
            let Some(head) = url.as_bytes().get(..n) else {
                continue;
            };
            // A match is all ASCII, so `n` is a char boundary of `url`.
            if !head[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
                || !head[prefix.len()..].eq_ignore_ascii_case(old.as_bytes())
            {
                continue;
            }
            let rest = &url[n..];
            if !old.ends_with("://") && rest.chars().next().is_some_and(|c| !"/?#".contains(c)) {
                continue;
            }
            return format!("{}{now}{rest}", &url[..prefix.len()]);
        }
    }
    url.to_owned()
}

/// Port of `fromLegacy` (`packages/sync/src/companion/legacy.ts:20`).
pub fn from_legacy(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(from_legacy_url(s)),
        Value::Array(items) => Value::Array(items.iter().map(from_legacy).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), from_legacy(v)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_only_whole_legacy_prefixes() {
        assert_eq!(from_legacy_url("NetnYahoo://Yahu?x"), "ardia://game?x");
        assert_eq!(
            from_legacy_url("view-source:chrome://yahu/a"),
            "view-source:chrome://game/a"
        );
        assert_eq!(from_legacy_url("chrome://yahux"), "chrome://yahux");
        assert_eq!(from_legacy_url("netnyahoo://yahux"), "ardia://yahux");
        assert_eq!(from_legacy_url("netnyahoo:/é"), "netnyahoo:/é");
    }
}
