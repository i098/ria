# Pending sequences for `state/profiles` (slice 2.12)

`cases.jsonl` holds 111 sequences recorded from Ardia (`record-profiles.mjs`, Ardia `551d3c13c`). The test `recorded_ardia_sequences_match`
(`crates/ria-state/src/profiles/replay_tests.rs`) replays all of them; none reaches a `todo!()` stub.

Not recorded yet, because the Rust body calls a stub in `pending_siblings.rs`. Gate G2 (slice 2.49) records and runs them.

| Sequence | Blocked by |
| --- | --- |
| `createProfile` (id, bookmark roots) | `browser::newId`, `engineProfile` (2.5), `bookmarks::ensureRoots` (2.16) |
| `deleteProfile` (windows, tabs, pins, settings, bookmarks) | `closeWindow` (2.9), `handPagesOutOfProfile` (2.13), `settingsWith` (2.7), `withNewTab`, `activated`, `removeTabs`, `newTab` (2.8), `removeBookmarkTree` (2.16) |
| `setDefaultProfile` | `settingsWith` (2.7) |
| `setProfileTheme`, `choosePresetColor`, `resetPresetTheme` | `sync::adoptProfileTheme` (2.31), `telemetry::noteThemePreset` (2.6) |
| `importArcTheme` | `import::parseArcTheme` and platform calls (2.40) |
| `showProfileMenu`, `renameProfile`, result actions | `app::menuSections`, `app::chooseFromMenu` (2.10) |
| `openProfile`, `markProfileUnloaded`, `startProfileLoading`, `applyUnload`, `checkIdleProfiles`, unload results | `tabs::*` page helpers (2.8), `telemetry::noteProfileUnloaded`, `noteUnloadedProfileReopened` (2.6) |
| `profileDataDeleted` cleanup drain | `telemetry::noteProfileDataDeleted`, `warnFor` (2.6), `engines::forgetSiteDataProfile` (2.21) |

Stubs are listed with Ardia file and line in `pending_siblings.rs`.
