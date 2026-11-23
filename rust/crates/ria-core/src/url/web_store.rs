// Ported from ardia packages/core/src/webStore.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Chrome Web Store extension ids and permission warnings.

use crate::fuzzy::is_js_space;
use ::url::Url;

pub struct ExtensionPermissions {
    pub permissions: Vec<String>,
    pub host_permissions: Vec<String>,
}

/// Port of `isExtensionId` (`webStore.ts:3`): `/^[a-p]{32}$/`.
pub fn is_extension_id(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| (b'a'..=b'p').contains(&b))
}

/// Port of `webStoreExtensionId` (`webStore.ts:5`).
pub fn web_store_extension_id(url_or_id: &str) -> Option<String> {
    let s = url_or_id.trim_matches(is_js_space);
    if is_extension_id(s) {
        return Some(s.to_string());
    }
    let url = Url::parse(s).ok()?;
    let host = url.host_str().unwrap_or("");
    let store = host == "chromewebstore.google.com"
        || (host == "chrome.google.com" && url.path().starts_with("/webstore/"));
    if !store {
        return None;
    }
    url.path()
        .split('/')
        .find(|p| is_extension_id(p))
        .map(str::to_string)
}

/// `/^(\*|https?|wss?):\/\/\*\//`.
fn is_all_hosts_pattern(h: &str) -> bool {
    ["*", "http", "https", "ws", "wss"].iter().any(|scheme| {
        h.strip_prefix(scheme)
            .is_some_and(|r| r.starts_with("://*/"))
    })
}

/// `hostLabel` (`webStore.ts:50`): `/^[^:]+:\/\/(\*\.)?([^/]+)/`.
fn host_label(pattern: &str) -> Option<String> {
    let at = pattern.find(':')?;
    if at == 0 {
        return None;
    }
    let rest = pattern[at..].strip_prefix("://")?;
    let name = |s: &str| s.split('/').next().unwrap_or("").to_string();
    if let Some(wild) = rest.strip_prefix("*.") {
        let host = name(wild);
        if !host.is_empty() {
            return Some(format!("all {host} sites"));
        }
    }
    let host = name(rest);
    (!host.is_empty()).then_some(host)
}

const PERMISSION_WARNINGS: &[(&str, &str)] = &[
    ("bookmarks", "Read and change your bookmarks"),
    ("clipboardRead", "Read data you copy and paste"),
    ("clipboardWrite", "Modify data you copy and paste"),
    (
        "contentSettings",
        "Change your settings that control websites' access to features such as cookies, JavaScript, plugins, geolocation, microphone, camera etc.",
    ),
    ("desktopCapture", "Capture content of your screen"),
    ("downloads", "Manage your downloads"),
    ("downloads.open", "Open downloaded files"),
    ("geolocation", "Detect your physical location"),
    ("management", "Manage your apps, extensions, and themes"),
    (
        "nativeMessaging",
        "Communicate with cooperating native applications",
    ),
    ("notifications", "Display notifications"),
    ("privacy", "Change your privacy-related settings"),
    ("readingList", "Read and change entries in the reading list"),
    (
        "sessions",
        "Read your browsing history on all your signed-in devices",
    ),
    ("system.storage", "Identify and eject storage devices"),
    ("tabCapture", "Capture content of your screen"),
    ("tabGroups", "View and manage your tab groups"),
    (
        "topSites",
        "Read a list of your most frequently visited websites",
    ),
    ("ttsEngine", "Read all text spoken using synthesized speech"),
    (
        "webAuthenticationProxy",
        "Read and change all your data on all websites",
    ),
];

/// Pushes `warning` unless it is already in the list (the `new Set` of the original).
fn push_unique(warnings: &mut Vec<String>, warning: String) {
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

/// Port of `permissionWarnings` (`webStore.ts:21`).
pub fn permission_warnings(pkg: &ExtensionPermissions) -> Vec<String> {
    let mut warnings: Vec<String> = Vec::new();
    let has = |p: &str| pkg.permissions.iter().any(|x| x == p);
    let hosts: Vec<&String> = pkg
        .host_permissions
        .iter()
        .filter(|h| {
            !["chrome:", "chrome-extension:", "about:", "data:"]
                .iter()
                .any(|p| h.starts_with(p))
        })
        .collect();
    let all_hosts = hosts
        .iter()
        .any(|h| *h == "<all_urls>" || is_all_hosts_pattern(h))
        || has("debugger");
    if all_hosts || has("proxy") || has("pageCapture") || has("debugger") {
        warnings.push("Read and change all your data on all websites".into());
    } else if !hosts.is_empty() {
        let mut names: Vec<String> = Vec::new();
        for name in hosts.iter().filter_map(|h| host_label(h)) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        let text = match names.as_slice() {
            [] => None,
            [a] => Some(format!("Read and change your data on {a}")),
            [a, b] => Some(format!("Read and change your data on {a} and {b}")),
            [a, b, c] => Some(format!("Read and change your data on {a}, {b}, and {c}")),
            _ => Some("Read and change your data on a number of websites".to_string()),
        };
        warnings.extend(text);
    }
    let clipboard = has("clipboardRead") && has("clipboardWrite");
    if clipboard {
        warnings.push("Read and modify data you copy and paste".into());
    }
    for (permission, message) in PERMISSION_WARNINGS {
        if clipboard && permission.starts_with("clipboard") {
            continue;
        }
        if has(permission) {
            warnings.push((*message).to_string());
        }
    }
    if has("declarativeNetRequest") && !all_hosts && !has("declarativeNetRequestWithHostAccess") {
        warnings.push("Block content on any page".into());
    }
    let history = has("history");
    if !history && (has("tabs") || has("webNavigation")) && !all_hosts {
        warnings.push("Read your browsing history".into());
    }
    if history {
        warnings.push("Read and change your browsing history on all your signed-in devices".into());
    }
    let mut unique = Vec::new();
    for w in warnings {
        push_unique(&mut unique, w);
    }
    unique
}
