// Ported from ardia apps/browser/src/lib/versions.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Semantic version order: `major.minor.patch[-pre]`.
//!
//! Changes from Ardia: none. The result is an `f64` because the TS result can be `NaN` (a part that is not
//! a number); prerelease parts compare by UTF-16 code units, as JS `<` does.

use super::js::js_number;

/// `compareVersions` (`versions.ts:1`): below 0 when `a` comes first, 0 when equal, above 0 when `b` comes first.
pub fn compare_versions(a: &str, b: &str) -> f64 {
    // `v.split(/-(.*)/s, 2)`: the text before the first `-`, and all the text after it.
    let parse = |v: &str| {
        let (core, pre) = v.split_once('-').unwrap_or((v, ""));
        (
            core.split('.').map(js_number).collect::<Vec<_>>(),
            pre.to_owned(),
        )
    };
    let (x, x_pre) = parse(a);
    let (y, y_pre) = parse(b);
    for i in 0..3 {
        let (m, n) = (x.get(i), y.get(i));
        // `undefined !== undefined` is false; `NaN !== NaN` is true.
        let same = match (m, n) {
            (None, None) => true,
            (Some(m), Some(n)) => m == n,
            _ => false,
        };
        if !same {
            return m.copied().unwrap_or(0.0) - n.copied().unwrap_or(0.0);
        }
    }
    if x_pre == y_pre {
        return 0.0;
    }
    if x_pre.is_empty() {
        return 1.0;
    }
    if y_pre.is_empty() {
        return -1.0;
    }
    let (p, q): (Vec<&str>, Vec<&str>) = (x_pre.split('.').collect(), y_pre.split('.').collect());
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    for (p, q) in p.iter().zip(&q) {
        if p == q {
            continue;
        }
        let (m, n) = (digits(p), digits(q));
        if m && n {
            return js_number(p) - js_number(q);
        }
        if m != n {
            return if m { -1.0 } else { 1.0 };
        }
        return if p.encode_utf16().lt(q.encode_utf16()) {
            -1.0
        } else {
            1.0
        };
    }
    p.len() as f64 - q.len() as f64
}
