// Ria original, no Ardia source.

//! Base64 helpers for extension packages.

use std::borrow::Cow;

use base64::Engine as _;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, STANDARD};

pub fn base64_encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

/// Decodes Base64 with padding rules, ignoring bytes outside the alphabet and padding.
pub fn base64_decode_lenient(text: &str) -> Option<Vec<u8>> {
    const DECODER: GeneralPurpose = GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true),
    );
    let known = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=');
    let bytes = if text.bytes().all(known) {
        Cow::Borrowed(text.as_bytes())
    } else {
        Cow::Owned(text.bytes().filter(|b| known(*b)).collect())
    };
    DECODER.decode(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors_and_ignores_whitespace() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64_encode(plain.as_bytes()), encoded);
            assert_eq!(base64_decode_lenient(encoded).unwrap(), plain.as_bytes());
        }
        assert_eq!(base64_decode_lenient("Zm\n8=").unwrap(), b"fo");
    }

    #[test]
    fn base64_rejects_incomplete_groups_and_bad_padding() {
        for invalid in [
            "Z", "Zg", "Zg=", "Zg===", "Zm8", "Zm=8", "Zm9v=", "Zg==Zg==",
        ] {
            assert_eq!(base64_decode_lenient(invalid), None, "{invalid}");
        }
    }
}
