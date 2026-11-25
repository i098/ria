// Ported from ardia apps/browser/src/lib/presetLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The own looks of the preset colours: the starting theme of a swatch, the theme each preset
//! shows, the "untouched" test, the edits that keep one look per colour, and the snapping of the
//! theme editor's slider and pad.
//!
//! Changes from Ardia:
//! - `presetThemes` is `Option<PresetThemes>` (`None` when left out); a `ProfileTheme | null` is
//!   `Option<ProfileTheme>`, and `ProfileTheme | null | undefined` is `Option<Option<ProfileTheme>>`
//!   (outer `None` is `undefined`).
//! - `withOwnLooksSeeded` is generic over any profile in TS; here it takes `SeededProfile`, the
//!   fields it reads and writes. A caller keeps the other fields of the profile.
//! - `withOwnLooksSeeded` compares `JSON.stringify` texts of the two themes. Here both themes are
//!   written with the fields in one fixed order (`theme_json`), so two themes that differ only in
//!   key order are the same; non-finite numbers and `-0` compare as `JSON.stringify` writes them.
//! - `profilesWithOwnLooks` takes and gives the entries of the record in `Object.entries` order.
//! - `snapPointToStart` takes and gives `(x, y)` pairs.
//! - `trackPress` is the struct `TrackPress`, its closures are methods.

use ria_core::identity::telemetry_wire::Json;

use super::js::{js_max, js_min};
use super::json_schema::json_stringify;
use super::profile_look::{
    PresetLookFields, PresetThemes, ProfileColor, ProfileTheme, ThemeLook, ThemeMode, ThemeStop,
    ThemeTexture, hex_color, look_angle, pad_position,
};

/// `presetStartingTheme` (`presetLook.ts:3`).
pub fn preset_starting_theme(swatch: &str) -> ProfileTheme {
    let color = hex_color(swatch);
    let (x, y) = pad_position(&color);
    ProfileTheme {
        look: ThemeLook {
            stops: vec![ThemeStop { color, x, y }],
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

/// `ownLooksOf` (`presetLook.ts:8`).
fn own_looks_of(profile: &PresetLookFields) -> &[(ProfileColor, ProfileTheme)] {
    profile.preset_themes.as_deref().unwrap_or_default()
}

/// The theme of `color` in `looks`.
fn look_of(looks: &[(ProfileColor, ProfileTheme)], color: ProfileColor) -> Option<ProfileTheme> {
    looks
        .iter()
        .find(|(c, _)| *c == color)
        .map(|(_, t)| t.clone())
}

/// `looks` without `color`, then `theme` as `color` when it is set: `{ ...others, [color]: theme }`.
fn with_look(
    looks: &[(ProfileColor, ProfileTheme)],
    color: ProfileColor,
    theme: Option<&ProfileTheme>,
) -> PresetThemes {
    let others = looks.iter().filter(|(c, _)| *c != color).cloned();
    others.chain(theme.map(|t| (color, t.clone()))).collect()
}

/// `presetThemeOf` (`presetLook.ts:10`).
pub fn preset_theme_of(profile: &PresetLookFields, color: ProfileColor) -> Option<ProfileTheme> {
    if color == profile.color {
        return profile.theme.clone();
    }
    look_of(own_looks_of(profile), color)
}

const SAME_VALUE: f64 = 1e-9;
const SAME_COLOUR: f64 = 0.002;
const SAME_ANGLE: f64 = 0.5;

/// `near` (`presetLook.ts:19`).
fn near(a: f64, b: f64, within: f64) -> bool {
    (a - b).abs() <= within
}

/// `sameLook` (`presetLook.ts:21`).
fn same_look(a: &ThemeLook, b: &ThemeLook) -> bool {
    if a.texture != b.texture || a.stops.len() != b.stops.len() {
        return false;
    }
    if !near(a.intensity, b.intensity, SAME_VALUE)
        || !near(a.grain, b.grain, SAME_VALUE)
        || !near(a.translucency, b.translucency, SAME_VALUE)
        || !near(look_angle(a.angle), look_angle(b.angle), SAME_ANGLE)
    {
        return false;
    }
    a.stops.iter().zip(&b.stops).all(|(s, o)| {
        let colour = |x: f64, y: f64| near(x, y, SAME_COLOUR);
        near(s.x, o.x, SAME_VALUE)
            && near(s.y, o.y, SAME_VALUE)
            && colour(s.color.r, o.color.r)
            && colour(s.color.g, o.color.g)
            && colour(s.color.b, o.color.b)
            && colour(s.color.a, o.color.a)
    })
}

/// `snapToStart` (`presetLook.ts:31`).
pub fn snap_to_start(value: f64, start: f64, track_pixels: f64) -> f64 {
    if (value - start).abs() * track_pixels <= 0.5 {
        start
    } else {
        value
    }
}

/// `snapPointToStart` (`presetLook.ts:35`).
pub fn snap_point_to_start(point: (f64, f64), start: (f64, f64), pad_pixels: f64) -> (f64, f64) {
    if (point.0 - start.0).abs() * pad_pixels <= 0.5
        && (point.1 - start.1).abs() * pad_pixels <= 0.5
    {
        start
    } else {
        point
    }
}

/// `isUntouched` (`presetLook.ts:39`).
pub fn is_untouched(theme: &ProfileTheme, start: &ProfileTheme) -> bool {
    theme.dark.is_none() && same_theme(Some(theme), Some(start))
}

/// `sameTheme` (`presetLook.ts:43`).
pub fn same_theme(a: Option<&ProfileTheme>, b: Option<&ProfileTheme>) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return a.is_none() && b.is_none();
    };
    if a.mode != b.mode || !same_look(&a.look, &b.look) {
        return false;
    }
    match (&a.dark, &b.dark) {
        (Some(x), Some(y)) => same_look(x, y),
        (x, y) => x.is_none() && y.is_none(),
    }
}

/// `OwnLook` (`presetLook.ts:49`).
#[derive(Clone, Debug, PartialEq)]
pub struct OwnLook {
    pub theme: Option<ProfileTheme>,
    pub preset_themes: PresetThemes,
}

/// `withOwnLook` (`presetLook.ts:51`).
pub fn with_own_look(profile: &PresetLookFields, theme: Option<ProfileTheme>) -> OwnLook {
    let preset_themes = with_look(own_looks_of(profile), profile.color, theme.as_ref());
    OwnLook {
        theme,
        preset_themes,
    }
}

/// `PresetChoice` (`presetLook.ts:56`).
#[derive(Clone, Debug, PartialEq)]
pub struct PresetChoice {
    pub color: ProfileColor,
    pub theme: Option<ProfileTheme>,
}

/// `withPresetChosen` (`presetLook.ts:58`).
pub fn with_preset_chosen(profile: &PresetLookFields, color: ProfileColor) -> Option<PresetChoice> {
    if color == profile.color {
        return None;
    }
    Some(PresetChoice {
        color,
        theme: look_of(own_looks_of(profile), color),
    })
}

/// `SyncedColor` (`presetLook.ts:63`): `theme` is `None` for `undefined`.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncedColor {
    pub color: ProfileColor,
    pub theme: Option<Option<ProfileTheme>>,
}

/// `withSyncedColor` (`presetLook.ts:65`).
pub fn with_synced_color(
    profile: &PresetLookFields,
    color: ProfileColor,
    synced: Option<Option<ProfileTheme>>,
) -> Option<SyncedColor> {
    (color != profile.color).then_some(SyncedColor {
        color,
        theme: synced,
    })
}

/// `editedPresetCount` (`presetLook.ts:69`).
pub fn edited_preset_count(profile: &PresetLookFields) -> usize {
    own_looks_of(profile).len()
}

/// The fields `withOwnLooksSeeded` (`presetLook.ts:73`) reads and writes; `seen_theme` is `None` for
/// `undefined`.
#[derive(Clone, Debug, PartialEq)]
pub struct SeededProfile {
    pub fields: PresetLookFields,
    pub seen_theme: Option<Option<ProfileTheme>>,
    pub seen_color: Option<ProfileColor>,
}

/// A theme as `JSON.stringify` writes it, the fields in one fixed order.
fn theme_json(theme: Option<&ProfileTheme>) -> String {
    fn look(l: &ThemeLook, out: &mut Vec<(String, Json)>) {
        let num = |n: f64| Json::Number(n);
        let stops = l.stops.iter().map(|s| {
            let c = s.color;
            Json::Object(vec![
                ("x".into(), num(s.x)),
                ("y".into(), num(s.y)),
                (
                    "color".into(),
                    Json::Object(vec![
                        ("r".into(), num(c.r)),
                        ("g".into(), num(c.g)),
                        ("b".into(), num(c.b)),
                        ("a".into(), num(c.a)),
                    ]),
                ),
            ])
        });
        out.push(("stops".into(), Json::Array(stops.collect())));
        out.push(("intensity".into(), num(l.intensity)));
        out.push(("grain".into(), num(l.grain)));
        out.push(("texture".into(), Json::String(l.texture.name().into())));
        out.push(("translucency".into(), num(l.translucency)));
        if let Some(a) = l.angle {
            out.push(("angle".into(), num(a)));
        }
    }
    let Some(t) = theme else {
        return "null".into();
    };
    let mut out = Vec::new();
    look(&t.look, &mut out);
    out.push(("mode".into(), Json::String(t.mode.name().into())));
    if let Some(d) = &t.dark {
        let mut dark = Vec::new();
        look(d, &mut dark);
        out.push(("dark".into(), Json::Object(dark)));
    }
    json_stringify(&Json::Object(out), None)
}

/// `withOwnLooksSeeded` (`presetLook.ts:73`).
pub fn with_own_looks_seeded(profile: &SeededProfile) -> SeededProfile {
    let p = &profile.fields;
    let seen = |fields: PresetLookFields| SeededProfile {
        seen_theme: Some(p.theme.clone()),
        seen_color: Some(p.color),
        fields,
    };
    let Some(looks) = &p.preset_themes else {
        let preset_themes = p.theme.iter().map(|t| (p.color, t.clone())).collect();
        return seen(PresetLookFields {
            preset_themes: Some(preset_themes),
            ..p.clone()
        });
    };
    let shown = p.theme.as_ref();
    match &profile.seen_theme {
        None => return profile.clone(),
        Some(seen_theme) if theme_json(shown) == theme_json(seen_theme.as_ref()) => {
            return profile.clone();
        }
        Some(_) => {}
    }
    let picked_elsewhere = shown.is_none() && profile.seen_color.is_some_and(|c| c != p.color);
    if picked_elsewhere {
        return seen(p.clone());
    }
    seen(PresetLookFields {
        preset_themes: Some(with_look(looks, p.color, shown)),
        ..p.clone()
    })
}

/// `profilesWithOwnLooks` (`presetLook.ts:84`).
pub fn profiles_with_own_looks(
    profiles: &[(String, SeededProfile)],
) -> Vec<(String, SeededProfile)> {
    profiles
        .iter()
        .map(|(id, profile)| (id.clone(), with_own_looks_seeded(profile)))
        .collect()
}

/// `trackPress` (`presetLook.ts:87`): a slider press that reads values only once granted a track.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrackPress {
    armed: bool,
}

/// `trackPress` (`presetLook.ts:87`).
pub fn track_press() -> TrackPress {
    TrackPress::default()
}

impl TrackPress {
    /// `armed` (`presetLook.ts:90`).
    pub fn armed(&self) -> bool {
        self.armed
    }

    /// `grant` (`presetLook.ts:91`).
    pub fn grant(&mut self, track_pixels: f64) {
        self.armed = track_pixels > 0.0;
    }

    /// `at` (`presetLook.ts:94`).
    pub fn at(
        &self,
        page_x: f64,
        track_x: f64,
        track_pixels: f64,
        snap_to: Option<f64>,
    ) -> Option<f64> {
        if self.armed {
            track_value_at(page_x, track_x, track_pixels, snap_to)
        } else {
            None
        }
    }
}

/// `trackValueAt` (`presetLook.ts:100`): `None` is `null`; `snap_to` `None` is `undefined`.
pub fn track_value_at(
    page_x: f64,
    track_x: f64,
    track_pixels: f64,
    snap_to: Option<f64>,
) -> Option<f64> {
    if track_pixels.is_nan() || track_pixels <= 0.0 {
        return None;
    }
    let value = js_min(1.0, js_max(0.0, (page_x - track_x) / track_pixels));
    Some(match snap_to {
        None => value,
        Some(start) => snap_to_start(value, start, track_pixels),
    })
}
