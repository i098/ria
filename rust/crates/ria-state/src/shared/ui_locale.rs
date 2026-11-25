// Ported from ardia apps/browser/src/lib/uiLocale.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The locale of the interface text: English, with the region of the system locale.
//!
//! Changes from Ardia: `UI_LOCALE` is a constant that reads the system locale with `Intl`. Rust's
//! standard library has no `Intl`, so `ui_locale` takes the system locale (a BCP 47 tag) from the caller.

use regex::Regex;
use std::sync::LazyLock;

// `\d` is ASCII only in a JS regex without the `u` flag.
static REGION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[A-Z]{2}|[0-9]{3})$").expect("static pattern"));

/// `UI_LOCALE` (`uiLocale.ts:1-3`): `en-<region>` for the first region subtag of `system_locale`, else `en-US`.
pub fn ui_locale(system_locale: &str) -> String {
    match system_locale
        .split('-')
        .skip(1)
        .find(|part| REGION.is_match(part))
    {
        Some(region) => format!("en-{region}"),
        None => "en-US".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::ui_locale;

    #[test]
    fn region_of_the_system_locale() {
        assert_eq!(ui_locale("en-GB"), "en-GB");
        assert_eq!(ui_locale("zh-Hans-CN"), "en-CN");
        assert_eq!(ui_locale("es-419"), "en-419");
        assert_eq!(ui_locale("fr"), "en-US");
        assert_eq!(ui_locale("de-de"), "en-US");
    }
}
