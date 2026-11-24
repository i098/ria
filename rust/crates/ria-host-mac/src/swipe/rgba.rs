// Ported from ardia packages/ardiacore/ios/RGBA.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use objc2::rc::Retained;
use objc2_app_kit::NSColor;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

/// Swift's `CharacterSet.whitespaces`: tab and the space separators, not the line breaks.
fn is_blank(c: char) -> bool {
    c.is_whitespace()
        && !matches!(
            c,
            '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        )
}

impl Rgba {
    pub fn from_hex(hex: &str) -> Option<Self> {
        let s = hex.trim_matches(is_blank);
        let s = s.strip_prefix('#').unwrap_or(s);
        let count = s.chars().count();
        if count != 6 && count != 8 {
            return None;
        }
        let v = u64::from_str_radix(s, 16).ok()?;
        let rgba = if count == 6 { (v << 8) | 0xFF } else { v };
        let channel = |shift: u32| ((rgba >> shift) & 0xFF) as f64 / 255.0;
        Some(Self {
            red: channel(24),
            green: channel(16),
            blue: channel(8),
            alpha: channel(0),
        })
    }

    pub fn srgb(self) -> Retained<NSColor> {
        NSColor::colorWithSRGBRed_green_blue_alpha(self.red, self.green, self.blue, self.alpha)
    }

    pub fn display_p3(self) -> Retained<NSColor> {
        NSColor::colorWithDisplayP3Red_green_blue_alpha(self.red, self.green, self.blue, self.alpha)
    }

    pub fn simd(self) -> [f32; 4] {
        [
            self.red as f32,
            self.green as f32,
            self.blue as f32,
            self.alpha as f32,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_forms() {
        let red = Rgba::from_hex(" #FF0000\t").unwrap();
        assert_eq!(
            red,
            Rgba {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0
            }
        );
        assert_eq!(Rgba::from_hex("00000080").unwrap().alpha, 128.0 / 255.0);
        assert_eq!(Rgba::from_hex("#FFF"), None);
        assert_eq!(Rgba::from_hex("GGGGGG"), None);
        assert_eq!(Rgba::from_hex("FF0000\n"), None);
    }
}
