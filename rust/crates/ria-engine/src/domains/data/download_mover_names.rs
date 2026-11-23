// Ported from ardia packages/ardiacore/ios/ArdiaDownloadMover.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Safe file names for the download mover: clean name, collision suffix, byte limit.

use unicode_segmentation::UnicodeSegmentation;

const DEFAULT_NAME: &str = "download";
const MAX_NAME_BYTES: usize = 255;
const MAX_KEPT_EXTENSION: usize = 24;

fn truncated(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let end = text
        .grapheme_indices(true)
        .take_while(|(start, grapheme)| start + grapheme.len() <= max_bytes)
        .last()
        .map_or(0, |(start, grapheme)| start + grapheme.len());
    &text[..end]
}

/// `pathExtension` and `stringByDeletingPathExtension` of a single name.
fn path_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(dot) if dot > 0 => (&name[..dot], &name[dot + 1..]),
        _ => (name, ""),
    }
}

fn split_name(name: &str) -> (String, String) {
    let (mut base, ext) = path_extension(name);
    let mut ext = ext.to_owned();
    if !ext.is_empty() {
        let (inner, inner_ext) = path_extension(base);
        if inner_ext.to_lowercase() == "tar" {
            ext = format!("tar.{ext}");
            base = inner;
        }
    }
    if base.is_empty() || ext.encode_utf16().count() > MAX_KEPT_EXTENSION {
        return (name.to_owned(), String::new());
    }
    (base.to_owned(), ext)
}

/// Puts `suffix` between the stem and the extension, and cuts the stem so the name fits in 255 bytes.
pub fn name_with_suffix(name: &str, suffix: &str) -> String {
    let (stem, ext) = split_name(name);
    let tail = if ext.is_empty() {
        suffix.to_owned()
    } else {
        format!("{suffix}.{ext}")
    };
    let room = MAX_NAME_BYTES - tail.len().min(MAX_NAME_BYTES - 1);
    let stem = truncated(&stem, room);
    let stem = if stem.is_empty() { DEFAULT_NAME } else { stem };
    format!("{stem}{tail}")
}

/// A file name with no control, bidi or path characters, no leading dots and at most 255 bytes.
pub fn safe_file_name(name: &str) -> String {
    let mut clean = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_control()
            || matches!(
                ch,
                '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}'
                    | '\u{2066}'..='\u{2069}' | '\u{FEFF}'
            )
        {
            continue;
        }
        clean.push(if matches!(ch, '/' | ':') { '_' } else { ch });
    }
    let mut trimmed = clean.trim_matches(char::is_whitespace);
    while let Some(rest) = trimmed.strip_prefix('.') {
        trimmed = rest.trim_matches(char::is_whitespace);
    }
    let trimmed = if trimmed.is_empty() {
        DEFAULT_NAME
    } else {
        trimmed
    };
    if trimmed.len() <= MAX_NAME_BYTES {
        return trimmed.to_owned();
    }
    name_with_suffix(trimmed, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_file_names() {
        assert_eq!(safe_file_name(".hidden"), "hidden");
        assert_eq!(safe_file_name("a/b:c.txt"), "a_b_c.txt");
        assert_eq!(safe_file_name("in\u{202E}gnp.exe"), "ingnp.exe");
        assert_eq!(safe_file_name("tab\there\n.txt"), "tabhere.txt");
        assert_eq!(
            safe_file_name("a\u{200D}:\u{200D}\u{202E}\u{061C}b\u{200D}\n\u{0085}.txt"),
            "a\u{200D}_\u{200D}b\u{200D}.txt"
        );
        assert_eq!(safe_file_name(" .. "), "download");
        let long_name = "é".repeat(400) + ".pdf";
        let cut = safe_file_name(&long_name);
        assert!(cut.len() <= 255);
        assert!(cut.ends_with(".pdf"));
    }

    #[test]
    fn suffixes() {
        assert_eq!(name_with_suffix("report.pdf", " (1)"), "report (1).pdf");
        assert_eq!(name_with_suffix("site.tar.gz", " (1)"), "site (1).tar.gz");
        assert_eq!(name_with_suffix("README", " (2)"), "README (2)");
        assert_eq!(name_with_suffix("e\u{301}.txt", ""), "e\u{301}.txt");
        assert_eq!(truncated("ae\u{301}", 2), "a");
    }

    #[test]
    fn truncation_keeps_flags_whole_before_and_after_collisions() {
        let stem = "a".repeat(244);
        let name = format!("{stem}🇨🇦.x");
        assert_eq!(safe_file_name(&name), name);
        assert_eq!(name_with_suffix(&name, " (1)"), format!("{stem} (1).x"));

        let stem = "a".repeat(247);
        let name = format!("{stem}🇨🇦.x");
        assert_eq!(safe_file_name(&name), format!("{stem}.x"));
        assert_eq!(
            safe_file_name(&format!("e{}.x", "\u{301}".repeat(128))),
            "download.x"
        );
    }
}
