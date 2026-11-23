// Ported from ardia packages/core/src/providerConfig.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `omniboxProviders` setting: checking each provider entry, and the provider configs the
//! omnibox runs.

use std::collections::HashSet;

use super::host::host_of;
use super::js_text::{is_js_space, js_trim};
use super::provider_rows::{
    JsValue, PROVIDER_LIMITS, ProviderRowInput, provider_icon, provider_url, strip_prefix_ci,
    utf16_len,
};

/// `ProviderEntry` (`providerConfig.ts:4`): one checked entry of the setting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProviderEntry {
    pub keyword: Option<String>,
    pub title: Option<String>,
    pub global: Option<bool>,
    pub rows: Option<Vec<ProviderRowInput>>,
    pub url: Option<String>,
    pub suggest: Option<String>,
}

/// `ProviderConfig` (`providerConfig.ts:13`).
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderConfig {
    pub id: String,
    pub title: String,
    pub keyword: Option<String>,
    pub global: bool,
    pub rows: Vec<ProviderRowInput>,
    pub url: Option<String>,
    pub suggest: Option<String>,
}

/// Where entry `n` of [`ProviderEntries::entries`] came from: the index of its list item and,
/// when it has rows, the indices of the rows kept. The TS code returns the input objects
/// themselves; a caller that must give back the same objects maps them with this.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntryOrigin {
    pub item: usize,
    pub rows: Vec<usize>,
}

/// The result of [`read_provider_entries`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProviderEntries {
    pub entries: Vec<ProviderEntry>,
    pub origins: Vec<EntryOrigin>,
    pub problems: Vec<String>,
}

/// `PROVIDER_CONFIG_LIMITS` (`providerConfig.ts:23`).
pub const PROVIDER_CONFIG_LIMITS_PROVIDERS: usize = 32;
/// `PROVIDER_CONFIG_LIMITS` (`providerConfig.ts:23`).
pub const PROVIDER_CONFIG_LIMITS_ROWS: usize = 200;

/// `PROVIDER_ENTRY_KEYS` (`providerConfig.ts:25`).
pub const PROVIDER_ENTRY_KEYS: [&str; 6] = ["keyword", "title", "global", "rows", "url", "suggest"];
const ROW_KEYS: [&str; 4] = ["title", "url", "subtitle", "icon"];

/// `QUERY_SLOT` (`providerConfig.ts:30`).
pub const QUERY_SLOT: &str = "{query}";

/// Port of `isProviderKeyword` (`providerConfig.ts:32`): `KEYWORD`, 1 to 32 code points without
/// white space or `/ ? # : @ . !`.
pub fn is_provider_keyword(value: &str) -> bool {
    let count = value.chars().count();
    (1..=PROVIDER_LIMITS.keyword).contains(&count)
        && !value
            .chars()
            .any(|c| is_js_space(c) || "/?#:@.!".contains(c))
}

/// `LOOPBACK` (`providerConfig.ts:28`):
/// `^http:\/\/(?:127\.0\.0\.1|localhost|\[::1\])(?::\d{1,5})?[/?]/i`.
fn is_loopback(url: &str) -> bool {
    let Some(rest) = strip_prefix_ci(url, "http://") else {
        return false;
    };
    let Some(rest) = ["127.0.0.1", "localhost", "[::1]"]
        .iter()
        .find_map(|host| strip_prefix_ci(rest, host))
    else {
        return false;
    };
    let rest = match rest.strip_prefix(':') {
        Some(port) => {
            let digits = port.bytes().take_while(u8::is_ascii_digit).count();
            if !(1..=5).contains(&digits) {
                return false;
            }
            &port[digits..]
        }
        None => rest,
    };
    rest.starts_with(['/', '?'])
}

/// Port of `isRemoteEndpoint` (`providerConfig.ts:34`): a `{query}` template that is https or
/// loopback http.
pub fn is_remote_endpoint(template: &str) -> bool {
    if !template.contains(QUERY_SLOT) {
        return false;
    }
    let sample = template.replace(QUERY_SLOT, "q");
    provider_url(Some(&sample)).is_some()
        && (strip_prefix_ci(&sample, "https://").is_some() || is_loopback(&sample))
}

/// `isTemplate` (`providerConfig.ts:40`).
fn is_template(template: &str) -> bool {
    template.contains(QUERY_SLOT)
        && provider_url(Some(&template.replace(QUERY_SLOT, "q"))).is_some()
}

/// The keys of `object` outside `allowed`, joined as `Object.keys(...).filter(...).join(", ")`.
fn unknown_keys(fields: &[(String, JsValue)], allowed: &[&str]) -> String {
    let unknown: Vec<&str> = fields
        .iter()
        .map(|(k, _)| k.as_str())
        .filter(|k| !allowed.contains(k))
        .collect();
    unknown.join(", ")
}

/// One row of `checkRows` (`providerConfig.ts:52`): the row, or the problem with it.
fn check_row(row: &JsValue, at: &str) -> Result<ProviderRowInput, String> {
    let JsValue::Object(fields) = row else {
        return Err(format!("{at} must be an object with title and url"));
    };
    let unknown = unknown_keys(fields, &ROW_KEYS);
    if !unknown.is_empty() {
        return Err(format!(
            "{at}: {unknown} isn't a row field (title, url, subtitle, icon)"
        ));
    }
    let Some(title) = row.get("title").as_str().filter(|t| !js_trim(t).is_empty()) else {
        return Err(format!("{at}.title must be a string"));
    };
    let url = row.get("url").as_str();
    if provider_url(url).is_none() {
        return Err(format!("{at}.url must be an http or https URL"));
    }
    let subtitle = match row.get("subtitle") {
        JsValue::Undefined => None,
        JsValue::String(s) => Some(s.clone()),
        _ => return Err(format!("{at}.subtitle must be a string")),
    };
    let icon = match row.get("icon") {
        JsValue::Undefined | JsValue::Null => None,
        icon => match provider_icon(icon.as_str()) {
            Some(_) => icon.as_str().map(str::to_string),
            None => {
                return Err(format!(
                    "{at}.icon must be an https URL or a small data:image"
                ));
            }
        },
    };
    Ok(ProviderRowInput {
        title: title.to_string(),
        url: url.unwrap_or_default().to_string(),
        subtitle,
        icon,
    })
}

/// Port of `checkRows` (`providerConfig.ts:44`): the kept rows with their indices, or `None`
/// when `rows` is not a list.
fn check_rows(
    rows: &JsValue,
    at: &str,
    problems: &mut Vec<String>,
) -> Option<Vec<(usize, ProviderRowInput)>> {
    let JsValue::Array(rows) = rows else {
        problems.push(format!("{at}.rows must be a list of {{ title, url }} rows"));
        return None;
    };
    if rows.len() > PROVIDER_CONFIG_LIMITS_ROWS {
        problems.push(format!(
            "{at}.rows has more than {PROVIDER_CONFIG_LIMITS_ROWS} rows; the rest are skipped"
        ));
    }
    let mut kept = Vec::new();
    for (i, row) in rows.iter().take(PROVIDER_CONFIG_LIMITS_ROWS).enumerate() {
        match check_row(row, &format!("{at}.rows[{i}]")) {
            Ok(row) => kept.push((i, row)),
            Err(problem) => problems.push(problem),
        }
    }
    Some(kept)
}

/// The string of an entry field that passed its check: a string or `undefined`.
fn checked_string(value: &JsValue) -> Option<String> {
    value.as_str().map(str::to_string)
}

/// Port of `checkEntry` (`providerConfig.ts:67`): the entry and the kept rows' indices, or `None`,
/// and every problem found.
fn check_entry(value: &JsValue, at: &str) -> (Option<(ProviderEntry, Vec<usize>)>, Vec<String>) {
    let fail = |problem: String| (None, vec![format!("{at}{problem}")]);
    let JsValue::Object(fields) = value else {
        return fail(" must be an object".into());
    };
    let unknown = unknown_keys(fields, &PROVIDER_ENTRY_KEYS);
    if !unknown.is_empty() {
        return fail(format!(
            ": {unknown} isn't a provider field ({})",
            PROVIDER_ENTRY_KEYS.join(", ")
        ));
    }
    let defined = |key| *value.get(key) != JsValue::Undefined;
    let keyword = value.get("keyword");
    if defined("keyword") && !keyword.as_str().is_some_and(is_provider_keyword) {
        return fail(".keyword must be 1 to 32 characters without spaces or / ? # : @ . !".into());
    }
    let title_ok =
        |t: &str| !js_trim(t).is_empty() && utf16_len(t) <= PROVIDER_LIMITS.provider_title;
    if defined("title") && !value.get("title").as_str().is_some_and(title_ok) {
        return fail(format!(
            ".title must be a string of 1 to {} characters",
            PROVIDER_LIMITS.provider_title
        ));
    }
    let global = match value.get("global") {
        JsValue::Undefined => None,
        JsValue::Bool(b) => Some(*b),
        _ => return fail(".global must be true or false".into()),
    };
    if global == Some(true) && defined("keyword") {
        return fail(": a provider with a keyword shows only in its keyword mode; drop global or the keyword".into());
    }
    if defined("url") && !value.get("url").as_str().is_some_and(is_template) {
        return fail(format!(
            ".url must be an http or https URL containing {QUERY_SLOT}"
        ));
    }
    if defined("url") && !defined("keyword") {
        return fail(
            ".url needs a keyword: it turns what you type after the keyword into a link".into(),
        );
    }
    if defined("suggest")
        && !value
            .get("suggest")
            .as_str()
            .is_some_and(is_remote_endpoint)
    {
        return fail(format!(
            ".suggest must be an https URL containing {QUERY_SLOT}"
        ));
    }
    if defined("suggest") && !defined("keyword") && global != Some(true) {
        return fail(
            ".suggest without a keyword needs global: true, since it sends everything you type to that server".into(),
        );
    }
    let mut problems = Vec::new();
    let rows = if defined("rows") {
        match check_rows(value.get("rows"), at, &mut problems) {
            Some(rows) => Some(rows),
            None => return (None, problems),
        }
    } else {
        None
    };
    let has_endpoint = defined("url") || defined("suggest");
    match &rows {
        None if !has_endpoint => return fail(" needs rows, url or suggest".into()),
        Some(rows) if rows.is_empty() && !has_endpoint => return (None, problems),
        _ => {}
    }
    let (row_indices, rows) = match rows {
        Some(rows) => {
            let (indices, rows) = rows.into_iter().unzip();
            (indices, Some(rows))
        }
        None => (vec![], None),
    };
    let entry = ProviderEntry {
        keyword: checked_string(keyword),
        title: checked_string(value.get("title")),
        global,
        rows,
        url: checked_string(value.get("url")),
        suggest: checked_string(value.get("suggest")),
    };
    (Some((entry, row_indices)), problems)
}

/// Port of `readProviderEntries` (`providerConfig.ts:91`): the usable entries of the setting
/// named `name`, and a problem for each part skipped. Keywords are unique, case-insensitively.
pub fn read_provider_entries(value: &JsValue, name: &str) -> ProviderEntries {
    let mut out = ProviderEntries::default();
    let JsValue::Array(items) = value else {
        out.problems
            .push(format!("{name} must be a list of providers"));
        return out;
    };
    let mut keywords = HashSet::new();
    for (i, item) in items.iter().enumerate() {
        let at = format!("{name}[{i}]");
        if out.entries.len() >= PROVIDER_CONFIG_LIMITS_PROVIDERS {
            out.problems.push(format!(
                "{at}: at most {PROVIDER_CONFIG_LIMITS_PROVIDERS} providers"
            ));
            continue;
        }
        let (checked, problems) = check_entry(item, &at);
        out.problems.extend(problems);
        let Some((entry, rows)) = checked else {
            continue;
        };
        if let Some(keyword) = entry.keyword.as_deref().map(str::to_lowercase) {
            if keywords.contains(&keyword) {
                out.problems.push(format!(
                    "{at}.keyword “{keyword}” is already used by another provider"
                ));
                continue;
            }
            keywords.insert(keyword);
        }
        out.entries.push(entry);
        out.origins.push(EntryOrigin { item: i, rows });
    }
    out
}

/// Port of `isProviderEntryList` (`providerConfig.ts:110`): a list without any problem.
pub fn is_provider_entry_list(value: &JsValue) -> bool {
    matches!(value, JsValue::Array(_))
        && read_provider_entries(value, "omniboxProviders")
            .problems
            .is_empty()
}

/// Port of `entryTitle` (`providerConfig.ts:113`): the title, else the endpoint's host, else the
/// keyword, else "Links".
fn entry_title(e: &ProviderEntry) -> String {
    if let Some(title) = e.title.as_deref().filter(|t| !t.is_empty()) {
        return js_trim(title).to_string();
    }
    let endpoint = e.url.as_deref().or(e.suggest.as_deref()).unwrap_or("");
    let host = host_of(&endpoint.replace(QUERY_SLOT, "q"));
    if !host.is_empty() {
        return host;
    }
    e.keyword
        .clone()
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| "Links".into())
}

/// Port of `providerConfigs` (`providerConfig.ts:119`): one config per entry, with unique ids.
pub fn provider_configs(entries: &[ProviderEntry]) -> Vec<ProviderConfig> {
    let mut ids = HashSet::new();
    entries
        .iter()
        .map(|e| {
            let keyword = e.keyword.as_deref().map(str::to_lowercase);
            let title = entry_title(e);
            let base = format!(
                "config:{}",
                keyword.clone().unwrap_or_else(|| title.to_lowercase())
            );
            let mut id = base.clone();
            let mut n = 2;
            while ids.contains(&id) {
                id = format!("{base}:{n}");
                n += 1;
            }
            ids.insert(id.clone());
            let keyword = keyword.filter(|k| !k.is_empty());
            ProviderConfig {
                id,
                title,
                global: keyword.is_none(),
                keyword,
                rows: e.rows.clone().unwrap_or_default(),
                url: e.url.clone().filter(|u| !u.is_empty()),
                suggest: e.suggest.clone().filter(|s| !s.is_empty()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_needs_a_path_or_query_after_the_port() {
        assert!(is_remote_endpoint("http://127.0.0.1:8123/s?q={query}"));
        assert!(is_remote_endpoint("HTTP://LOCALHOST?{query}"));
        assert!(!is_remote_endpoint("http://localhost:123456/{query}"));
        assert!(!is_remote_endpoint("http://localhost#{query}"));
        assert!(!is_remote_endpoint("http://a.com/s?q={query}"));
    }

    #[test]
    fn keywords_count_code_points() {
        assert!(is_provider_keyword(&"😀".repeat(32)));
        assert!(!is_provider_keyword(&"😀".repeat(33)));
        assert!(!is_provider_keyword("a.b"));
    }
}
