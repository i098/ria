// Ported from ardia apps/browser/src/lib/color.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Colour parsing and maths: hex and `rgb()` text, mixing, luminance, contrast, HSL.
//! Channels are `f64` because the original keeps `NaN` for text that does not parse.
//!
//! Changes from Ardia: text is UTF-8; lengths and slices that JavaScript takes in UTF-16 code
//! units (`short.length`, `h.slice`) use UTF-16 code units here too. `Math.max` and `Math.min`
//! are `js::js_max` and `js::js_min`, and `Math.hypot` is `js_hypot`: they keep JavaScript's `NaN`,
//! `-0` and rounding rules (Rust's `f64::max` ignores `NaN`, and `f64::hypot` rounds differently).

use regex::Regex;
use std::sync::LazyLock;

use super::js::{js_max, js_min, js_number, js_parse_int, js_round, js_trim};

/// `[r, g, b, a]`.
pub type Rgba = [f64; 4];

// `[^)]` and `.` in these patterns match the same characters as in JavaScript, because neither uses `\w`, `\s` or `\b`.
static RGB_FUNCTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"rgba?\(([^)]+)\)").expect("static pattern"));

/// `isHexColor` (`color.ts:5`): `#` and 6 hex digits at the start of the trimmed text.
pub fn is_hex_color(color: &str) -> bool {
    let t = js_trim(color).as_bytes();
    t.len() >= 7 && t[0] == b'#' && t[1..7].iter().all(u8::is_ascii_hexdigit)
}

/// `isRgbFunction` (`color.ts:7`).
pub fn is_rgb_function(color: &str) -> bool {
    RGB_FUNCTION.is_match(color)
}

/// `parseColor` (`color.ts:9`).
pub fn parse_color(color: &str) -> Rgba {
    if let Some(m) = RGB_FUNCTION.captures(color) {
        let mut parts = m[1].split(',').map(js_number);
        let mut next = |default: f64| parts.next().unwrap_or(default);
        return [next(0.0), next(0.0), next(0.0), next(1.0)];
    }
    // `replace("#", "")` removes the first `#` only.
    let short = js_trim(color).replacen('#', "", 1);
    // `short.length` counts UTF-16 code units; `Array.from` doubles each code point.
    let length = short.encode_utf16().count();
    let h: String = if length == 3 || length == 4 {
        short.chars().flat_map(|c| [c, c]).collect()
    } else {
        short
    };
    let units: Vec<u16> = h.encode_utf16().collect();
    let ch = |i: usize| {
        js_parse_int(
            &String::from_utf16_lossy(
                units
                    .get(2 * i..(2 * i + 2).min(units.len()))
                    .unwrap_or_default(),
            ),
            16,
        )
    };
    [
        ch(0),
        ch(1),
        ch(2),
        if units.len() >= 8 { ch(3) / 255.0 } else { 1.0 },
    ]
}

/// `toHex` (`color.ts:21`): `#RRGGBB[AA]`, each channel clamped to 0..255 and rounded.
pub fn to_hex(channels: &[f64]) -> String {
    let body: String = channels
        .iter()
        // `Math.min(255, Math.max(0, NaN))` is NaN, and `NaN.toString(16)` is `"NaN"`.
        .map(|v| {
            if v.is_nan() {
                "NaN".to_string()
            } else {
                format!("{:02x}", js_round(v.clamp(0.0, 255.0)) as u32)
            }
        })
        .collect();
    format!("#{}", body.to_uppercase())
}

/// `mix` (`color.ts:24`).
pub fn mix(a: &str, b: &str, t: f64) -> String {
    let (from, to) = (parse_color(a), parse_color(b));
    to_hex(&[0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * t))
}

/// `over` (`color.ts:30`).
pub fn over(top: &str, under: &str) -> String {
    let [r, g, b, a] = parse_color(top);
    let base = parse_color(under);
    to_hex(
        &[r, g, b]
            .iter()
            .enumerate()
            .map(|(i, v)| a * v + (1.0 - a) * base[i])
            .collect::<Vec<_>>(),
    )
}

/// `colorDistance` (`color.ts:38`): the distance as a share of black to white.
pub fn color_distance(a: &str, b: &str) -> f64 {
    let (from, to) = (parse_color(a), parse_color(b));
    js_hypot(&[from[0] - to[0], from[1] - to[1], from[2] - to[2]])
        / js_hypot(&[255.0, 255.0, 255.0])
}

/// `luminance` (`color.ts:44`): `0` for text that is not a hex colour.
pub fn luminance(color: &str) -> f64 {
    if !is_hex_color(color) {
        return 0.0;
    }
    let [r, g, b] = parse_color(color)[..3]
        .iter()
        .map(|v| {
            let s = v / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        })
        .collect::<Vec<_>>()[..]
    else {
        unreachable!("3 channels")
    };
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// `contrastRatio` (`color.ts:53`).
pub fn contrast_ratio(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (js_max(la, lb) + 0.05) / (js_min(la, lb) + 0.05)
}

/// `isLight` (`color.ts:58`).
pub fn is_light(color: &str) -> bool {
    let l = luminance(color);
    1.05 / (l + 0.05) < (l + 0.05) / 0.05
}

/// `hslToRgb` (`color.ts:63`).
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
    if s == 0.0 {
        return [l, l, l];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let channel = |t: f64| {
        let t = if t < 0.0 {
            t + 1.0
        } else if t > 1.0 {
            t - 1.0
        } else {
            t
        };
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    [channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)]
}

/// `rgbToHsl` (`color.ts:74`).
pub fn rgb_to_hsl(r: f64, g: f64, b: f64) -> [f64; 3] {
    let (max, min) = (js_max(js_max(r, g), b), js_min(js_min(r, g), b));
    let (l, d) = ((max + min) / 2.0, max - min);
    if d == 0.0 {
        return [0.0, 0.0, l];
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    [h / 6.0, s, l]
}

/// `Math.hypot(...values)` as V8 computes it: `Infinity` before `NaN`, then the Kahan sum of the
/// squares scaled by the largest magnitude.
fn js_hypot(values: &[f64]) -> f64 {
    let max = values
        .iter()
        .filter(|v| !v.is_nan())
        .fold(0.0, |m: f64, v| if v.abs() > m { v.abs() } else { m });
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if values.iter().any(|v| v.is_nan()) {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0.0, 0.0);
    for v in values {
        let n = v.abs() / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}
