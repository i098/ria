// Ported from ardia packages/core/src/pageText.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Lowercased, word-split text of a page row (URL and title) for omnibox matching.
//!
//! The original calls `hostOf` and `urlForDisplay` of `omnibox.ts`; here the caller passes
//! the results (`shown`) or the functions, so this module does not depend on the omnibox.
//! The original caches a `PageText` for each row object in a `WeakMap`; the caller of this
//! port keeps the value with the row instead.

use super::is_line_terminator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    pub url: String,
    pub key: String,
    pub shown: String,
    pub shown_lower: String,
    pub title_lower: String,
    pub title_words: String,
    pub shown_words: Option<String>,
    pub shown_blank: Option<bool>,
    pub host: Option<String>,
    /// `None`: not computed. `Some(None)`: computed, the URL has no http(s) origin.
    pub origin: Option<Option<String>>,
    pub host_lower: Option<String>,
}

/// Port of `displayUrl` (`pageText.ts:4`); `url_for_display` is `urlForDisplay` of `omnibox.ts`.
pub fn display_url(url: &str, url_for_display: impl FnOnce(&str) -> String) -> String {
    let shown = url_for_display(url);
    shown.strip_suffix('/').unwrap_or(&shown).to_string()
}

/// Port of `pageKey` (`pageText.ts:8`): `url.replace(/#.*$/, "").replace(/\/$/, "")`.
pub fn page_key(url: &str) -> String {
    // `#.*$` needs a `#` after the last line terminator (`.` does not match one).
    let from = url
        .rfind(is_line_terminator)
        .map_or(0, |i| i + url[i..].chars().next().map_or(1, char::len_utf8));
    let cut = url[from..].find('#').map_or(url, |i| &url[..from + i]);
    cut.strip_suffix('/').unwrap_or(cut).to_string()
}

/// Port of `withRootSlash` (`pageText.ts:10`): adds `/` after the host of an http(s) URL.
pub fn with_root_slash(url: &str) -> String {
    let scheme = ["https://", "http://"].iter().find_map(|s| {
        url.get(..s.len())
            .filter(|h| h.eq_ignore_ascii_case(s))
            .map(|_| s.len())
    });
    let Some(scheme) = scheme else {
        return url.to_string();
    };
    let host_end = scheme
        + url[scheme..]
            .find(['/', '?', '#'])
            .unwrap_or(url.len() - scheme);
    if url[host_end..].starts_with('/') {
        return url.to_string();
    }
    format!("{}/{}", &url[..host_end], &url[host_end..])
}

/// Port of `logVisits` (`pageText.ts:29`); the original memoizes the same value.
pub fn log_visits(visits: f64) -> f64 {
    (1.0 + visits).log2() * 8.0
}

/// `wordsOf`: ` lower` with each run of characters outside `[a-z0-9]` as one space.
pub fn words_of(lower: &str) -> String {
    let mut out = String::with_capacity(lower.len() + 1);
    let mut in_run = true; // the leading space is a run of its own
    out.push(' ');
    for c in lower.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            in_run = false;
            out.push(c);
        } else if !in_run {
            in_run = true;
            out.push(' ');
        }
    }
    out
}

/// Port of `hostPart` (`pageText.ts:64`): `/^[^/?#]*/.exec(shown)[0]`.
pub fn host_part(shown: &str) -> &str {
    &shown[..shown.find(['/', '?', '#']).unwrap_or(shown.len())]
}

impl PageText {
    /// Port of `pageText` (`pageText.ts:36`); `shown` is `displayUrl(url)`.
    pub fn new(url: &str, title: &str, shown: String) -> Self {
        let shown_lower = shown.to_lowercase();
        let title_lower = title.to_lowercase();
        Self {
            url: url.to_string(),
            key: page_key(url),
            shown,
            shown_lower,
            title_words: words_of(&title_lower),
            title_lower,
            shown_words: None,
            shown_blank: None,
            host: None,
            origin: None,
            host_lower: None,
        }
    }

    /// Port of `shownWordsOf` (`pageText.ts:34`).
    pub fn shown_words(&mut self) -> &str {
        let lower = &self.shown_lower;
        self.shown_words.get_or_insert_with(|| words_of(lower))
    }

    /// Port of `tabPageText` (`pageText.ts:50`) for a row with a custom title; `text` is
    /// `PageText::new` of the row and `listed` the text of the listed row, if any.
    pub fn renamed(text: &PageText, custom_title: &str, listed: Option<&PageText>) -> PageText {
        let mut titles = vec![custom_title.to_lowercase(), text.title_lower.clone()];
        if let Some(l) = listed.filter(|l| l.title_lower != text.title_lower) {
            titles.push(l.title_lower.clone());
        }
        let title_lower = titles.join("\n");
        PageText {
            title_words: words_of(&title_lower),
            title_lower,
            ..listed.unwrap_or(text).clone()
        }
    }

    /// Port of `hostOfPage` (`pageText.ts:63`); `host_of` is `hostOf` of `omnibox.ts`.
    pub fn host_of_page(&mut self, host_of: impl FnOnce(&str) -> String) -> &str {
        let url = &self.url;
        self.host.get_or_insert_with(|| host_of(url))
    }

    /// Port of `hostLowerOfPage` (`pageText.ts:65`).
    pub fn host_lower_of_page(&mut self) -> &str {
        let shown_lower = &self.shown_lower;
        self.host_lower
            .get_or_insert_with(|| host_part(shown_lower).to_string())
    }

    /// Port of `originOfPage` (`pageText.ts:66`): `/^https?:\/\/[^/?#]+/i.exec(url)?.[0]`.
    pub fn origin_of_page(&mut self) -> Option<&str> {
        let url = &self.url;
        self.origin
            .get_or_insert_with(|| {
                let scheme = ["https://", "http://"].iter().find_map(|s| {
                    url.get(..s.len())
                        .filter(|h| h.eq_ignore_ascii_case(s))
                        .map(|_| s.len())
                })?;
                let len = url[scheme..]
                    .find(['/', '?', '#'])
                    .unwrap_or(url.len() - scheme);
                (len > 0).then(|| url[..scheme + len].to_string())
            })
            .as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renamed_title_joins_custom_text_and_listed_title() {
        let text = PageText::new("https://a.test/", "Alpha", "a.test".into());
        let listed = PageText::new("https://a.test/", "Beta", "a.test".into());
        let renamed = PageText::renamed(&text, "Mine", Some(&listed));
        assert_eq!(renamed.title_lower, "mine\nalpha\nbeta");
        assert_eq!(renamed.title_words, " mine alpha beta");
        assert_eq!(
            PageText::renamed(&text, "Mine", None).title_lower,
            "mine\nalpha"
        );
    }

    #[test]
    fn words_split_on_runs_and_page_key_cuts_fragment_after_last_line_break() {
        assert_eq!(words_of("a.b  c"), " a b c");
        assert_eq!(words_of("..a"), " a");
        assert_eq!(page_key("https://a/#x\ny#z/"), "https://a/#x\ny");
        assert_eq!(page_key("https://a/b/#frag"), "https://a/b");
    }
}
