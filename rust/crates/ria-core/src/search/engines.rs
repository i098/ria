// Ported from ardia packages/core/src/engines.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Search engines: the built-in list, custom and extension engines, search and suggest URLs, and
//! the reading of remote suggest replies.
//!
//! The TS functions that take `unknown` JSON (`customEngineFrom`, `parseRemoteSuggestions`) take
//! here the typed parts they read; the caller does the JS `typeof` checks when it reads the JSON.
//! `suggestReplyJson` is split the same way: [`strip_reply_prefix`] is the port, the JSON text
//! parsing stays with the caller.

use std::collections::HashSet;
use std::sync::LazyLock;

use super::host::{host_of, search_url_for};
use super::js_text::{is_js_space, js_trim};

/// `SearchEngine` (`engines.ts:3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchEngine {
    pub id: String,
    pub name: String,
    pub keyword: String,
    pub url: String,
    pub suggest_url: Option<String>,
    pub custom: bool,
    pub extension: Option<EngineExtension>,
}

/// The `extension` field of [`SearchEngine`]: the extension that added the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineExtension {
    pub id: String,
    pub name: String,
}

/// `CustomSearchEngine` (`engines.ts:13`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomSearchEngine {
    pub id: String,
    pub name: String,
    pub keyword: String,
    pub url: String,
}

/// `ExtensionSearchEngine` (`engines.ts:15`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSearchEngine {
    pub profile: String,
    pub extension_id: String,
    pub extension_name: String,
    pub name: String,
    pub keyword: String,
    pub url: String,
    pub suggest_url: Option<String>,
    pub is_default: bool,
}

/// `(id, name, keyword, url, suggestUrl)` of each built-in engine.
type BuiltIn = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);

const BUILT_IN: [BuiltIn; 12] = [
    (
        "google",
        "Google",
        "google.com",
        "https://www.google.com/search?q=%s",
        Some("https://www.google.com/complete/search?client=chrome&q=%s"),
    ),
    (
        "bing",
        "Bing",
        "bing.com",
        "https://www.bing.com/search?q=%s",
        Some("https://api.bing.com/osjson.aspx?query=%s"),
    ),
    (
        "duckduckgo",
        "DuckDuckGo",
        "duckduckgo.com",
        "https://duckduckgo.com/?q=%s",
        Some("https://duckduckgo.com/ac/?q=%s&type=list"),
    ),
    (
        "perplexity",
        "Perplexity",
        "perplexity.ai",
        "https://www.perplexity.ai/search?q=%s",
        None,
    ),
    (
        "chatgpt",
        "ChatGPT",
        "chatgpt.com",
        "https://chatgpt.com/?q=%s",
        None,
    ),
    (
        "ecosia",
        "Ecosia",
        "ecosia.org",
        "https://www.ecosia.org/search?q=%s",
        Some("https://ac.ecosia.org/autocomplete?q=%s&type=list"),
    ),
    (
        "brave",
        "Brave Search",
        "search.brave.com",
        "https://search.brave.com/search?q=%s",
        Some("https://search.brave.com/api/suggest?q=%s"),
    ),
    (
        "kagi",
        "Kagi",
        "kagi.com",
        "https://kagi.com/search?q=%s",
        None,
    ),
    (
        "startpage",
        "Startpage",
        "startpage.com",
        "https://www.startpage.com/do/search?q=%s",
        Some("https://www.startpage.com/osuggestions?q=%s"),
    ),
    (
        "yahoo",
        "Yahoo",
        "yahoo.com",
        "https://search.yahoo.com/search?p=%s",
        Some("https://search.yahoo.com/sugg/ff?output=fxjson&command=%s"),
    ),
    (
        "baidu",
        "Baidu",
        "baidu.com",
        "https://www.baidu.com/s?wd=%s",
        Some("https://www.baidu.com/su?action=opensearch&ie=utf-8&wd=%s"),
    ),
    (
        "yandex",
        "Yandex",
        "yandex.com",
        "https://yandex.com/search/?text=%s",
        Some("https://suggest.yandex.com/suggest-ff.cgi?part=%s"),
    ),
];

/// `BUILT_IN_ENGINES` (`engines.ts:26`); Google is first and is the fallback of [`engine_by_id`].
pub static BUILT_IN_ENGINES: LazyLock<Vec<SearchEngine>> = LazyLock::new(|| {
    BUILT_IN
        .iter()
        .map(|&(id, name, keyword, url, suggest_url)| SearchEngine {
            id: id.into(),
            name: name.into(),
            keyword: keyword.into(),
            url: url.into(),
            suggest_url: suggest_url.map(Into::into),
            custom: false,
            extension: None,
        })
        .collect()
});

/// `/^https?:\/\/[^/?#\s]+/i.test(text)`. Without the `u` flag, `/i` folds ASCII letters only.
fn starts_with_http_host(text: &str) -> bool {
    let Some(rest) = text
        .get(..4)
        .filter(|s| s.eq_ignore_ascii_case("http"))
        .map(|_| &text[4..])
    else {
        return false;
    };
    let rest = rest.strip_prefix(['s', 'S']).unwrap_or(rest);
    rest.strip_prefix("://")
        .and_then(|r| r.chars().next())
        .is_some_and(|c| !matches!(c, '/' | '?' | '#') && !is_js_space(c))
}

/// Port of `validSearchTemplate` (`engines.ts:89`) for a string; any other JS value is not one.
pub fn valid_search_template(url: &str) -> bool {
    starts_with_http_host(url) && url.contains("%s")
}

/// The fields `customEngineFrom` reads from a stored engine: each one when it is a JS string.
/// A value that is not an object has none.
#[derive(Debug, Clone, Copy, Default)]
pub struct StoredEngine<'a> {
    pub id: Option<&'a str>,
    pub name: Option<&'a str>,
    pub keyword: Option<&'a str>,
    pub url: Option<&'a str>,
}

/// Port of `customEngineFrom` (`engines.ts:91`).
pub fn custom_engine_from(raw: &StoredEngine) -> Option<CustomSearchEngine> {
    let url = raw.url.filter(|u| valid_search_template(u))?;
    Some(CustomSearchEngine {
        id: raw.id?.into(),
        name: raw.name?.into(),
        keyword: raw.keyword?.into(),
        url: url.into(),
    })
}

/// Port of `customEnginesFrom` (`engines.ts:98`) for an array; any other JS value gives none.
pub fn custom_engines_from(raw: &[StoredEngine]) -> Vec<CustomSearchEngine> {
    raw.iter().filter_map(custom_engine_from).collect()
}

/// Port of `allEngines` (`engines.ts:101`): the built-in engines, then the custom engines with a
/// valid template, then one engine per extension (the first one each extension gives).
pub fn all_engines(
    custom: &[CustomSearchEngine],
    extension: &[ExtensionSearchEngine],
) -> Vec<SearchEngine> {
    let mut engines = BUILT_IN_ENGINES.clone();
    engines.extend(
        custom
            .iter()
            .filter(|e| valid_search_template(&e.url))
            .map(|e| SearchEngine {
                id: e.id.clone(),
                name: e.name.clone(),
                keyword: e.keyword.clone(),
                url: e.url.clone(),
                suggest_url: None,
                custom: true,
                extension: None,
            }),
    );
    let mut seen = HashSet::new();
    for e in extension {
        if !seen.insert(e.extension_id.as_str()) {
            continue;
        }
        engines.push(SearchEngine {
            id: extension_engine_id(&e.extension_id),
            name: e.name.clone(),
            keyword: e.keyword.clone(),
            url: e.url.clone(),
            suggest_url: e.suggest_url.clone().filter(|s| !s.is_empty()),
            custom: false,
            extension: Some(EngineExtension {
                id: e.extension_id.clone(),
                name: e.extension_name.clone(),
            }),
        });
    }
    engines
}

/// Port of `extensionEngineId` (`engines.ts:119`).
pub fn extension_engine_id(extension_id: &str) -> String {
    format!("extension:{extension_id}")
}

/// Port of `controllingExtensionEngine` (`engines.ts:121`).
pub fn controlling_extension_engine(
    extension: &[ExtensionSearchEngine],
) -> Option<&ExtensionSearchEngine> {
    extension.iter().find(|e| e.is_default)
}

/// Port of `engineById` (`engines.ts:125`): Google when no engine has the id.
pub fn engine_by_id<'a>(engines: &'a [SearchEngine], id: Option<&str>) -> &'a SearchEngine {
    engines
        .iter()
        .find(|e| Some(e.id.as_str()) == id)
        .unwrap_or(&BUILT_IN_ENGINES[0])
}

/// Port of `searchUrl` (`engines.ts:129`); `engine_url` is the engine's `url`.
pub fn search_url(engine_url: &str, query: &str) -> String {
    search_url_for(engine_url, query)
}

/// Port of `suggestRequestUrl` (`engines.ts:131`): custom engines and blank queries ask nothing.
pub fn suggest_request_url(engine: &SearchEngine, query: &str) -> Option<String> {
    let suggest = engine.suggest_url.as_deref().filter(|s| !s.is_empty())?;
    (!engine.custom && !js_trim(query).is_empty()).then(|| search_url_for(suggest, query))
}

/// Port of `engineHost` (`engines.ts:134`); `engine_url` is the engine's `url`.
pub fn engine_host(engine_url: &str) -> String {
    host_of(&engine_url.replace("%s", "q"))
}

/// The text `suggestReplyJson` (`engines.ts:142`) gives `JSON.parse`: without the `)]}'` guard
/// some engines put before their JSON, and the white space after it.
pub fn strip_reply_prefix(text: &str) -> &str {
    text.strip_prefix(")]}'")
        .map_or(text, |rest| rest.trim_start_matches(is_js_space))
}

/// `RemoteSite` (`engines.ts:139`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSite {
    pub url: String,
    pub title: String,
}

/// `RemoteSuggestions` (`engines.ts:140`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoteSuggestions {
    pub queries: Vec<String>,
    pub sites: Vec<RemoteSite>,
}

/// What `parseRemoteSuggestions` reads from a parsed suggest reply `body`. The reader gives empty
/// `suggestions` when `body` or `body[1]` is not an array.
#[derive(Debug, Clone, Default)]
pub struct SuggestReply<'a> {
    /// `body[1]`: each item when it is a string.
    pub suggestions: Vec<Option<&'a str>>,
    /// `body[2]` when it is an array: each item when it is a string.
    pub titles: Vec<Option<&'a str>>,
    /// `String(item)` of each item of `body[4]["google:suggesttype"]` when that is an array.
    pub types: Vec<String>,
}

/// Port of `parseRemoteSuggestions` (`engines.ts:144`): navigation types become sites (http(s)
/// addresses only), calculator answers are dropped, the rest are queries.
pub fn parse_remote_suggestions(reply: &SuggestReply) -> RemoteSuggestions {
    let mut out = RemoteSuggestions::default();
    for (i, s) in reply.suggestions.iter().enumerate() {
        let Some(s) = s.map(js_trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        // `String(types[i])` is "undefined" past the end, which is not a known type.
        match reply.types.get(i).map_or("undefined", String::as_str) {
            "NAVIGATION" | "PERSONALIZED_NAVIGATION" => {
                if starts_with_http_host(s) {
                    let title = reply.titles.get(i).copied().flatten().map_or("", js_trim);
                    out.sites.push(RemoteSite {
                        url: s.into(),
                        title: title.into(),
                    });
                }
            }
            "CALCULATOR" => {}
            _ => out.queries.push(s.into()),
        }
    }
    out
}

/// Port of `parseSuggestions` (`engines.ts:161`).
pub fn parse_suggestions(reply: &SuggestReply) -> Vec<String> {
    parse_remote_suggestions(reply).queries
}

/// `EngineInput` (`engines.ts:163`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInput {
    pub name: String,
    pub keyword: String,
    pub url: String,
}

/// Port of `validateEngine` (`engines.ts:165`): the message to show, or `None` when the engine
/// is valid. `ignore_id` is the engine being edited, whose keyword may stay.
pub fn validate_engine(
    input: &EngineInput,
    engines: &[SearchEngine],
    ignore_id: Option<&str>,
) -> Option<String> {
    let name = js_trim(&input.name);
    let keyword = js_trim(&input.keyword).to_lowercase();
    let url = js_trim(&input.url);
    let message = if name.is_empty() {
        "Enter a name."
    } else if keyword.is_empty() {
        "Enter a shortcut."
    } else if keyword.contains(is_js_space) {
        "The shortcut can't contain spaces."
    } else if !starts_with_http_host(url) {
        "Enter a URL that starts with http:// or https://."
    } else if !url.contains("%s") {
        "The URL must contain %s where the search terms go."
    } else if engines
        .iter()
        .any(|e| Some(e.id.as_str()) != ignore_id && e.keyword.to_lowercase() == keyword)
    {
        return Some(format!(
            "“{keyword}” is already used by another search engine."
        ));
    } else {
        return None;
    };
    Some(message.into())
}

/// Port of `makeCustomEngine` (`engines.ts:178`).
pub fn make_custom_engine(input: &EngineInput, id: &str) -> CustomSearchEngine {
    CustomSearchEngine {
        id: id.into(),
        name: js_trim(&input.name).into(),
        keyword: js_trim(&input.keyword).to_lowercase(),
        url: js_trim(&input.url).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_needs_http_host_and_placeholder() {
        assert!(valid_search_template("HTTPS://x.org/?q=%s"));
        assert!(!valid_search_template("https:///?q=%s"));
        assert!(!valid_search_template("https:// x/?q=%s"));
        assert!(!valid_search_template("ftp://x/?q=%s"));
        assert!(!valid_search_template("https://x/"));
    }

    #[test]
    fn reply_sites_queries_and_calculator() {
        let reply = SuggestReply {
            suggestions: vec![
                Some(" maps "),
                Some("https://maps.google.com/"),
                Some("1+1"),
                None,
                Some("ftp://x"),
            ],
            titles: vec![None, Some(" Google Maps ")],
            types: vec![
                "QUERY".into(),
                "NAVIGATION".into(),
                "CALCULATOR".into(),
                "QUERY".into(),
                "NAVIGATION".into(),
            ],
        };
        let got = parse_remote_suggestions(&reply);
        assert_eq!(got.queries, ["maps"]);
        assert_eq!(
            got.sites,
            [RemoteSite {
                url: "https://maps.google.com/".into(),
                title: "Google Maps".into()
            }]
        );
    }

    #[test]
    fn extensions_keep_their_first_engine() {
        let ext = |id: &str, name: &str| ExtensionSearchEngine {
            profile: "p".into(),
            extension_id: id.into(),
            extension_name: "E".into(),
            name: name.into(),
            keyword: "k".into(),
            url: "https://e/?q=%s".into(),
            suggest_url: Some(String::new()),
            is_default: false,
        };
        let all = all_engines(&[], &[ext("a", "first"), ext("a", "second")]);
        let last = &all[all.len() - 1];
        assert_eq!(
            (all.len(), last.id.as_str(), last.name.as_str()),
            (13, "extension:a", "first")
        );
        assert_eq!(last.suggest_url, None);
    }

    #[test]
    fn duplicate_keyword_unless_ignored() {
        let input = EngineInput {
            name: "G".into(),
            keyword: " Google.COM ".into(),
            url: "https://g/?q=%s".into(),
        };
        assert_eq!(
            validate_engine(&input, &BUILT_IN_ENGINES, None).as_deref(),
            Some("“google.com” is already used by another search engine.")
        );
        assert_eq!(
            validate_engine(&input, &BUILT_IN_ENGINES, Some("google")),
            None
        );
    }
}
