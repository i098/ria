// Ported from ardia packages/core/src/quickCreate.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! "new doc", "create sheet": shortcuts that open a blank document of a web app.

use crate::fuzzy::is_js_space;

/// One quick-create target.
#[derive(Debug, PartialEq, Eq)]
pub struct QuickCreate {
    pub id: &'static str,
    pub title: &'static str,
    pub url: &'static str,
    pub aliases: &'static [&'static str],
    pub site: &'static str,
}

const fn row(
    id: &'static str,
    title: &'static str,
    url: &'static str,
    aliases: &'static [&'static str],
    site: &'static str,
) -> QuickCreate {
    QuickCreate {
        id,
        title,
        url,
        aliases,
        site,
    }
}

pub static QUICK_CREATE: [QuickCreate; 17] = [
    row(
        "doc",
        "New Google Doc",
        "https://docs.new",
        &["doc", "docs", "document", "google doc", "gdoc"],
        "docs.google.com",
    ),
    row(
        "sheet",
        "New Google Sheet",
        "https://sheets.new",
        &["sheet", "sheets", "spreadsheet", "google sheet"],
        "sheets.google.com",
    ),
    row(
        "slides",
        "New Google Slides",
        "https://slides.new",
        &["slides", "slide", "presentation", "deck", "google slides"],
        "slides.google.com",
    ),
    row(
        "form",
        "New Google Form",
        "https://forms.new",
        &["form", "forms", "survey", "google form"],
        "forms.google.com",
    ),
    row(
        "meeting",
        "New Google Calendar Event",
        "https://meeting.new",
        &["meeting", "event", "calendar", "invite"],
        "calendar.google.com",
    ),
    row(
        "meet",
        "New Google Meet",
        "https://meet.new",
        &["meet", "call", "video call", "google meet"],
        "meet.google.com",
    ),
    row(
        "notion",
        "New Notion Page",
        "https://notion.new",
        &["notion", "note", "page", "notion page"],
        "notion.so",
    ),
    row(
        "linear",
        "New Linear Issue",
        "https://linear.new",
        &["linear", "issue", "ticket", "linear issue"],
        "linear.app",
    ),
    row(
        "jira",
        "New Jira Issue",
        "https://jira.new",
        &["jira", "ticket", "issue", "jira issue"],
        "atlassian.com",
    ),
    row(
        "confluence",
        "New Confluence Page",
        "https://confluence.new",
        &["wiki", "confluence", "confluence page"],
        "atlassian.com",
    ),
    row(
        "gist",
        "New GitHub Gist",
        "https://gist.new",
        &["gist", "snippet", "github gist"],
        "github.com",
    ),
    row(
        "repo",
        "New GitHub Repository",
        "https://repo.new",
        &["repo", "repository", "github repo"],
        "github.com",
    ),
    row(
        "figma",
        "New Figma Design",
        "https://figma.new",
        &["figma", "design", "figma design"],
        "figma.com",
    ),
    row(
        "figjam",
        "New FigJam Board",
        "https://figjam.new",
        &["figjam", "whiteboard", "board"],
        "figma.com",
    ),
    row(
        "word",
        "New Word Document",
        "https://word.new",
        &["word", "word document"],
        "word.cloud.microsoft",
    ),
    row(
        "excel",
        "New Excel Workbook",
        "https://excel.new",
        &["excel", "workbook"],
        "excel.cloud.microsoft",
    ),
    row(
        "powerpoint",
        "New PowerPoint Presentation",
        "https://powerpoint.new",
        &["powerpoint", "ppt"],
        "powerpoint.cloud.microsoft",
    ),
];

/// The text after `new` or `create` (any case) and a run of whitespace, or `None` when the query
/// is not of the form `/^(?:new|create)(\s+(.*))?$/i` with a non-empty whitespace run.
fn rest_after_verb(query: &str) -> Option<&str> {
    let query = query.trim_start_matches(is_js_space);
    let after = [("new", 3), ("create", 6)]
        .iter()
        .find_map(|&(verb, len)| {
            query
                .get(..len)
                .filter(|head| head.eq_ignore_ascii_case(verb))
                .map(|_| &query[len..])
        })?;
    let rest = after.trim_start_matches(is_js_space);
    // `.` does not match a line terminator, and `$` is the end of the text.
    (rest.len() < after.len() && !rest.contains(['\n', '\r', '\u{2028}', '\u{2029}']))
        .then_some(rest)
}

/// Port of `matchQuickCreate` (`packages/core/src/quickCreate.ts:29`).
pub fn match_quick_create(query: &str, limit: usize) -> Vec<&'static QuickCreate> {
    let Some(rest) = rest_after_verb(query) else {
        return Vec::new();
    };
    let rest = rest
        .to_lowercase()
        .split(is_js_space)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if rest.is_empty() {
        return QUICK_CREATE.iter().take(limit).collect();
    }
    let rank = |c: &QuickCreate| {
        if c.aliases.contains(&rest.as_str()) {
            Some(0)
        } else {
            c.aliases.iter().any(|a| a.starts_with(&rest)).then_some(1)
        }
    };
    let mut ranked: Vec<_> = QUICK_CREATE
        .iter()
        .filter_map(|c| rank(c).map(|r| (r, c)))
        .collect();
    ranked.sort_by_key(|&(r, _)| r);
    ranked.into_iter().take(limit).map(|(_, c)| c).collect()
}
