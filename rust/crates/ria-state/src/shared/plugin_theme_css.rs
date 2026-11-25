// Ported from ardia apps/browser/src/lib/pluginThemeCss.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The CSS a plugin page gets from the browser theme: the theme roles as `--ardia-*` variables
//! with safe values, and base styles that use them.
//!
//! Changes from Ardia:
//! - `unknown` is `Json` and `Record<string, unknown>` is `Fields`, read in `Object.entries` order (`js_order`).
//! - `scheme` is a `String`: any text but `"dark"` gives `light`, as in the TS.
//! - `\s` is the JS whitespace set, written out. `UNSAFE_WORDS` uses Unicode case folding where JS
//!   folds ASCII only; it runs only on text that `SAFE_VALUE` passed, which is ASCII and spaces, so
//!   the results are the same.

use regex::Regex;
use ria_core::identity::telemetry_wire::{Json, js_number_to_string, js_order};
use std::sync::LazyLock;

use super::decode::Fields;
use super::js::js_trim;

/// `PluginThemeRoles` (`pluginThemeCss.ts:1`).
#[derive(Debug, Clone, PartialEq)]
pub struct PluginThemeRoles {
    pub scheme: String,
    pub roles: Fields,
}

static ROLE_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[A-Za-z][A-Za-z0-9]{0,47}$").expect("static pattern"));
static SAFE_VALUE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[#A-Za-z0-9(),.%\t\n\x0B\f\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}/+-]{1,96}$")
        .expect("static pattern")
});
static UNSAFE_WORDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(url|expression|image|var|attr|env)[\t\n\x0B\f\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*\(")
        .expect("static pattern")
});
const MAX_ROLES: usize = 256;

/// `cssRoleName` (`pluginThemeCss.ts:8`): `--ardia-` and the role in kebab case, other characters dropped.
pub fn css_role_name(role: &str) -> String {
    let mut out = String::from("--ardia-");
    for ch in role.chars() {
        if ch.is_ascii_uppercase() {
            out.push('-');
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
            out.push(ch);
        }
    }
    out
}

/// `cssRoleValue` (`pluginThemeCss.ts:10`): a finite number, or safe text, as CSS.
pub fn css_role_value(value: &Json) -> Option<String> {
    match value {
        Json::Number(n) => n.is_finite().then(|| js_number_to_string(*n)),
        Json::String(s) => {
            let text = js_trim(s);
            (SAFE_VALUE.is_match(text) && !UNSAFE_WORDS.is_match(text)).then(|| text.to_owned())
        }
        _ => None,
    }
}

/// `pluginThemeVariables` (`pluginThemeCss.ts:18`): up to 256 `[name, value]` pairs.
pub fn plugin_theme_variables(roles: &Fields) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (role, value) in js_order(roles) {
        if out.len() >= MAX_ROLES {
            break;
        }
        if !ROLE_NAME.is_match(role) {
            continue;
        }
        if let Some(css) = css_role_value(value) {
            out.push((css_role_name(role), css));
        }
    }
    out
}

/// `fallback` (`pluginThemeCss.ts:29`).
fn fallback(role: &str, system: &str) -> String {
    format!("var({}, {system})", css_role_name(role))
}

/// `buildPluginThemeCss` (`pluginThemeCss.ts:31`).
pub fn build_plugin_theme_css(theme: &PluginThemeRoles) -> String {
    let scheme = if theme.scheme == "dark" {
        "dark"
    } else {
        "light"
    };
    let mut lines = vec![":root {".to_owned(), format!("  color-scheme: {scheme};")];
    lines.extend(
        plugin_theme_variables(&theme.roles)
            .into_iter()
            .map(|(name, value)| format!("  {name}: {value};")),
    );
    lines.extend([
        "}".to_owned(),
        "*, *::before, *::after { box-sizing: border-box; }".to_owned(),
        format!(
            "html, body {{ margin: 0; padding: 0; background: {}; color: {}; }}",
            fallback("background", "Canvas"),
            fallback("text", "CanvasText")
        ),
        "body { font: 13px/1.4 -apple-system, BlinkMacSystemFont, system-ui, sans-serif; -webkit-font-smoothing: antialiased; }".to_owned(),
        format!("a {{ color: {}; }}", fallback("accent", "LinkText")),
        format!("::selection {{ background: {}; }}", fallback("selection", "Highlight")),
        String::new(),
    ]);
    lines.join("\n")
}
