// Ria original, no Ardia source.

//! URL units of `packages/core`: app URLs, tracker removal, IDN display, page text and
//! matching, text fragments, web store ids and the public suffix TLD list.

pub mod app_urls;
pub mod clean_url;
pub mod idn;
pub mod page_match;
pub mod page_text;
pub mod text_fragment;
pub mod tlds;
pub mod web_store;

/// UTF-16 length of `s`: the JS `string.length`.
pub(crate) fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// `s` without its first `prefix` (ASCII, compared without case), as `/^prefix/i` does.
pub(crate) fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

/// JS `/[\n\r\u2028\u2029]/`: the characters that `.` does not match.
pub(crate) fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}
