// Ported from ardia apps/browser/src/lib/pluginPageBar.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Site switches and the striped page bar a plugin can add (`contributes.siteSwitches`,
//! `contributes.pageBar`): checking them, the origin a switch keys on, and the consent lines.
//!
//! Changes from Ardia:
//! - `HOST_ACTIONS` is a list of `HostActionInfo` in the TS key order; `HostAction`,
//!   `PageBarButton` and `PageBarLook` are `&'static str`.
//! - `RowRef` borrows the row's key and type.
//! - `unknown` is `Option<&Json>`: `None` is `undefined`.
//! - `pageBarConsent` with a bar without host actions writes `undefined` as the TS does.

use super::decode::Fields;
use super::plugin_checks::{
    Check, KEY, KEY_RULE, at, list, matching, object, only, symbol_of, text, unique,
};
use super::url_parts::url_parts;
use regex::Regex;
use ria_core::identity::telemetry_wire::Json;
use serde::Serialize;
use std::sync::LazyLock;

/// One entry of `HOST_ACTIONS` (`pluginPageBar.ts:4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostActionInfo {
    pub name: &'static str,
    pub title: &'static str,
    pub consent: &'static str,
}

const fn host(name: &'static str, title: &'static str, consent: &'static str) -> HostActionInfo {
    HostActionInfo {
        name,
        title,
        consent,
    }
}

/// `HOST_ACTIONS` (`pluginPageBar.ts:4`).
pub const HOST_ACTIONS: [HostActionInfo; 8] = [
    host("siteControls", "Site Controls", "open Site Controls"),
    host("address", "Edit Address", "edit the address"),
    host("copyLink", "Copy URL", "copy the page address"),
    host("console", "Toggle Console", "open the Console"),
    host("network", "Toggle Network Panel", "open the Network panel"),
    host("inspect", "Inspect Element", "inspect an element"),
    host("extensions", "Extensions", "run your extensions"),
    host("split", "Split View Options", "add a split"),
];
/// `HostAction` (`pluginPageBar.ts:14`): a `name` of `HOST_ACTIONS`.
pub type HostAction = &'static str;
/// `HOST_ACTION_NAMES` (`pluginPageBar.ts:15`).
pub const HOST_ACTION_NAMES: [&str; 8] = [
    "siteControls",
    "address",
    "copyLink",
    "console",
    "network",
    "inspect",
    "extensions",
    "split",
];
/// `BUTTON_DIVIDER` (`pluginPageBar.ts:16`).
pub const BUTTON_DIVIDER: &str = "|";
/// `PageBarButton` (`pluginPageBar.ts:17`): a host action or `BUTTON_DIVIDER`.
pub type PageBarButton = &'static str;
/// `PAGE_BAR_LOOKS` (`pluginPageBar.ts:18`).
pub const PAGE_BAR_LOOKS: [&str; 1] = ["stripes"];
/// `PageBarLook` (`pluginPageBar.ts:19`).
pub type PageBarLook = &'static str;

/// `PAGE_BAR_LIMITS` (`pluginPageBar.ts:21`).
pub struct PageBarLimits {
    pub site_switches: usize,
    pub buttons: usize,
    pub switch_title: usize,
}
/// `PAGE_BAR_LIMITS` (`pluginPageBar.ts:21`).
pub const PAGE_BAR_LIMITS: PageBarLimits = PageBarLimits {
    site_switches: 4,
    buttons: 16,
    switch_title: 40,
};

/// `SiteSwitchSpec` (`pluginPageBar.ts:23`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SiteSwitchSpec {
    pub key: String,
    pub title: String,
    pub icon: String,
}

/// `PageBarSpec["when"]` (`pluginPageBar.ts:24`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageBarWhen {
    pub site_switch: Option<String>,
    pub local_addresses: Option<String>,
}

/// `PageBarSpec` (`pluginPageBar.ts:24`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PageBarSpec {
    pub look: PageBarLook,
    pub when: PageBarWhen,
    pub buttons: Vec<PageBarButton>,
}

/// `RowRef` (`pluginPageBar.ts:28`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowRef<'a> {
    pub key: &'a str,
    pub r#type: &'a str,
}

/// A consent line of `pageBarConsent` (`pluginPageBar.ts:102`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConsentLine {
    pub icon: String,
    pub text: String,
}

const P: &str = "contributes";

fn field<'a>(doc: &'a Fields, key: &str) -> Option<&'a Json> {
    doc.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// The host action named `name`.
fn host_action(name: &str) -> Option<HostAction> {
    HOST_ACTION_NAMES.into_iter().find(|n| *n == name)
}

/// `isHostAction` (`pluginPageBar.ts:26`).
pub fn is_host_action(value: Option<&Json>) -> bool {
    matches!(value, Some(Json::String(s)) if host_action(s).is_some())
}

/// `siteSwitchesOf` (`pluginPageBar.ts:32`).
pub fn site_switches_of(c: &mut Check, value: Option<&Json>) -> Vec<SiteSwitchSpec> {
    let path = format!("{P}.siteSwitches");
    let mut out = Vec::new();
    for (i, raw) in list(c, &path, value, PAGE_BAR_LIMITS.site_switches)
        .iter()
        .enumerate()
    {
        let at_ = at(&path, i);
        let Some(doc) = object(c, &at_, Some(raw)) else {
            continue;
        };
        only(c, &at_, doc, &["key", "title", "icon"]);
        let key = matching(
            c,
            &format!("{at_}.key"),
            field(doc, "key"),
            &KEY,
            KEY_RULE,
            true,
        );
        let title = text(
            c,
            &format!("{at_}.title"),
            field(doc, "title"),
            PAGE_BAR_LIMITS.switch_title,
            true,
        );
        let icon = symbol_of(c, &format!("{at_}.icon"), field(doc, "icon"), "puzzlepiece");
        if let (Some(key), Some(title)) = (key, title) {
            out.push(SiteSwitchSpec {
                key: key.to_owned(),
                title: title.to_owned(),
                icon: icon.to_owned(),
            });
        }
    }
    unique(c, &path, out.iter().map(|s| Some(&s.key)), "switches");
    out
}

/// `buttonsOf` (`pluginPageBar.ts:48`).
fn buttons_of(c: &mut Check, value: Option<&Json>) -> Vec<PageBarButton> {
    let path = format!("{P}.pageBar.buttons");
    let items = match value {
        Some(Json::Array(items)) if !items.is_empty() && items.len() <= PAGE_BAR_LIMITS.buttons => {
            items
        }
        _ => {
            c.problems.push(format!(
                "{path} must be a list of 1 to {}",
                PAGE_BAR_LIMITS.buttons
            ));
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for (i, b) in items.iter().enumerate() {
        let name = match b {
            Json::String(s) if s == BUTTON_DIVIDER => {
                out.push(BUTTON_DIVIDER);
                continue;
            }
            Json::String(s) => host_action(s),
            _ => None,
        };
        let Some(name) = name else {
            c.problems.push(format!(
                "{} must be {BUTTON_DIVIDER} or one of {}",
                at(&path, i),
                HOST_ACTION_NAMES.join(", ")
            ));
            continue;
        };
        if out.contains(&name) {
            c.problems.push(format!("{path} has {name} twice"));
        } else {
            out.push(name);
        }
    }
    if !out.is_empty() && !out.iter().any(|b| host_action(b).is_some()) {
        c.problems.push(format!(
            "{path} needs at least one action besides {BUTTON_DIVIDER}"
        ));
    }
    out
}

/// `pageBarOf` (`pluginPageBar.ts:65`).
pub fn page_bar_of(
    c: &mut Check,
    value: Option<&Json>,
    switches: &[SiteSwitchSpec],
    rows: &[RowRef],
) -> Option<PageBarSpec> {
    let path = format!("{P}.pageBar");
    value?;
    let doc = object(c, &path, value)?;
    only(c, &path, doc, &["look", "when", "buttons"]);
    let look = match field(doc, "look") {
        Some(Json::String(l)) => PAGE_BAR_LOOKS.into_iter().find(|x| x == l),
        _ => None,
    };
    if look.is_none() {
        c.problems.push(format!(
            "{path}.look must be {}",
            PAGE_BAR_LOOKS.join(" or ")
        ));
    }
    let when_path = format!("{path}.when");
    let no_fields = Fields::new();
    let when = object(c, &when_path, field(doc, "when")).unwrap_or(&no_fields);
    only(c, &when_path, when, &["siteSwitch", "localAddresses"]);
    let site_switch = match field(when, "siteSwitch") {
        Some(Json::String(s)) if switches.iter().any(|x| &x.key == s) => Some(s.clone()),
        _ => None,
    };
    if site_switch.is_none() {
        c.problems.push(format!(
            "{path}.when.siteSwitch must name one of this plugin's siteSwitches"
        ));
    }
    let mut local_addresses = None;
    if let Some(given) = field(when, "localAddresses") {
        match given {
            Json::String(s) if rows.iter().any(|r| r.key == s && r.r#type == "toggle") => {
                local_addresses = Some(s.clone())
            }
            _ => c.problems.push(format!(
                "{path}.when.localAddresses must name one of this plugin's toggle rows"
            )),
        }
    }
    let buttons = buttons_of(c, field(doc, "buttons"));
    if !buttons.iter().any(|b| host_action(b).is_some()) {
        return None;
    }
    Some(PageBarSpec {
        look: look?,
        when: PageBarWhen {
            site_switch: Some(site_switch?),
            local_addresses,
        },
        buttons,
    })
}

/// `switchOrigin` (`pluginPageBar.ts:87`): `scheme://host[:port]` of an http(s) URL.
pub fn switch_origin(url: &str) -> Option<String> {
    let parts = url_parts(url)?;
    if (parts.scheme != "http" && parts.scheme != "https") || parts.host.is_empty() {
        return None;
    }
    let host = parts.host.strip_suffix('.').unwrap_or(&parts.host);
    if host.is_empty() {
        return None;
    }
    let port = if parts.port.is_empty() {
        String::new()
    } else {
        format!(":{}", parts.port)
    };
    Some(format!("{}://{host}{port}", parts.scheme))
}

static LOCAL_HOST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:localhost|[a-z0-9_-]+(?:\.[a-z0-9_-]+)*\.localhost|127(?:\.[0-9]{1,3}){3}|0\.0\.0\.0|\[::1\])$").expect("LOCAL_HOST")
});
static SCHEME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^https?://").expect("SCHEME"));
static PORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":[0-9]+$").expect("PORT"));

/// `isLocalOrigin` (`pluginPageBar.ts:96`).
pub fn is_local_origin(origin: &str) -> bool {
    let host = SCHEME.replace(origin, "");
    let host = PORT.replace(&host, "");
    LOCAL_HOST.is_match(&host)
}

/// `pageBarConsent` (`pluginPageBar.ts:101`).
pub fn page_bar_consent(
    switches: &[SiteSwitchSpec],
    bar: Option<&PageBarSpec>,
) -> Vec<ConsentLine> {
    let mut lines = Vec::new();
    if let Some(first) = switches.first() {
        let what = if switches.len() == 1 {
            format!("a {} switch", first.title)
        } else {
            format!(
                "switches ({})",
                switches
                    .iter()
                    .map(|s| s.title.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        lines.push(ConsentLine {
            icon: first.icon.clone(),
            text: format!("Adds {what} to Site Controls, for each site"),
        });
    }
    let Some(bar) = bar else { return lines };
    let verbs: Vec<&str> = bar
        .buttons
        .iter()
        .filter_map(|b| HOST_ACTIONS.iter().find(|h| h.name == *b))
        .map(|h| h.consent)
        .collect();
    let list = match verbs.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        Some((only_one, _)) => (*only_one).to_owned(),
        None => "undefined".to_owned(),
    };
    let title = bar
        .when
        .site_switch
        .as_ref()
        .and_then(|k| switches.iter().find(|s| &s.key == k))
        .map_or("it", |s| s.title.as_str());
    lines.push(ConsentLine {
        icon: "menubar.rectangle".to_owned(),
        text: format!("Shows a striped bar on sites where {title} is on, with buttons to {list}"),
    });
    if bar
        .when
        .local_addresses
        .as_deref()
        .is_some_and(|l| !l.is_empty())
    {
        lines.push(ConsentLine {
            icon: "desktopcomputer".to_owned(),
            text: format!("With one of its settings on, a local address you type in (like localhost) turns {title} on for that site"),
        });
    }
    lines
}
