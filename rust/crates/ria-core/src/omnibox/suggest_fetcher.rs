// Ported from ardia packages/core/src/suggestFetcher.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Remote search suggestions as the user types: one request at a time, after a short delay,
//! with a small cache of replies by URL.
//!
//! The original owns a timer, a `fetch` function, an `AbortController` and a callback. This port
//! has no I/O: [`SuggestFetcher::request`] says what to do ([`Next`]), and the caller waits
//! [`DEFAULT_DELAY_MS`], fetches, and gives the reply to [`SuggestFetcher::finish`]. A
//! [`Ticket`] is current until the next `request` or `cancel`; the caller then clears its timer
//! and aborts its fetch, as the original's `cancel` does.

use std::collections::VecDeque;

use crate::search::bangs::{Json, parse_json};
use crate::search::engines::{
    RemoteSuggestions, SearchEngine, SuggestReply, parse_remote_suggestions, strip_reply_prefix,
    suggest_request_url,
};

/// The default delay before a request, in milliseconds.
pub const DEFAULT_DELAY_MS: u32 = 120;
/// The default number of cached replies.
pub const DEFAULT_CACHE_SIZE: usize = 64;

/// One request: the URL to fetch, valid while no later request or cancel came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    generation: u64,
    pub url: String,
}

/// What the caller does after [`SuggestFetcher::request`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The engine asks nothing for this query.
    Nothing,
    /// A cached reply: deliver it on the next turn of the event loop if the ticket is current.
    Cached(Ticket, RemoteSuggestions),
    /// Wait the delay, then fetch `ticket.url` with the engine, and call `finish`.
    Fetch(Ticket),
}

/// Port of `createSuggestFetcher` (`suggestFetcher.ts:11`) without its I/O.
#[derive(Debug, Clone)]
pub struct SuggestFetcher {
    cache: VecDeque<(String, RemoteSuggestions)>,
    cache_size: usize,
    generation: u64,
}

impl Default for SuggestFetcher {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_SIZE)
    }
}

impl SuggestFetcher {
    pub fn new(cache_size: usize) -> Self {
        Self {
            cache: VecDeque::new(),
            cache_size,
            generation: 0,
        }
    }

    /// `cancel`: no ticket given out so far is current.
    pub fn cancel(&mut self) {
        self.generation += 1;
    }

    /// `request`: cancels, then gives the cached reply of the suggest URL, or a fetch to make.
    pub fn request(&mut self, engine: &SearchEngine, query: &str) -> Next {
        self.cancel();
        let Some(url) = suggest_request_url(engine, query) else {
            return Next::Nothing;
        };
        let cached = self
            .cache
            .iter()
            .find(|(u, _)| *u == url)
            .map(|(_, s)| s.clone());
        let ticket = Ticket {
            generation: self.generation,
            url,
        };
        match cached {
            Some(s) => Next::Cached(ticket, s),
            None => Next::Fetch(ticket),
        }
    }

    /// Whether no request or cancel came after `ticket`.
    pub fn is_current(&self, ticket: &Ticket) -> bool {
        ticket.generation == self.generation
    }

    /// The fetch of `ticket` answered: `reply` is the body of an ok response, `None` for a
    /// response that is not ok or whose body failed to read. A current ticket caches and returns
    /// the suggestions (none for a bad reply); a stale one was aborted and gives `None`. A fetch
    /// that failed outright needs no call.
    pub fn finish(&mut self, ticket: &Ticket, reply: Option<&str>) -> Option<RemoteSuggestions> {
        if !self.is_current(ticket) {
            return None;
        }
        let suggestions = reply.map(reply_suggestions).unwrap_or_default();
        self.remember(&ticket.url, suggestions.clone());
        Some(suggestions)
    }

    fn remember(&mut self, url: &str, suggestions: RemoteSuggestions) {
        self.cache.retain(|(u, _)| u != url);
        self.cache.push_back((url.to_string(), suggestions));
        if self.cache.len() > self.cache_size {
            self.cache.pop_front();
        }
    }
}

/// `parseRemoteSuggestions(suggestReplyJson(text))`, with no suggestions when the text is not
/// JSON.
pub fn reply_suggestions(text: &str) -> RemoteSuggestions {
    parse_json(strip_reply_prefix(text)).map_or_else(
        |_| RemoteSuggestions::default(),
        |body| parse_remote_suggestions(&suggest_reply(&body)),
    )
}

/// The parts of a parsed reply `parseRemoteSuggestions` reads (`engines.ts:145`).
fn suggest_reply(body: &Json) -> SuggestReply<'_> {
    let Json::Array(body) = body else {
        return SuggestReply::default();
    };
    let Some(Json::Array(items)) = body.get(1) else {
        return SuggestReply::default();
    };
    let titles = match body.get(2) {
        Some(Json::Array(t)) => strings(t),
        _ => Vec::new(),
    };
    let types = match body.get(4).and_then(|x| x.get("google:suggesttype")) {
        Some(Json::Array(t)) => t.iter().map(js_string).collect(),
        _ => Vec::new(),
    };
    SuggestReply {
        suggestions: strings(items),
        titles,
        types,
    }
}

/// Each item when it is a string.
fn strings(items: &[Json]) -> Vec<Option<&str>> {
    items
        .iter()
        .map(|v| match v {
            Json::String(s) => Some(s.as_str()),
            _ => None,
        })
        .collect()
}

/// `String(value)` as far as comparing it with a suggest type needs: a number prints as some
/// digits, which never equal a type name.
fn js_string(value: &Json) -> String {
    match value {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => n.to_string(),
        Json::String(s) => s.clone(),
        Json::Array(items) => items
            .iter()
            .map(|v| match v {
                Json::Null => String::new(),
                v => js_string(v),
            })
            .collect::<Vec<_>>()
            .join(","),
        Json::Object(_) => "[object Object]".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> SearchEngine {
        SearchEngine {
            id: "g".into(),
            name: "G".into(),
            keyword: "g".into(),
            url: "https://g.test/?q=%s".into(),
            suggest_url: Some("https://g.test/s?q=%s".into()),
            custom: false,
            extension: None,
        }
    }

    #[test]
    fn fetches_once_then_answers_from_the_cache() {
        let mut f = SuggestFetcher::new(1);
        let Next::Fetch(ticket) = f.request(&engine(), "a") else {
            panic!("expected a fetch");
        };
        let reply = r#")]}' ["a",["ab"," "],[],[],{"google:suggesttype":[["QUERY"]]}]"#;
        let got = f.finish(&ticket, Some(reply)).expect("current");
        assert_eq!(got.queries, ["ab"]);
        assert!(matches!(f.request(&engine(), "a"), Next::Cached(_, s) if s == got));
        assert_eq!(f.request(&engine(), " "), Next::Nothing);
        let Next::Fetch(stale) = f.request(&engine(), "b") else {
            panic!("expected a fetch");
        };
        f.cancel();
        assert_eq!(f.finish(&stale, Some("[]")), None);
    }

    #[test]
    fn navigation_types_read_through_arrays() {
        let body = parse_json(
            r#"["q",["https://a.test"],["A"],[],{"google:suggesttype":[["NAVIGATION"]]}]"#,
        )
        .expect("json");
        let got = parse_remote_suggestions(&suggest_reply(&body));
        assert_eq!(got.sites.len(), 1);
        assert_eq!(js_string(&Json::Object(vec![])), "[object Object]");
        assert_eq!(reply_suggestions("{"), RemoteSuggestions::default());
    }
}
