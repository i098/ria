// Ported from ardia apps/browser/src/lib/tileLetter.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The letter on a page tile that has no icon: the first letter of the site name, else of the title.
//!
//! Changes from Ardia: `toLocaleUpperCase` is the locale-free `char::to_uppercase`.

use regex::Regex;
use std::sync::LazyLock;

use super::url::display_host_of;

/// `LETTER` (`tileLetter.ts:3`).
static LETTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{L}$").expect("static pattern"));

fn is_letter(c: char) -> bool {
    LETTER.is_match(c.encode_utf8(&mut [0; 4]))
}

/// `firstLetter` (`tileLetter.ts:5`): the first letter, in upper case.
pub fn first_letter(text: &str) -> Option<String> {
    text.chars()
        .find(|&c| is_letter(c))
        .map(|c| c.to_uppercase().collect())
}

/// `SECOND_LEVEL` (`tileLetter.ts:10`).
const SECOND_LEVEL: [&str; 10] = [
    "co", "com", "net", "org", "gov", "edu", "ac", "ne", "or", "go",
];

/// `siteName` (`tileLetter.ts:12`): the label before the public suffix, like `bbc` in `www.bbc.co.uk`.
pub fn site_name(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').filter(|l| !l.is_empty()).collect();
    if labels.len() < 2 {
        return host.to_string();
    }
    let suffix = if labels.len() > 2 && SECOND_LEVEL.contains(&labels[labels.len() - 2]) {
        2
    } else {
        1
    };
    labels[labels.len() - 1 - suffix].to_string()
}

/// `tileLetterOf` (`tileLetter.ts:19`): the site letter, else the title letter, else `•`.
pub fn tile_letter_of(url: &str, title: &str) -> String {
    let host = if url.is_empty() {
        String::new()
    } else {
        site_name(&display_host_of(url))
    };
    let from_host = if host.chars().next().is_some_and(is_letter) {
        first_letter(&host)
    } else {
        None
    };
    from_host
        .or_else(|| first_letter(title))
        .unwrap_or_else(|| "•".to_string())
}
