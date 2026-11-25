// Ported from ardia apps/browser/src/lib/profileLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The look of a profile: the preset colour names, the theme types and maths of `@ardia/core`
//! (re-exported from `ria_core::identity::theme`), and the backdrop props of a theme.
//!
//! Changes from Ardia:
//! - `ProfileColor` is an enum; `PROFILE_COLOR_NAMES` holds its values, `ProfileColor::name` the text.
//! - `PresetThemes` (a partial record) is a list of `(ProfileColor, ProfileTheme)` in insertion order,
//!   the order of `Object.keys`.
//! - `spaceLookSource` takes the `spacesFollowAppearance` setting, not the settings object.
//! - `themeBackdrop` reads `lookAngle(theme)` as `look_angle(theme.look.angle)`, the `ria_core` form.
//! - `hexColor` is this file's (it uses `color::parse_color`), not `ria_core`'s `hex_color`, as in Ardia.

pub use ria_core::identity::theme::{
    DARKENED_LOOK_KEEP, DEFAULT_THEME_ANGLE, MAX_THEME_STOPS, PAD_LIGHTNESS, ProfileTheme,
    ShownLook, ThemeBackdropProps, ThemeColor, ThemeLook, ThemeLookSource, ThemeMode, ThemeStop,
    ThemeTexture, arc_layer_alpha, color_hex, darkened_look, flat_colors, from_oklab, look_angle,
    mix_oklab, oklab, opaque_theme_tint, pad_color, pad_position, readable_dark,
    requested_dark_for, resolve_theme, sanitize_theme, shown_look, starting_theme, stop_lightness,
    theme_accent, theme_area, theme_dark, theme_grain_alpha, theme_layer_alpha, theme_luminance,
    theme_palette, theme_under_color, theme_underlay_alpha, with_added_stop, with_angle,
    with_stop_at, with_stop_lightness, without_stop,
};

use super::color::parse_color;

/// `LookSource` (`profileLook.ts:48`).
pub type LookSource = ThemeLookSource;

/// `ProfileColor` (`profileLook.ts:52`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileColor {
    Plum,
    Blue,
    Purple,
    Pink,
    Red,
    Orange,
    Yellow,
    Green,
    Neutral,
}

/// `PROFILE_COLOR_NAMES` (`profileLook.ts:50`).
pub const PROFILE_COLOR_NAMES: [ProfileColor; 9] = [
    ProfileColor::Plum,
    ProfileColor::Blue,
    ProfileColor::Purple,
    ProfileColor::Pink,
    ProfileColor::Red,
    ProfileColor::Orange,
    ProfileColor::Yellow,
    ProfileColor::Green,
    ProfileColor::Neutral,
];

impl ProfileColor {
    /// The JS name of the colour.
    pub fn name(self) -> &'static str {
        match self {
            Self::Plum => "plum",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Pink => "pink",
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Neutral => "neutral",
        }
    }

    /// The colour of a JS name; `None` for any other string.
    pub fn from_name(name: &str) -> Option<Self> {
        PROFILE_COLOR_NAMES.into_iter().find(|c| c.name() == name)
    }
}

/// `PresetThemes` (`profileLook.ts:54`): at most one theme per colour, in insertion order.
pub type PresetThemes = Vec<(ProfileColor, ProfileTheme)>;

/// `ProfileLookFields` (`profileLook.ts:56`).
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileLookFields {
    pub color: ProfileColor,
    pub theme: Option<ProfileTheme>,
}

/// `PresetLookFields` (`profileLook.ts:58`): `preset_themes` is `None` when left out.
#[derive(Clone, Debug, PartialEq)]
pub struct PresetLookFields {
    pub color: ProfileColor,
    pub theme: Option<ProfileTheme>,
    pub preset_themes: Option<PresetThemes>,
}

/// `PRESET_BACKDROP` (`profileLook.ts:60`).
pub const PRESET_BACKDROP: ThemeBackdropProps = ThemeBackdropProps {
    theme_colors: Vec::new(),
    theme_opacity: 1.0,
    theme_base_alpha: 0.0,
    theme_grain: 0.0,
    theme_texture: ThemeTexture::Grain,
    theme_appearance: ThemeMode::Auto,
    theme_angle: DEFAULT_THEME_ANGLE,
};

/// `hexColor` (`profileLook.ts:70`): an opaque theme colour from colour text.
pub fn hex_color(hex: &str) -> ThemeColor {
    let [r, g, b, _] = parse_color(hex);
    ThemeColor {
        r: r / 255.0,
        g: g / 255.0,
        b: b / 255.0,
        a: 1.0,
    }
}

/// `spaceLookSource` (`profileLook.ts:75`).
pub fn space_look_source(spaces_follow_appearance: bool) -> LookSource {
    if spaces_follow_appearance {
        ThemeLookSource::App
    } else {
        ThemeLookSource::Own
    }
}

/// `themeBackdrop` (`profileLook.ts:79`).
pub fn theme_backdrop(theme: &ProfileTheme, dark: bool) -> ThemeBackdropProps {
    ThemeBackdropProps {
        theme_colors: flat_colors(&theme.look),
        theme_opacity: theme_layer_alpha(&theme.look),
        theme_base_alpha: theme_underlay_alpha(&theme.look, dark),
        theme_grain: theme_grain_alpha(&theme.look),
        theme_texture: theme.look.texture,
        theme_appearance: theme.mode,
        theme_angle: look_angle(theme.look.angle),
    }
}
