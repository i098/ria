// Ported from ardia packages/ardiacore/ios/ChromeTabCyclingPref.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Reads the tab cycling order from the `Preferences` file of a profile.

use std::path::Path;

use serde_json::Value;

/// Returns `None` if `Preferences` cannot be read as a JSON object.
/// Returns `Some(false)` if the object has no Boolean `helium.browser.mru_tab_cycling` value.
pub fn is_most_recent_first(profile_dir: &Path) -> Option<bool> {
    let data = std::fs::read(profile_dir.join("Preferences")).ok()?;
    let prefs: Value = serde_json::from_slice(&data)
        .ok()
        .filter(Value::is_object)?;
    Some(
        prefs["helium"]["browser"]["mru_tab_cycling"]
            .as_bool()
            .unwrap_or(false),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_flag_and_defaults_to_false() {
        let dir = std::env::temp_dir().join(format!("ria-cycle-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(is_most_recent_first(&dir), None);
        std::fs::write(dir.join("Preferences"), "[]").unwrap();
        assert_eq!(is_most_recent_first(&dir), None);
        std::fs::write(dir.join("Preferences"), "{}").unwrap();
        assert_eq!(is_most_recent_first(&dir), Some(false));
        std::fs::write(
            dir.join("Preferences"),
            r#"{"helium":{"browser":{"mru_tab_cycling":true}}}"#,
        )
        .unwrap();
        assert_eq!(is_most_recent_first(&dir), Some(true));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
