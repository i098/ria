// Ported from ardia packages/core/src/providerNetwork.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Which provider endpoints Ria may call, and the approval key of a provider definition.

use super::provider_rows::{JsValue, provider_url};

const LOOPBACK_TEST_HOST: &str = "127.0.0.1";
const PRIVATE_SUFFIXES: [&str; 8] = [
    ".localhost",
    ".local",
    ".internal",
    ".lan",
    ".home.arpa",
    ".test",
    ".invalid",
    ".onion",
];

/// One label of `DNS_NAME` (`providerNetwork.ts:5`): 1 to 63 of `[a-z0-9-]`, not starting or
/// ending with `-`; the last label starts with a letter.
fn is_dns_label(label: &str, last: bool) -> bool {
    let b = label.as_bytes();
    let (Some(&first), Some(&end)) = (b.first(), b.last()) else {
        return false;
    };
    b.len() <= 63
        && if last {
            first.is_ascii_lowercase()
        } else {
            first.is_ascii_lowercase() || first.is_ascii_digit()
        }
        && (end.is_ascii_lowercase() || end.is_ascii_digit())
        && b.iter()
            .all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// Port of `isPublicHost` (`providerNetwork.ts:8`): a DNS name of two or more labels, not
/// `localhost` and not under a private suffix.
pub fn is_public_host(host: &str) -> bool {
    let h = host.to_lowercase();
    let name = h.strip_suffix('.').unwrap_or(&h);
    let labels: Vec<&str> = name.split('.').collect();
    let Some((last, rest)) = labels.split_last() else {
        return false;
    };
    if rest.is_empty() || !is_dns_label(last, true) || !rest.iter().all(|l| is_dns_label(l, false))
    {
        return false;
    }
    name != "localhost" && !PRIVATE_SUFFIXES.iter().any(|s| name.ends_with(s))
}

/// The scheme (lower case) and host of `plainAuthority` (`providerNetwork.ts:15`).
struct Authority<'a> {
    scheme: String,
    host: &'a str,
}

/// Port of `plainAuthority` (`providerNetwork.ts:15`): no userinfo, percent sign, IP literal,
/// default port or control character.
fn plain_authority(url: &str) -> Option<Authority<'_>> {
    if url.chars().any(|c| c <= ' ' || c == '\u{7f}' || c == '\\') {
        return None;
    }
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty() || !scheme.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let scheme = scheme.to_ascii_lowercase();
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    if authority.contains(['@', '%']) || authority.starts_with('[') {
        return None;
    }
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if let Some(port) = port {
        let valid = (1..=5).contains(&port.len())
            && !port.starts_with('0')
            && port.bytes().all(|b| b.is_ascii_digit())
            && port.parse::<u32>().is_ok_and(|p| p <= 65535);
        if !valid || port == if scheme == "https" { "443" } else { "80" } {
            return None;
        }
    }
    Some(Authority { scheme, host })
}

/// Port of `remoteRequestAllowed` (`providerNetwork.ts:31`): https to a public host, or, when
/// `allow_loopback`, the loopback test host.
pub fn remote_request_allowed(url: &str, allow_loopback: bool) -> bool {
    if provider_url(Some(url)).is_none() {
        return false;
    }
    let Some(parts) = plain_authority(url) else {
        return false;
    };
    if parts.scheme == "https" && is_public_host(parts.host) {
        return true;
    }
    allow_loopback && parts.host == LOOPBACK_TEST_HOST
}

/// JS `Number.prototype.toString()` of a finite number, as `JSON.stringify` prints it.
pub(crate) fn js_number_string(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    // `{:e}` gives the shortest round-trip digits, as JS does: `d.ddde±x`.
    let exp_form = format!("{:e}", n.abs());
    let Some((mantissa, exp)) = exp_form.split_once('e') else {
        return exp_form;
    };
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = digits.len() as i32;
    let point = exp.parse::<i32>().unwrap_or(0) + 1;
    let body = if k <= point && point <= 21 {
        format!("{digits}{}", "0".repeat((point - k) as usize))
    } else if 0 < point && point <= 21 {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let sign = if point > 0 { '+' } else { '-' };
        let rest = if k > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        format!("{}{rest}e{sign}{}", &digits[..1], (point - 1).abs())
    };
    if n < 0.0 { format!("-{body}") } else { body }
}

/// `JSON.stringify(text)`.
fn json_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Port of `canonical` (`providerNetwork.ts:40`): JSON with object keys sorted by UTF-16 code
/// units and `undefined` properties dropped; a lone `undefined` prints `null`.
fn canonical(value: &JsValue, out: &mut String) {
    match value {
        JsValue::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical(item, out);
            }
            out.push(']');
        }
        JsValue::Object(fields) => {
            let mut fields: Vec<_> = fields
                .iter()
                .filter(|(_, v)| *v != JsValue::Undefined)
                .collect();
            fields.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (key, v)) in fields.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json_string(key, out);
                out.push(':');
                canonical(v, out);
            }
            out.push('}');
        }
        JsValue::String(s) => json_string(s, out),
        JsValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsValue::Number(n) if n.is_finite() => out.push_str(&js_number_string(*n)),
        JsValue::Number(_) | JsValue::Null | JsValue::Undefined => out.push_str("null"),
    }
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Port of `sha256Hex` (`providerNetwork.ts:72`): SHA-256 of the UTF-8 bytes, in lower-case hex.
pub fn sha256_hex(text: &str) -> String {
    let mut bytes = text.as_bytes().to_vec();
    let bits = (bytes.len() as u64).wrapping_mul(8);
    bytes.push(0x80);
    while bytes.len() % 64 != 56 {
        bytes.push(0);
    }
    bytes.extend_from_slice(&bits.to_be_bytes());
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut w = [0u32; 64];
    for block in bytes.as_chunks::<64>().0 {
        for (slot, word) in w.iter_mut().zip(block.as_chunks::<4>().0) {
            *slot = u32::from_be_bytes(*word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut k] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = k
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let t2 = s0.wrapping_add((a & b) ^ (a & c) ^ (b & c));
            (k, g, f, e, d, c, b, a) = (g, f, e, d.wrapping_add(t1), c, b, a, t1.wrapping_add(t2));
        }
        for (slot, v) in h.iter_mut().zip([a, b, c, d, e, f, g, k]) {
            *slot = slot.wrapping_add(v);
        }
    }
    h.iter().map(|v| format!("{v:08x}")).collect()
}

/// Port of `providerApprovalKey` (`providerNetwork.ts:99`): the key a user's approval of a
/// provider definition is stored under; any change to the definition changes it.
pub fn provider_approval_key(definition: &JsValue) -> String {
    let mut text = String::new();
    canonical(definition, &mut text);
    format!("omnibox-provider:{}", sha256_hex(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_digests() {
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let long = "a".repeat(1000);
        assert_eq!(
            sha256_hex(&long),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    #[test]
    fn numbers_print_as_js() {
        for (n, s) in [
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (1e-7, "1e-7"),
            (1.5e-6, "0.0000015"),
            (-0.1, "-0.1"),
            (123.456, "123.456"),
            (5e-324, "5e-324"),
        ] {
            assert_eq!(js_number_string(n), s);
        }
    }

    #[test]
    fn canonical_sorts_by_utf16_and_drops_undefined() {
        let v = JsValue::Object(vec![
            ("\u{ff61}".into(), JsValue::Number(1.0)),
            ("😀".into(), JsValue::Null),
            ("a".into(), JsValue::Undefined),
            (
                "B".into(),
                JsValue::Array(vec![JsValue::Undefined, JsValue::String("\u{1}\"".into())]),
            ),
        ]);
        let mut out = String::new();
        canonical(&v, &mut out);
        assert_eq!(
            out,
            "{\"B\":[null,\"\\u0001\\\"\"],\"😀\":null,\"\u{ff61}\":1}"
        );
    }
}
