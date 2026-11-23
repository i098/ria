// Ria original, no Ardia source.

//! JavaScript string rules that Rust's `str` does not share: the `\s` and `trim` set, and
//! `encodeURIComponent`. Rust strings hold no lone surrogates, so callers see them as U+FFFD.

/// The characters of the JS `\s` class and `String.prototype.trim`.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `text.trim()`.
pub fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

/// The characters that `.` does not match in a JS regex without the `s` flag.
pub fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `encodeURIComponent(text)`.
pub fn encode_uri_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_like_encode_uri_component() {
        assert_eq!(encode_uri_component("a b/é!~*'()"), "a%20b%2F%C3%A9!~*'()");
    }

    #[test]
    fn trims_the_js_set() {
        assert_eq!(js_trim("\u{feff} x\u{85}\u{a0}"), "x\u{85}");
    }
}
