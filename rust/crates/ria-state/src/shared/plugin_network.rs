// Ported from ardia apps/browser/src/lib/pluginNetwork.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Host patterns a plugin may reach (`api.example.com`, `*.example.com`, `*`) and their checks.
//!
//! Changes from Ardia:
//! - `unknown` is `&Json` (`ria_core::identity::telemetry_wire::Json`); `undefined` is not text, as `null` is not.
//! - `LABEL` uses lookarounds, which the `regex` crate lacks: `is_label` checks the same rule in code.
//! - `\d` is ASCII in JS: the patterns use `[0-9]`. `host.length` counts UTF-16 code units.

use regex::Regex;
use ria_core::identity::telemetry_wire::Json;
use std::sync::LazyLock;

use super::js::{is_js_space, js_trim};

/// `ANY_HOST` (`pluginNetwork.ts:1`).
pub const ANY_HOST: &str = "*";
/// `MAX_HOST_LENGTH` (`pluginNetwork.ts:2`).
pub const MAX_HOST_LENGTH: usize = 253;

static IPV4: LazyLock<Regex> = LazyLock::new(|| {
    let byte = "(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])";
    Regex::new(&format!(r"^{byte}(\.{byte}){{3}}$")).expect("static pattern")
});

/// `LABEL` (`pluginNetwork.ts:4`): 1 to 63 of `[a-z0-9-]`, not starting or ending with `-`.
fn is_label(label: &str) -> bool {
    (1..=63).contains(&label.len())
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `NUMERIC_LABELS` (`pluginNetwork.ts:6`).
fn is_numeric_labels(host: &str) -> bool {
    !host.is_empty() && host.bytes().all(|b| b.is_ascii_digit() || b == b'.')
}

/// `normalizedHost` (`pluginNetwork.ts:8`): trimmed, lowercase, without `[]` and a final dot.
pub fn normalized_host(host: &str) -> String {
    let lower = js_trim(host).to_lowercase();
    let bare = match lower.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
        Some(inner) => inner,
        None => &lower,
    };
    bare.strip_suffix('.').unwrap_or(bare).to_owned()
}

/// `isHostName` (`pluginNetwork.ts:14`).
fn is_host_name(host: &str) -> bool {
    if host.is_empty() || host.encode_utf16().count() > MAX_HOST_LENGTH {
        return false;
    }
    if IPV4.is_match(host) {
        return true;
    }
    if is_numeric_labels(host) {
        return false;
    }
    host.split('.').all(is_label)
}

/// The problem of a host pattern that is text.
fn text_problem(pattern: &str) -> Option<&'static str> {
    if pattern == ANY_HOST {
        return None;
    }
    if pattern != pattern.to_lowercase() {
        return Some("must be lowercase");
    }
    if pattern
        .chars()
        .any(|ch| is_js_space(ch) || "/:@?#\\[]".contains(ch))
    {
        return Some(
            "must be a bare host, like api.example.com or *.example.com, without a scheme, port or path",
        );
    }
    if let Some(rest) = pattern.strip_prefix("*.") {
        if rest.contains('*') {
            return Some("can only have one * at its start");
        }
        if IPV4.is_match(rest) {
            return Some("can't put * in front of an IP address");
        }
        if !is_host_name(rest) || rest.split('.').count() < 2 {
            return Some(
                "must put * in front of a domain with at least two parts, like *.example.com",
            );
        }
        return None;
    }
    if pattern.contains('*') {
        return Some("can only have * on its own or as *. at its start");
    }
    if !is_host_name(pattern) {
        return Some("isn't a valid host name");
    }
    None
}

/// `hostPatternProblem` (`pluginNetwork.ts:21`): why `pattern` is not a host pattern, or `None`.
pub fn host_pattern_problem(pattern: &Json) -> Option<&'static str> {
    match pattern {
        Json::String(s) => text_problem(s),
        _ => Some("isn't text"),
    }
}

/// `isHostPattern` (`pluginNetwork.ts:38`).
pub fn is_host_pattern(pattern: &Json) -> bool {
    host_pattern_problem(pattern).is_none()
}

/// `matchesAllowedHost` (`pluginNetwork.ts:40`): whether one of `allowed_hosts` lets `host` through.
pub fn matches_allowed_host(host: &str, allowed_hosts: &[&str]) -> bool {
    let target = normalized_host(host);
    if target.is_empty() || target.encode_utf16().count() > MAX_HOST_LENGTH {
        return false;
    }
    for &pattern in allowed_hosts {
        if text_problem(pattern).is_some() {
            continue;
        }
        if pattern == ANY_HOST {
            return true;
        }
        if let Some(domain) = pattern.strip_prefix("*.") {
            if target
                .strip_suffix(domain)
                .is_some_and(|t| t.ends_with('.'))
            {
                return true;
            }
        } else if target == pattern {
            return true;
        }
    }
    false
}

/// `hostPatternCovers` (`pluginNetwork.ts:54`): whether `broader` allows every host `narrower` does.
pub fn host_pattern_covers(broader: &str, narrower: &str) -> bool {
    if text_problem(broader).is_some() || text_problem(narrower).is_some() {
        return false;
    }
    if broader == ANY_HOST || broader == narrower {
        return true;
    }
    if narrower == ANY_HOST || !broader.starts_with("*.") {
        return false;
    }
    matches_allowed_host(narrower.strip_prefix("*.").unwrap_or(narrower), &[broader])
}
