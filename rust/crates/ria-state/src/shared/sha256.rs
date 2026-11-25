// Ported from ardia apps/browser/src/lib/sha256.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! SHA-256 of text and unpadded base64url, for the PKCE S256 challenge.
//!
//! Changes from Ardia:
//! - `sha256` uses the `sha2` crate instead of the hand-written rounds of `sha256.ts:12-56`
//!   (the `K` table, `rotr`, padding and the message schedule). Both are FIPS 180-4 SHA-256; the
//!   `sha256-rs` recorded cases (empty text, block boundaries, non-ASCII text) prove equal digests.
//! - Text is a `&str` (UTF-8). `TextEncoder` writes a lone surrogate as U+FFFD, which a Rust
//!   string cannot hold; the JS bridge makes the same replacement.
//! - The digest is a `[u8; 32]`, not a `Uint8Array`; `base64_url` takes a byte slice.

use sha2::{Digest, Sha256};

const BASE64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// `sha256` (`sha256.ts:12`): the SHA-256 digest of the UTF-8 bytes of `text`.
pub fn sha256(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

/// `base64Url` (`sha256.ts:60`): base64url without `=` padding.
pub fn base64_url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        // `Math.ceil(bits / 6)` of the bytes left, at most 4.
        for j in 0..(chunk.len() * 8).div_ceil(6) {
            out.push(char::from(BASE64URL[((n >> (18 - 6 * j)) & 63) as usize]));
        }
    }
    out
}
