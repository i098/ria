// Ported from ardia apps/browser/src/lib/linkWrappers.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Unwraps redirect links of Google, Slack, Outlook, Facebook, YouTube and others to the page they lead to.
//!
//! Changes from Ardia: the `i` flag of the original matches only ASCII letters here, so the
//! patterns spell out both cases; `\d` is `[0-9]`.

use regex::Regex;
use std::sync::LazyLock;

use super::js::js_trim;
use super::url::decode_uri_component;

/// `LinkWrapper` (`linkWrappers.ts:1`).
struct LinkWrapper {
    host: Regex,
    path: Regex,
    params: &'static [&'static str],
}

/// `LINK_WRAPPERS` (`linkWrappers.ts:3`).
static LINK_WRAPPERS: LazyLock<Vec<LinkWrapper>> = LazyLock::new(|| {
    [
        (
            r"^(www\.)?google\.[a-z]{2,3}(\.[a-z]{2})?$",
            r"^/url$",
            &["q", "url"][..],
        ),
        (r"^slack-redir\.net$", r"^/link$", &["url"]),
        (r"\.safelinks\.protection\.outlook\.com$", r"^/?$", &["url"]),
        (
            r"^(l|lm)\.facebook\.com$|^l\.messenger\.com$",
            r"^/l\.php$",
            &["u"],
        ),
        (r"^l\.instagram\.com$|^l\.threads\.com$", r"^/?$", &["u"]),
        (r"^(www\.|m\.)?youtube\.com$", r"^/redirect$", &["q"]),
        (r"^out\.reddit\.com$", r"^/", &["url"]),
        (r"^(www\.)?linkedin\.com$", r"^/redir/redirect/?$", &["url"]),
        (r"^steamcommunity\.com$", r"^/linkfilter/?$", &["url", "u"]),
        (r"^away\.vk\.com$|^vk\.com$", r"^/away\.php$", &["to"]),
        (r"^t\.umblr\.com$", r"^/redirect$", &["z"]),
    ]
    .into_iter()
    .map(|(host, path, params)| LinkWrapper {
        host: Regex::new(host).expect("static pattern"),
        path: Regex::new(path).expect("static pattern"),
        params,
    })
    .collect()
});

static URL_PARTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^[hH][tT][tT][pP][sS]?://(?:[^@/?#]*@)?([^/?#:]+)(?::[0-9]+)?([^?#]*)(?:\?([^#]*))?",
    )
    .expect("static pattern")
});

/// `MAX_UNWRAPS` (`linkWrappers.ts:18`).
const MAX_UNWRAPS: usize = 3;

fn is_web(text: &str) -> bool {
    ["http://", "https://"].iter().any(|p| {
        text.get(..p.len())
            .is_some_and(|h| h.eq_ignore_ascii_case(p))
    })
}

/// `queryParam` (`linkWrappers.ts:20`): the decoded value of the first `name=` pair; `None` when it
/// does not decode.
pub fn query_param(query: &str, name: &str) -> Option<String> {
    let pair = query
        .split('&')
        .find(|pair| pair.split_once('=').is_some_and(|(key, _)| key == name))?;
    decode_uri_component(&pair[name.len() + 1..].replace('+', " "))
}

/// `unwrapOnce` (`linkWrappers.ts:33`).
pub fn unwrap_once(url: &str) -> Option<String> {
    let m = URL_PARTS.captures(url)?;
    let host = m[1].to_lowercase();
    let path = match &m[2] {
        "" => "/",
        p => p,
    };
    let wrapper = LINK_WRAPPERS
        .iter()
        .find(|w| w.host.is_match(&host) && w.path.is_match(path))?;
    let query = m.get(3).map_or("", |q| q.as_str());
    wrapper.params.iter().find_map(|name| {
        let target = query_param(query, name)?;
        let target = js_trim(&target);
        (!target.is_empty() && is_web(target)).then(|| target.to_string())
    })
}

/// `linkDestination` (`linkWrappers.ts:47`): the URL with up to 3 redirect wrappers removed.
pub fn link_destination(url: &str) -> String {
    let mut at = url.to_string();
    for _ in 0..MAX_UNWRAPS {
        match unwrap_once(&at) {
            Some(next) => at = next,
            None => break,
        }
    }
    at
}
