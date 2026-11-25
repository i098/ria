// Ported from ardia apps/browser/src/lib/platformTerms.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The Mac's words in Ardia's text (Finder, Keychain, Dock) and the OS ids of reports.
//! The file exports only types and constants, so it has no bridge crate and no recorded cases.
//!
//! Not ported: `platformTerms.linux.ts` and `platformTerms.windows.ts` (the Linux and Windows
//! builds); this port is the Mac build only.
//!
//! Changes from Ardia: the strings are `&'static str`.

/// `PlatformTerms` (`platformTerms.ts:1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformTerms {
    pub computer: &'static str,
    pub computer_title: &'static str,
    pub os: &'static str,
    pub file_manager: &'static str,
    pub trash: &'static str,
    pub dock: &'static str,
    pub low_power_mode: &'static str,
    pub password_key: &'static str,
    pub device_unlock: &'static str,
    pub full_screen: &'static str,
    pub secret_store: &'static str,
    pub synced_computer: &'static str,
    pub cloud_drive: &'static str,
    pub turn_on_cloud_drive: &'static str,
    pub cloud_drive_settings_url: &'static str,
    pub login_items_settings: &'static str,
    pub system_contrast: &'static str,
    pub link_source_apps: &'static str,
    pub link_source_examples: &'static str,
    pub switch_tabs_key: &'static str,
}

/// `PlatformIds` (`platformTerms.ts:24`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformIds {
    pub report: &'static str,
    pub otel: &'static str,
}

/// `PLATFORM_IDS` (`platformTerms.ts:26`).
pub const PLATFORM_IDS: PlatformIds = PlatformIds {
    report: "macos",
    otel: "darwin",
};

/// `PLATFORM_TERMS` (`platformTerms.ts:28`).
pub const PLATFORM_TERMS: PlatformTerms = PlatformTerms {
    computer: "Mac",
    computer_title: "Mac",
    os: "macOS",
    file_manager: "Finder",
    trash: "Trash",
    dock: "Dock",
    low_power_mode: "Low Power Mode",
    password_key: "a key in your Keychain",
    device_unlock: "Touch ID or your login password",
    full_screen: "macOS full screen",
    secret_store: "your Keychain",
    synced_computer: "Mac",
    cloud_drive: "iCloud Drive",
    turn_on_cloud_drive: "Turn on iCloud Drive in System Settings",
    cloud_drive_settings_url: "x-apple.systempreferences:com.apple.preferences.AppleIDPrefPane",
    login_items_settings: "System Settings › General › Login Items",
    system_contrast: "macOS Increase Contrast",
    link_source_apps: "Mail, Slack, Notes",
    link_source_examples: "Discord, Slack or Mail",
    switch_tabs_key: "⌃⇥",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::platform_chrome::{POP_UP_CHROME, PopUpChrome, PopUpIndicator};
    use crate::shared::platform_features::PLATFORM_FEATURES;

    // `platformIds.test.mjs`: constants only, so no recorded case holds them.
    #[test]
    fn mac_ids_features_and_pop_up() {
        assert_eq!(
            PLATFORM_IDS,
            PlatformIds {
                report: "macos",
                otel: "darwin"
            }
        );
        const { assert!(PLATFORM_FEATURES.memory_pressure) };
        assert_eq!(
            POP_UP_CHROME,
            PopUpChrome {
                indicator: PopUpIndicator::AccentUpDown,
                grows_to_its_title: false
            }
        );
    }
}
