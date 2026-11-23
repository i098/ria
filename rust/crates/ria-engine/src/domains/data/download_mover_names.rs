// Ported from ardia packages/ardiacore/ios/ArdiaDownloadMover.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Safe file names for the download mover: clean name, collision suffix, byte limit.

const MAX_NAME_BYTES: usize = 255;
const MAX_KEPT_EXTENSION: usize = 24;

/// True when `c` extends the composed character sequence before it.
// shortcut: covers combining marks, variation selectors, emoji modifiers and ZWJ only, not full UAX #29; upgrade with a segmentation crate if names need it.
fn extends(c: char) -> bool {
    matches!(c as u32,
        0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x0610..=0x061A | 0x064B..=0x065F
        | 0x0900..=0x0903 | 0x093A..=0x094F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x200C | 0x200D
        | 0x20D0..=0x20FF | 0x302A..=0x302F | 0x3099 | 0x309A | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F
        | 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F | 0xE0100..=0xE01EF)
}

/// Splits `text` into composed character sequences, as `NSStringEnumerationByComposedCharacterSequences`.
fn clusters(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut previous: Option<char> = None;
    for (index, c) in text.char_indices() {
        let joins = match previous {
            None => false,
            Some('\r') => c == '\n',
            Some('\u{200D}') => true,
            Some(p) => (p as u32) >= 0x20 && p != '\u{7F}' && extends(c),
        };
        if !joins && index > 0 {
            out.push(&text[start..index]);
            start = index;
        }
        previous = Some(c);
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

fn truncated(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_owned();
    }
    let mut kept = String::new();
    for piece in clusters(text) {
        if kept.len() + piece.len() > max_bytes {
            break;
        }
        kept.push_str(piece);
    }
    kept
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
    truncated(&stem, room) + &tail
}

/// A file name with no control, bidi or path characters, no leading dots and at most 255 bytes.
pub fn safe_file_name(name: &str) -> String {
    let mut clean = String::new();
    for piece in clusters(name) {
        let c = piece.chars().next().map_or(0, |c| c as u32);
        if c < 0x20
            || c == 0x7F
            || (0x202A..=0x202E).contains(&c)
            || (0x2066..=0x2069).contains(&c)
            || c == 0x200E
            || c == 0x200F
            || c == 0xFEFF
        {
            continue;
        }
        clean.push_str(if c == '/' as u32 || c == ':' as u32 {
            "_"
        } else {
            piece
        });
    }
    let mut trimmed = clean.trim_matches(char::is_whitespace);
    while let Some(rest) = trimmed.strip_prefix('.') {
        trimmed = rest.trim_matches(char::is_whitespace);
    }
    let trimmed = if trimmed.is_empty() {
        "download"
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
}
