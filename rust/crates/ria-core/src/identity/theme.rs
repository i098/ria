// Ported from ardia packages/core/src/theme.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Profile themes: gradient stops, Oklab mixing, the colors a theme shows and the
//! sanitizing of stored themes. The arithmetic follows JS: NaN goes through, as in `Math.max`.

use std::f64::consts::PI;

use super::js::{js_max, js_min, js_round};

/// The grain textures a theme can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeTexture {
    Grain,
    Sand,
    Denim,
    Tweed,
}

pub const THEME_TEXTURES: [ThemeTexture; 4] = [
    ThemeTexture::Grain,
    ThemeTexture::Sand,
    ThemeTexture::Denim,
    ThemeTexture::Tweed,
];

impl ThemeTexture {
    /// The JS name of the texture.
    pub fn name(self) -> &'static str {
        match self {
            Self::Grain => "grain",
            Self::Sand => "sand",
            Self::Denim => "denim",
            Self::Tweed => "tweed",
        }
    }

    /// The texture of a JS name; `None` for any other string.
    pub fn from_name(name: &str) -> Option<Self> {
        THEME_TEXTURES.into_iter().find(|t| t.name() == name)
    }

    /// `TEXTURE_STRENGTH[texture]`.
    pub fn strength(self) -> f64 {
        match self {
            Self::Grain => 0.8,
            Self::Sand => 0.15,
            Self::Denim => 0.6,
            Self::Tweed => 0.4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Auto,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Auto, Self::Light, Self::Dark]
            .into_iter()
            .find(|m| m.name() == name)
    }
}

/// The props of the theme backdrop view.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeBackdropProps {
    pub theme_colors: Vec<f64>,
    pub theme_opacity: f64,
    pub theme_base_alpha: f64,
    pub theme_grain: f64,
    pub theme_texture: ThemeTexture,
    pub theme_appearance: ThemeMode,
    pub theme_angle: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeStop {
    pub color: ThemeColor,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeLook {
    pub stops: Vec<ThemeStop>,
    pub intensity: f64,
    pub grain: f64,
    pub texture: ThemeTexture,
    pub translucency: f64,
    pub angle: Option<f64>,
}

/// `ThemeLook & { mode; dark? }`.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileTheme {
    pub look: ThemeLook,
    pub mode: ThemeMode,
    pub dark: Option<ThemeLook>,
}

impl AsRef<ThemeLook> for ThemeLook {
    fn as_ref(&self) -> &ThemeLook {
        self
    }
}

impl AsMut<ThemeLook> for ThemeLook {
    fn as_mut(&mut self) -> &mut ThemeLook {
        self
    }
}

impl AsRef<ThemeLook> for ProfileTheme {
    fn as_ref(&self) -> &ThemeLook {
        &self.look
    }
}

impl AsMut<ThemeLook> for ProfileTheme {
    fn as_mut(&mut self) -> &mut ThemeLook {
        &mut self.look
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeLookSource {
    Own,
    App,
    Utility,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShownLook {
    pub look: ProfileTheme,
    pub requested_dark: bool,
    pub dark: bool,
}

pub const MAX_THEME_STOPS: usize = 3;
pub const PAD_LIGHTNESS: f64 = 0.6;
pub const DEFAULT_THEME_ANGLE: f64 = 135.0;
pub const DARKENED_LOOK_KEEP: f64 = 0.4;
pub const STARTING_NOISE: f64 = 0.5;

const BASE_REACH: f64 = 0.75;
const MAX_SANITIZED_STOPS: usize = 16;
const SAMPLE_POINTS: [f64; 3] = [0.0, 0.5, 1.0];
const MIN_BLACK_TEXT_CONTRAST: f64 = 3.0;
const MIN_WHITE_TEXT_CONTRAST: f64 = 2.0;

pub type Lab = [f64; 3];
pub type Rgb = [f64; 3];

/// JS `Math.sign` (`f64::signum` gives 1 for +0).
fn js_sign(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        x
    } else {
        x.signum()
    }
}

/// V8's `Math.cbrt`, FreeBSD's `s_cbrt.c`; the system `cbrt` differs in the last bit for about
/// half of all inputs, and Oklab of out-of-range colors magnifies that.
fn js_cbrt(x: f64) -> f64 {
    const B1: u64 = 715_094_163;
    const B2: u64 = 696_219_795;
    const P0: f64 = 1.875_951_824_271_77;
    const P1: f64 = -1.884_979_795_433_771_7;
    const P2: f64 = 1.621_429_720_105_354_5;
    const P3: f64 = -0.758_397_934_778_766;
    const P4: f64 = 0.145_996_192_886_612_45;
    let high = |v: f64| v.to_bits() >> 32;
    let sign = high(x) & 0x8000_0000;
    let hx = high(x) ^ sign;
    if hx >= 0x7ff0_0000 {
        return x + x;
    }
    let mut t = if hx < 0x0010_0000 {
        if x == 0.0 {
            return x;
        }
        let scaled = f64::from_bits(0x4350_0000 << 32) * x;
        f64::from_bits((sign | ((high(scaled) & 0x7fff_ffff) / 3 + B2)) << 32)
    } else {
        f64::from_bits((sign | (hx / 3 + B1)) << 32)
    };
    let r = (t * t) * (t / x);
    t *= (P0 + r * (P1 + r * P2)) + ((r * r) * r) * (P3 + r * P4);
    t = f64::from_bits((t.to_bits() + 0x8000_0000) & 0xffff_ffff_c000_0000);
    let s = t * t;
    let r = x / s;
    let w = t + t;
    t + t * ((r - t) / (w + r))
}

/// V8's `Math.hypot`: scaled by the largest value, Kahan-summed.
fn js_hypot(a: f64, b: f64) -> f64 {
    let (a, b) = (a.abs(), b.abs());
    if a == f64::INFINITY || b == f64::INFINITY {
        return f64::INFINITY;
    }
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    let max = if b > a { b } else { a };
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0.0, 0.0);
    for v in [a, b] {
        let n = v / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

fn clamp01(v: f64) -> f64 {
    js_min(1.0, js_max(0.0, v))
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
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

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> [f64; 3] {
    let max = js_max(js_max(r, g), b);
    let min = js_min(js_min(r, g), b);
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

/// One channel as two upper-case hex digits; JS gives `NAN` for NaN.
fn hex_byte(v: f64) -> String {
    let x = js_round(js_min(255.0, js_max(0.0, clamp01(v) * 255.0)));
    if x.is_nan() {
        "NAN".into()
    } else {
        format!("{:02X}", x as u8)
    }
}

fn unit_hex(c: &[f64]) -> String {
    std::iter::once("#".to_string())
        .chain(c.iter().map(|&v| hex_byte(v)))
        .collect()
}

/// The color of the color pad at `x`, `y`: hue by angle, saturation by distance from the center.
pub fn pad_color(x: f64, y: f64, lightness: f64) -> ThemeColor {
    let (dx, dy) = (clamp01(x) - 0.5, clamp01(y) - 0.5);
    let mut hue = dy.atan2(dx) / (2.0 * PI);
    if hue < 0.0 {
        hue += 1.0;
    }
    let sat = js_min(1.0, js_hypot(dx, dy) / 0.5);
    let [r, g, b] = hsl_to_rgb(hue, sat, clamp01(lightness));
    ThemeColor { r, g, b, a: 1.0 }
}

/// Where `color` sits on the color pad: the inverse of `pad_color`, lightness dropped.
pub fn pad_position(color: &ThemeColor) -> (f64, f64) {
    let [h, s, _] = rgb_to_hsl(clamp01(color.r), clamp01(color.g), clamp01(color.b));
    let angle = h * 2.0 * PI;
    (0.5 + 0.5 * s * angle.cos(), 0.5 + 0.5 * s * angle.sin())
}

fn decode(v: f64) -> f64 {
    let a = v.abs();
    js_sign(v)
        * if a <= 0.04045 {
            a / 12.92
        } else {
            ((a + 0.055) / 1.055).powf(2.4)
        }
}

fn encode(v: f64) -> f64 {
    let a = v.abs();
    js_sign(v)
        * if a <= 0.0031308 {
            a * 12.92
        } else {
            1.055 * a.powf(1.0 / 2.4) - 0.055
        }
}

/// sRGB to Oklab.
pub fn oklab(c: &ThemeColor) -> Lab {
    let [r, g, b] = [c.r, c.g, c.b].map(decode);
    let l = js_cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
    let m = js_cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
    let s = js_cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
    [
        0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    ]
}

/// Oklab to sRGB, not clamped.
pub fn from_oklab([l, a, b]: Lab) -> Rgb {
    let lc = (l + 0.3963377774 * a + 0.2158037573 * b).powf(3.0);
    let m = (l - 0.1055613458 * a - 0.0638541728 * b).powf(3.0);
    let s = (l - 0.0894841775 * a - 1.291485548 * b).powf(3.0);
    [
        4.0767416621 * lc - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * lc + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * lc - 0.7034186147 * m + 1.707614701 * s,
    ]
    .map(encode)
}

/// `#RRGGBB` of a color.
pub fn color_hex(color: &ThemeColor) -> String {
    unit_hex(&[color.r, color.g, color.b])
}

/// JS whitespace for `trim` and `parseInt`, as UTF-16 units.
fn is_js_space_unit(u: u16) -> bool {
    char::from_u32(u.into()).is_some_and(crate::fuzzy::is_js_space)
}

/// JS `parseInt(units, 16)`: NaN when no hex digit follows the space, sign and `0x`.
fn parse_hex_int(units: &[u16]) -> f64 {
    let mut rest = units;
    while let [u, tail @ ..] = rest
        && is_js_space_unit(*u)
    {
        rest = tail;
    }
    let mut sign = 1.0;
    if let [u @ (0x2B | 0x2D), tail @ ..] = rest {
        sign = if *u == 0x2D { -1.0 } else { 1.0 };
        rest = tail;
    }
    if let [0x30, 0x58 | 0x78, tail @ ..] = rest {
        rest = tail;
    }
    let digits: Vec<u32> = rest
        .iter()
        .map_while(|&u| char::from_u32(u.into()).and_then(|c| c.to_digit(16)))
        .collect();
    if digits.is_empty() {
        return f64::NAN;
    }
    sign * f64::from(digits.iter().fold(0, |n, d| n * 16 + d))
}

/// The color of a `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA` string; its alpha is dropped.
/// A channel that is not hex is NaN, as in JS.
pub fn hex_color(hex: &str) -> ThemeColor {
    let units: Vec<u16> = hex.encode_utf16().collect();
    let start = units
        .iter()
        .position(|&u| !is_js_space_unit(u))
        .unwrap_or(units.len());
    let end = units
        .iter()
        .rposition(|&u| !is_js_space_unit(u))
        .map_or(start, |e| e + 1);
    let mut short = units[start..end].to_vec();
    if let Some(at) = short.iter().position(|&u| u == u16::from(b'#')) {
        short.remove(at);
    }
    let h: Vec<u16> = if short.len() == 3 || short.len() == 4 {
        // `Array.from` walks code points: a surrogate pair doubles as one.
        char::decode_utf16(short.iter().copied())
            .flat_map(|c| {
                let mut buf = [0; 2];
                let unit = match c {
                    Ok(c) => c.encode_utf16(&mut buf).to_vec(),
                    Err(e) => vec![e.unpaired_surrogate()],
                };
                [unit.clone(), unit].concat()
            })
            .collect()
    } else {
        short
    };
    let ch = |i: usize| {
        let from = (2 * i).min(h.len());
        parse_hex_int(&h[from..(2 * i + 2).min(h.len())]) / 255.0
    };
    ThemeColor {
        r: ch(0),
        g: ch(1),
        b: ch(2),
        a: 1.0,
    }
}

/// Mixes `a` and `b` in Oklab; `t` 0 is `a`.
pub fn mix_oklab(a: &ThemeColor, b: &ThemeColor, t: f64) -> Rgb {
    let (la, lb) = (oklab(a), oklab(b));
    from_oklab([0, 1, 2].map(|i| la[i] + (lb[i] - la[i]) * t))
}

pub fn look_angle(angle: Option<f64>) -> f64 {
    angle.unwrap_or(DEFAULT_THEME_ANGLE)
}

/// The gradient position of a point along `degrees`, scaled by `half` of the box's reach.
fn along(degrees: f64, x: f64, y: f64, width: f64, height: f64, overlay: bool) -> f64 {
    let r = (degrees * PI) / 180.0;
    let (dx, dy) = (r.sin(), -r.cos());
    let half = if overlay {
        (width * dx.abs() + height * dy.abs()) / 2.0
    } else {
        (js_hypot(width * dx, height * dy) * js_hypot(width, height))
            / (2.0 * js_max(width, height))
    };
    ((x - width / 2.0) * dx + (y - height / 2.0) * dy) / (2.0 * half) + 0.5
}

/// The theme color at `x`, `y` of a `width` by `height` box.
///
/// # Panics
///
/// When `theme.stops` is empty: the original throws there.
pub fn theme_area(theme: &ThemeLook, x: f64, y: f64, width: f64, height: f64) -> Rgb {
    let c0 = &theme.stops[0].color;
    let c1 = theme.stops.get(1).map_or(c0, |s| &s.color);
    let angle = look_angle(theme.angle);
    let t = clamp01(along(angle, x, y, width, height, false) / BASE_REACH);
    let base = mix_oklab(c0, c1, t);
    let Some(c2) = theme.stops.get(2).map(|s| &s.color) else {
        return base;
    };
    let a = clamp01(along(angle + 45.0, x, y, width, height, true)) * c2.a;
    let over = [c2.r, c2.g, c2.b];
    [0, 1, 2].map(|i| base[i] + (over[i] - base[i]) * a)
}

pub fn theme_dark(theme: &ProfileTheme, app_dark: bool) -> bool {
    match theme.mode {
        ThemeMode::Auto => app_dark,
        mode => mode == ThemeMode::Dark,
    }
}

/// The look to show: the dark look, when `dark` and there is one, takes the place of the light
/// one (its own angle or none).
pub fn resolve_theme(theme: &ProfileTheme, dark: bool) -> ProfileTheme {
    let look = match &theme.dark {
        Some(dark_look) if dark => dark_look.clone(),
        _ => theme.look.clone(),
    };
    ProfileTheme {
        look,
        mode: theme.mode,
        dark: None,
    }
}

/// The theme with each stop color scaled by `keep`.
pub fn darkened_look(theme: &ProfileTheme, keep: f64) -> ProfileTheme {
    let mut out = theme.clone();
    for s in &mut out.look.stops {
        s.color.r *= keep;
        s.color.g *= keep;
        s.color.b *= keep;
    }
    out
}

pub fn requested_dark_for(theme: &ProfileTheme, app_dark: bool, source: ThemeLookSource) -> bool {
    if source == ThemeLookSource::Own {
        theme_dark(theme, app_dark)
    } else {
        app_dark
    }
}

/// The look a page shows and whether its text goes dark-mode.
///
/// # Panics
///
/// When a look it measures has no stops, as `theme_area`.
pub fn shown_look(theme: &ProfileTheme, app_dark: bool, source: ThemeLookSource) -> ShownLook {
    let requested_dark = requested_dark_for(theme, app_dark, source);
    let resolved = resolve_theme(theme, requested_dark);
    let light_only = theme.mode != ThemeMode::Dark && theme.dark.is_none();
    let look = if source != ThemeLookSource::Own
        && requested_dark
        && light_only
        && !readable_dark(&resolved.look, true)
    {
        darkened_look(&resolved, DARKENED_LOOK_KEEP)
    } else {
        resolved
    };
    let dark = if source == ThemeLookSource::Utility {
        requested_dark
    } else {
        readable_dark(&look.look, requested_dark)
    };
    ShownLook {
        look,
        requested_dark,
        dark,
    }
}

pub fn arc_layer_alpha(intensity: f64) -> f64 {
    0.2 + 0.55 * clamp01(intensity)
}

pub fn theme_layer_alpha(theme: &ThemeLook) -> f64 {
    1.0 - clamp01(theme.translucency) * (1.0 - arc_layer_alpha(theme.intensity))
}

pub fn theme_underlay_alpha(theme: &ThemeLook, dark: bool) -> f64 {
    let arc = js_min(arc_layer_alpha(theme.intensity), 0.5);
    clamp01(theme.translucency) * if dark { 0.35 * arc } else { arc }
}

pub fn theme_grain_alpha(theme: &ThemeLook) -> f64 {
    clamp01(theme.grain) * theme.texture.strength()
}

/// `r, g, b, a` of the first `MAX_THEME_STOPS` stops.
pub fn flat_colors(theme: &ThemeLook) -> Vec<f64> {
    theme
        .stops
        .iter()
        .take(MAX_THEME_STOPS)
        .flat_map(|s| [s.color.r, s.color.g, s.color.b, s.color.a])
        .collect()
}

/// # Panics
///
/// When `theme.stops` is empty, as `theme_area`.
pub fn theme_accent(theme: &ThemeLook) -> String {
    unit_hex(&theme_area(theme, 0.5, 0.5, 1.0, 1.0))
}

pub fn theme_palette(theme: &ThemeLook) -> Vec<String> {
    theme.stops.iter().map(|s| color_hex(&s.color)).collect()
}

fn backdrop_under_grey(underlay_alpha: f64, dark: bool) -> f64 {
    let u = clamp01(underlay_alpha);
    let blur = if dark { 0.16 } else { 0.92 };
    (if dark { 0.0 } else { 1.0 }) * u + blur * (1.0 - u)
}

pub fn backdrop_under_color(underlay_alpha: f64, dark: bool) -> String {
    let grey = backdrop_under_grey(underlay_alpha, dark);
    unit_hex(&[grey, grey, grey])
}

pub fn theme_under_color(theme: &ThemeLook, dark: bool) -> String {
    backdrop_under_color(theme_underlay_alpha(theme, dark), dark)
}

fn composite(theme: &ThemeLook, dark: bool, x: f64, y: f64) -> Rgb {
    let a = theme_layer_alpha(theme);
    let under = backdrop_under_grey(theme_underlay_alpha(theme, dark), dark);
    theme_area(theme, x, y, 1.0, 1.0).map(|v| a * clamp01(v) + (1.0 - a) * under)
}

/// The opaque colors of the top-left and bottom-right corners.
///
/// # Panics
///
/// When `theme.stops` is empty, as `theme_area`.
pub fn opaque_theme_tint(theme: &ThemeLook, dark: bool) -> [String; 2] {
    [
        composite(theme, dark, 0.0, 0.0),
        composite(theme, dark, 1.0, 1.0),
    ]
    .map(|c| unit_hex(&c))
}

fn relative_luminance(c: Rgb) -> f64 {
    let [r, g, b] = c.map(|v| decode(clamp01(v)));
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// The mean relative luminance of nine sample points.
///
/// # Panics
///
/// When `theme.stops` is empty, as `theme_area`.
pub fn theme_luminance(theme: &ThemeLook, dark: bool) -> f64 {
    let mut sum = 0.0;
    for x in SAMPLE_POINTS {
        for y in SAMPLE_POINTS {
            sum += relative_luminance(composite(theme, dark, x, y));
        }
    }
    sum / (SAMPLE_POINTS.len() * SAMPLE_POINTS.len()) as f64
}

/// Whether text on the theme reads better white (dark mode), starting from `dark`.
///
/// # Panics
///
/// When `theme.stops` is empty, as `theme_area`.
pub fn readable_dark(theme: &ThemeLook, dark: bool) -> bool {
    let y = theme_luminance(theme, dark);
    let (black_text, white_text) = ((y + 0.05) / 0.05, 1.05 / (y + 0.05));
    if dark {
        !(white_text < MIN_WHITE_TEXT_CONTRAST && black_text > white_text)
    } else {
        black_text < MIN_BLACK_TEXT_CONTRAST && white_text > black_text
    }
}

pub fn stop_lightness(stop: &ThemeStop) -> f64 {
    rgb_to_hsl(
        clamp01(stop.color.r),
        clamp01(stop.color.g),
        clamp01(stop.color.b),
    )[2]
}

/// The stop at `index` moved to `x`, `y`, keeping its lightness and alpha.
pub fn with_stop_at<T: AsRef<ThemeLook> + AsMut<ThemeLook> + Clone>(
    theme: &T,
    index: f64,
    x: f64,
    y: f64,
) -> T {
    let mut out = theme.clone();
    for (i, s) in out.as_mut().stops.iter_mut().enumerate() {
        if i as f64 == index {
            let color = pad_color(x, y, stop_lightness(s));
            *s = ThemeStop {
                x,
                y,
                color: ThemeColor {
                    a: s.color.a,
                    ..color
                },
            };
        }
    }
    out
}

/// The stop at `index` with a new lightness, keeping its place and alpha.
pub fn with_stop_lightness<T: AsRef<ThemeLook> + AsMut<ThemeLook> + Clone>(
    theme: &T,
    index: f64,
    lightness: f64,
) -> T {
    let mut out = theme.clone();
    for (i, s) in out.as_mut().stops.iter_mut().enumerate() {
        if i as f64 == index {
            s.color = ThemeColor {
                a: s.color.a,
                ..pad_color(s.x, s.y, lightness)
            };
        }
    }
    out
}

/// `degrees` rounded into `0..360`.
pub fn wrap_angle(degrees: f64) -> f64 {
    ((js_round(degrees) % 360.0) + 360.0) % 360.0
}

pub fn with_angle<T: AsMut<ThemeLook> + Clone>(theme: &T, degrees: f64) -> T {
    let mut out = theme.clone();
    out.as_mut().angle = Some(wrap_angle(degrees));
    out
}

/// The theme with one more stop, a fifth of a turn past the last; unchanged at the maximum.
pub fn with_added_stop<T: AsRef<ThemeLook> + AsMut<ThemeLook> + Clone>(theme: &T) -> T {
    let mut out = theme.clone();
    let stops = &mut out.as_mut().stops;
    if stops.len() >= MAX_THEME_STOPS {
        return out;
    }
    let (lx, ly) = stops.last().map_or((0.8, 0.5), |s| (s.x, s.y));
    let angle = (ly - 0.5).atan2(lx - 0.5) + (2.0 * PI) / 5.0;
    let radius = js_max(0.3, js_hypot(lx - 0.5, ly - 0.5));
    let (x, y) = (0.5 + radius * angle.cos(), 0.5 + radius * angle.sin());
    stops.push(ThemeStop {
        x,
        y,
        color: pad_color(x, y, PAD_LIGHTNESS),
    });
    out
}

/// The theme without the stop at `index`; the last stop stays.
pub fn without_stop<T: AsRef<ThemeLook> + AsMut<ThemeLook> + Clone>(theme: &T, index: f64) -> T {
    let mut out = theme.clone();
    let stops = &mut out.as_mut().stops;
    if stops.len() > 1 {
        let mut i = 0.0;
        stops.retain(|_| {
            i += 1.0;
            i - 1.0 != index
        });
    }
    out
}

/// `theme.grain || STARTING_NOISE`: a texture needs some grain to show.
pub fn grain_or_starting(grain: f64) -> f64 {
    if grain == 0.0 || grain.is_nan() {
        STARTING_NOISE
    } else {
        grain
    }
}

pub fn with_texture<T: AsRef<ThemeLook> + AsMut<ThemeLook> + Clone>(
    theme: &T,
    texture: ThemeTexture,
) -> T {
    let mut out = theme.clone();
    let look = out.as_mut();
    look.texture = texture;
    look.grain = grain_or_starting(look.grain);
    out
}

/// A two-stop theme around the color of `swatch`.
pub fn starting_theme(swatch: &str) -> ProfileTheme {
    let (bx, by) = pad_position(&hex_color(swatch));
    let angle = (by - 0.5).atan2(bx - 0.5);
    let radius = js_max(0.3, js_hypot(bx - 0.5, by - 0.5));
    let stop = |offset: f64| {
        let (x, y) = (
            0.5 + radius * (angle + offset).cos(),
            0.5 + radius * (angle + offset).sin(),
        );
        ThemeStop {
            x,
            y,
            color: pad_color(x, y, PAD_LIGHTNESS),
        }
    };
    ProfileTheme {
        look: ThemeLook {
            stops: vec![stop(-0.45), stop(0.45)],
            intensity: 0.6,
            grain: 0.0,
            texture: ThemeTexture::Grain,
            translucency: 1.0,
            angle: None,
        },
        mode: ThemeMode::Auto,
        dark: None,
    }
}

/// A stored color read without trusting its shape: `None` for a missing field or one that
/// is not a number.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawColor {
    pub r: Option<f64>,
    pub g: Option<f64>,
    pub b: Option<f64>,
    pub a: Option<f64>,
}

/// A stored stop; `color` is `None` when it is not an object.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawStop {
    pub color: Option<RawColor>,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

/// A stored look; `stops` is `None` when it is not an array, `texture` when it is not a texture name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawLook {
    pub stops: Option<Vec<RawStop>>,
    pub intensity: Option<f64>,
    pub grain: Option<f64>,
    pub texture: Option<ThemeTexture>,
    pub translucency: Option<f64>,
    pub angle: Option<f64>,
}

/// A stored theme; `dark` is `None` when it is not an object.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawTheme {
    pub look: RawLook,
    pub mode: Option<ThemeMode>,
    pub dark: Option<RawLook>,
}

fn finite(v: Option<f64>) -> Option<f64> {
    v.filter(|n| n.is_finite())
}

fn sanitize_color(c: &RawColor) -> Option<ThemeColor> {
    let part = |n| finite(n).filter(|n: &f64| n.abs() < 4.0);
    Some(ThemeColor {
        r: part(c.r)?,
        g: part(c.g)?,
        b: part(c.b)?,
        a: clamp01(finite(c.a).unwrap_or(1.0)),
    })
}

fn sanitize_look(t: &RawLook) -> Option<ThemeLook> {
    let stops: Vec<ThemeStop> = t
        .stops
        .as_ref()?
        .iter()
        .take(MAX_SANITIZED_STOPS)
        .filter_map(|s| {
            Some(ThemeStop {
                color: sanitize_color(s.color.as_ref()?)?,
                x: clamp01(finite(s.x).unwrap_or(0.5)),
                y: clamp01(finite(s.y).unwrap_or(0.5)),
            })
        })
        .take(MAX_THEME_STOPS)
        .collect();
    if stops.is_empty() {
        return None;
    }
    Some(ThemeLook {
        stops,
        intensity: clamp01(finite(t.intensity).unwrap_or(0.6)),
        grain: clamp01(finite(t.grain).unwrap_or(0.0)),
        texture: t.texture.unwrap_or(ThemeTexture::Grain),
        translucency: clamp01(finite(t.translucency).unwrap_or(1.0)),
        angle: finite(t.angle).map(|a| ((a % 360.0) + 360.0) % 360.0),
    })
}

/// A valid theme from stored data: bad stops dropped, numbers clamped, defaults filled in;
/// `None` when no stop is valid.
pub fn sanitize_theme(v: &RawTheme) -> Option<ProfileTheme> {
    Some(ProfileTheme {
        look: sanitize_look(&v.look)?,
        mode: match v.mode {
            Some(m @ (ThemeMode::Light | ThemeMode::Dark)) => m,
            _ => ThemeMode::Auto,
        },
        dark: v.dark.as_ref().and_then(sanitize_look),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklab_round_trips() {
        for c in [
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.2, 0.7, 0.4],
            [1.0, 0.0, 0.5],
        ] {
            let back = from_oklab(oklab(&ThemeColor {
                r: c[0],
                g: c[1],
                b: c[2],
                a: 1.0,
            }));
            for i in 0..3 {
                assert!((back[i] - c[i]).abs() < 1e-6, "{c:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn cbrt_matches_v8() {
        // V8 gives ..525 here; glibc `cbrt` gives ..524.
        assert_eq!(js_cbrt(0.14113384864345838), 0.5206474290872525);
        assert_eq!(js_cbrt(-27.0), -3.0);
        assert_eq!(js_cbrt(-0.0).to_bits(), (-0.0f64).to_bits());
        assert!(js_cbrt(f64::NAN).is_nan());
        assert_eq!(js_cbrt(5e-324), 1.7031839360032603e-108);
    }

    #[test]
    fn hex_parse_lengths_and_junk() {
        assert_eq!(hex_color(" #fA0 ").r, 1.0);
        assert_eq!(hex_color("#00ff80cc").g, 1.0);
        assert!(hex_color("#zz0000").r.is_nan());
        assert_eq!(hex_color("-f0000").r, -15.0 / 255.0);
        assert!(hex_color("0x0000").r.is_nan());
        assert!(hex_color("#12").g.is_nan());
        assert_eq!(
            color_hex(&ThemeColor {
                r: 0.5,
                g: f64::NAN,
                b: 2.0,
                a: 1.0
            }),
            "#80NANFF"
        );
    }

    #[test]
    fn sanitize_bounds() {
        let color = |r| {
            Some(RawColor {
                r: Some(r),
                g: Some(0.0),
                b: Some(0.0),
                a: None,
            })
        };
        let raw = RawTheme {
            look: RawLook {
                stops: Some(vec![
                    RawStop {
                        color: color(4.0),
                        ..RawStop::default()
                    },
                    RawStop {
                        color: color(-3.9),
                        x: Some(f64::NAN),
                        y: Some(2.0),
                    },
                ]),
                angle: Some(-30.0),
                ..RawLook::default()
            },
            mode: Some(ThemeMode::Light),
            dark: Some(RawLook::default()),
        };
        let t = sanitize_theme(&raw).unwrap();
        assert_eq!(t.look.stops.len(), 1);
        assert_eq!((t.look.stops[0].x, t.look.stops[0].y), (0.5, 1.0));
        assert_eq!(t.look.angle, Some(330.0));
        assert_eq!((t.mode, t.dark), (ThemeMode::Light, None));
        assert_eq!(sanitize_theme(&RawTheme::default()), None);
    }
}
