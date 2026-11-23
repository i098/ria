# Halyard port map

Source of truth for what is ported and where it goes. One row for each unit.

- TypeScript and JavaScript rows: `rust-anything analyze --json --lang ts` (rust-anything 3.3.0) run in `ardia/` at `551d3c13c`; 672 units. Column `Status` is its readiness (`ready`, `caution`, `keep`, `blocked`).
- Swift, Objective-C++ and C++ rows: made by hand with a script, because `analyze` lists only TypeScript and JavaScript here. One row for each file stem (`Foo.h`, `Foo.mm`, `Foo+Bar.mm`, `Foo.swift` are one unit). Column `Status` is `native`.
- The `plugin-broker` row is by hand: it is Rust and `analyze` does not list it.
- Scope `mac` is the first scope (ADR-C3). Scope `out` rows are listed so that every unit of `analyze` has a row, and are not ported.
- Targets are a first assignment from the port plan, section 7. They are drafts; a phase may move a unit. Column `Port state` starts as `todo` for scope `mac`.

Counts: 1255 rows, 814 in scope `mac`, 441 `out`.

| Scope | Unit | Language | Lines | Status | Target | Source | Port state |
| --- | --- | --- | ---: | --- | --- | --- | --- |
| mac | `apps/browser/assets/offline-game/` | TypeScript/JavaScript | 1045 | caution | assets, copied into the bundle | analyze | todo |
| mac | `apps/browser/assets/offline-game/assets/` | TypeScript/JavaScript | 3375 | caution | assets, copied into the bundle | analyze | todo |
| mac | `apps/browser/index.js` | TypeScript/JavaScript | 10 | ready | ardia-app | analyze | todo |
| mac | `apps/browser/macos/Ardia-macOS/AppDelegate.*` | Swift | 100 | native | ardia-app | hand | todo |
| mac | `apps/browser/macos/Ardia-macOS/BrowserWindow.*` | Swift | 61 | native | ardia-app | hand | todo |
| mac | `apps/browser/macos/Ardia-macOS/DisableFusebox.*` | Objective-C++ | 14 | native | ardia-app | hand | todo |
| mac | `apps/browser/macos/Ardia-macOS/main.*` | Swift | 18 | native | ardia-app | hand | todo |
| mac | `apps/browser/scripts/` | TypeScript/JavaScript | 9800 | blocked | stays JavaScript (drives the port through the socket) | analyze | - |
| mac | `apps/browser/scripts/perf/` | TypeScript/JavaScript | 9955 | blocked | stays JavaScript (drives the port through the socket) | analyze | - |
| mac | `apps/browser/scripts/sim/` | TypeScript/JavaScript | 1714 | blocked | stays JavaScript (drives the port through the socket) | analyze | - |
| mac | `apps/browser/src/App.tsx` | TypeScript/JavaScript | 127 | keep | ardia-app / domain crates (lib, platform, entry) | analyze | todo |
| mac | `apps/browser/src/components/bookmarks/` | TypeScript/JavaScript | 1276 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/dev/` | TypeScript/JavaScript | 292 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/downloads/` | TypeScript/JavaScript | 585 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/extensions/` | TypeScript/JavaScript | 731 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/favicon/` | TypeScript/JavaScript | 130 | keep | ardia-ui-kit | analyze | todo |
| mac | `apps/browser/src/components/find/` | TypeScript/JavaScript | 161 | keep | ardia-ui-overlays | analyze | todo |
| mac | `apps/browser/src/components/glyph/` | TypeScript/JavaScript | 77 | keep | ardia-ui-kit | analyze | todo |
| mac | `apps/browser/src/components/iconPicker/` | TypeScript/JavaScript | 313 | keep | ardia-ui-kit | analyze | todo |
| mac | `apps/browser/src/components/import/` | TypeScript/JavaScript | 410 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/import/steps/` | TypeScript/JavaScript | 604 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/littleArdia/` | TypeScript/JavaScript | 382 | keep | ardia-ui-window | analyze | todo |
| mac | `apps/browser/src/components/live/` | TypeScript/JavaScript | 694 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/media/` | TypeScript/JavaScript | 1356 | keep | ardia-ui-media | analyze | todo |
| mac | `apps/browser/src/components/ntp/` | TypeScript/JavaScript | 1014 | keep | ardia-ui-ntp | analyze | todo |
| mac | `apps/browser/src/components/omnibox/` | TypeScript/JavaScript | 2194 | keep | ardia-ui-omnibox | analyze | todo |
| mac | `apps/browser/src/components/onboarding/` | TypeScript/JavaScript | 3112 | keep | ardia-ui-onboarding | analyze | todo |
| mac | `apps/browser/src/components/onboarding/previews/` | TypeScript/JavaScript | 1226 | keep | ardia-ui-onboarding | analyze | todo |
| mac | `apps/browser/src/components/onboarding/previews/scenes/` | TypeScript/JavaScript | 831 | keep | ardia-ui-onboarding | analyze | todo |
| mac | `apps/browser/src/components/onboarding/tour/` | TypeScript/JavaScript | 239 | keep | ardia-ui-onboarding | analyze | todo |
| mac | `apps/browser/src/components/pages/` | TypeScript/JavaScript | 1249 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/peek/` | TypeScript/JavaScript | 422 | keep | ardia-ui-overlays | analyze | todo |
| mac | `apps/browser/src/components/plugins/` | TypeScript/JavaScript | 2061 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/profiles/` | TypeScript/JavaScript | 609 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/report/` | TypeScript/JavaScript | 545 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/settings/` | TypeScript/JavaScript | 2150 | keep | ardia-ui-settings | analyze | todo |
| mac | `apps/browser/src/components/settings/panes/` | TypeScript/JavaScript | 6330 | keep | ardia-ui-settings | analyze | todo |
| mac | `apps/browser/src/components/settings/rows/` | TypeScript/JavaScript | 1160 | ready | ardia-ui-settings | analyze | todo |
| mac | `apps/browser/src/components/sidebar/` | TypeScript/JavaScript | 8312 | keep | ardia-ui-sidebar | analyze | todo |
| mac | `apps/browser/src/components/site/` | TypeScript/JavaScript | 1702 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/split/` | TypeScript/JavaScript | 548 | keep | ardia-ui-window | analyze | todo |
| mac | `apps/browser/src/components/tabstrip/` | TypeScript/JavaScript | 2927 | keep | ardia-ui-tabstrip | analyze | todo |
| mac | `apps/browser/src/components/toolbar/` | TypeScript/JavaScript | 1451 | keep | ardia-ui-toolbar | analyze | todo |
| mac | `apps/browser/src/components/ui/` | TypeScript/JavaScript | 4031 | keep | ardia-ui-kit | analyze | todo |
| mac | `apps/browser/src/components/unread/` | TypeScript/JavaScript | 96 | keep | ardia-ui-panels | analyze | todo |
| mac | `apps/browser/src/components/window/` | TypeScript/JavaScript | 3306 | keep | ardia-ui-window | analyze | todo |
| mac | `apps/browser/src/lib/` | TypeScript/JavaScript | 6174 | caution | ardia-app / domain crates (lib, platform, entry) | analyze | todo |
| mac | `apps/browser/src/main.tsx` | TypeScript/JavaScript | 102 | keep | ardia-app / domain crates (lib, platform, entry) | analyze | todo |
| mac | `apps/browser/src/platform/` | TypeScript/JavaScript | 1752 | keep | ardia-app / domain crates (lib, platform, entry) | analyze | todo |
| mac | `apps/browser/src/state/airTraffic/` | TypeScript/JavaScript | 478 | keep | ardia-app | analyze | todo |
| mac | `apps/browser/src/state/app/` | TypeScript/JavaScript | 1637 | keep | ardia-settings | analyze | todo |
| mac | `apps/browser/src/state/automation/` | TypeScript/JavaScript | 7311 | keep | ardia-automation | analyze | todo |
| mac | `apps/browser/src/state/bookmarks/` | TypeScript/JavaScript | 1854 | caution | ardia-library | analyze | todo |
| mac | `apps/browser/src/state/browser/` | TypeScript/JavaScript | 1357 | keep | ardia-settings | analyze | todo |
| mac | `apps/browser/src/state/commands/` | TypeScript/JavaScript | 1934 | keep | ardia-settings | analyze | todo |
| mac | `apps/browser/src/state/config/` | TypeScript/JavaScript | 620 | caution | ardia-settings | analyze | todo |
| mac | `apps/browser/src/state/downloads/` | TypeScript/JavaScript | 245 | keep | ardia-library | analyze | todo |
| mac | `apps/browser/src/state/drag/` | TypeScript/JavaScript | 486 | ready | ardia-windows | analyze | todo |
| mac | `apps/browser/src/state/engines/` | TypeScript/JavaScript | 1905 | caution | ardia-settings | analyze | todo |
| mac | `apps/browser/src/state/extensions/` | TypeScript/JavaScript | 1479 | keep | ardia-extensions | analyze | todo |
| mac | `apps/browser/src/state/favicons/` | TypeScript/JavaScript | 686 | keep | ardia-library | analyze | todo |
| mac | `apps/browser/src/state/find/` | TypeScript/JavaScript | 27 | ready | ardia-library | analyze | todo |
| mac | `apps/browser/src/state/focus/` | TypeScript/JavaScript | 145 | ready | ardia-windows | analyze | todo |
| mac | `apps/browser/src/state/groups/` | TypeScript/JavaScript | 847 | caution | ardia-tabs | analyze | todo |
| mac | `apps/browser/src/state/history/` | TypeScript/JavaScript | 567 | caution | ardia-library | analyze | todo |
| mac | `apps/browser/src/state/import/` | TypeScript/JavaScript | 1631 | caution | ardia-import | analyze | todo |
| mac | `apps/browser/src/state/littleArdia/` | TypeScript/JavaScript | 299 | ready | ardia-windows | analyze | todo |
| mac | `apps/browser/src/state/live/` | TypeScript/JavaScript | 2468 | keep | ardia-app | analyze | todo |
| mac | `apps/browser/src/state/media/` | TypeScript/JavaScript | 595 | keep | ardia-app | analyze | todo |
| mac | `apps/browser/src/state/omnibox/` | TypeScript/JavaScript | 1428 | keep | ardia-core | analyze | todo |
| mac | `apps/browser/src/state/onboarding/` | TypeScript/JavaScript | 1121 | caution | ardia-settings | analyze | todo |
| mac | `apps/browser/src/state/pages/` | TypeScript/JavaScript | 90 | ready | ardia-library | analyze | todo |
| mac | `apps/browser/src/state/peek/` | TypeScript/JavaScript | 446 | keep | ardia-app | analyze | todo |
| mac | `apps/browser/src/state/pins/` | TypeScript/JavaScript | 2798 | caution | ardia-tabs | analyze | todo |
| mac | `apps/browser/src/state/plugins/` | TypeScript/JavaScript | 4386 | keep | ardia-plugins | analyze | todo |
| mac | `apps/browser/src/state/presets/` | TypeScript/JavaScript | 229 | ready | ardia-profiles | analyze | todo |
| mac | `apps/browser/src/state/profiles/` | TypeScript/JavaScript | 752 | caution | ardia-profiles | analyze | todo |
| mac | `apps/browser/src/state/reader/` | TypeScript/JavaScript | 599 | ready | ardia-library | analyze | todo |
| mac | `apps/browser/src/state/releaseNotes/` | TypeScript/JavaScript | 111 | ready | ardia-telemetry | analyze | todo |
| mac | `apps/browser/src/state/report/` | TypeScript/JavaScript | 1021 | caution | ardia-telemetry | analyze | todo |
| mac | `apps/browser/src/state/session/` | TypeScript/JavaScript | 752 | caution | ardia-windows | analyze | todo |
| mac | `apps/browser/src/state/settings/` | TypeScript/JavaScript | 1155 | keep | ardia-settings | analyze | todo |
| mac | `apps/browser/src/state/sharedFolders/` | TypeScript/JavaScript | 1108 | caution | ardia-sync | analyze | todo |
| mac | `apps/browser/src/state/sidebar/` | TypeScript/JavaScript | 3282 | keep | ardia-sidebar | analyze | todo |
| mac | `apps/browser/src/state/site/` | TypeScript/JavaScript | 3154 | caution | ardia-sites | analyze | todo |
| mac | `apps/browser/src/state/splits/` | TypeScript/JavaScript | 525 | caution | ardia-tabs | analyze | todo |
| mac | `apps/browser/src/state/sync/` | TypeScript/JavaScript | 3809 | caution | ardia-sync | analyze | todo |
| mac | `apps/browser/src/state/tabs/` | TypeScript/JavaScript | 4327 | keep | ardia-tabs | analyze | todo |
| mac | `apps/browser/src/state/telemetry/` | TypeScript/JavaScript | 7618 | caution | ardia-telemetry | analyze | todo |
| mac | `apps/browser/src/state/windows/` | TypeScript/JavaScript | 2446 | keep | ardia-windows | analyze | todo |
| mac | `apps/browser/src/test-hooks.mjs` | TypeScript/JavaScript | 144 | keep | ardia-app / domain crates (lib, platform, entry) | analyze | todo |
| mac | `apps/browser/src/test-loader.mjs` | TypeScript/JavaScript | 52 | ready | ardia-app / domain crates (lib, platform, entry) | analyze | todo |
| mac | `apps/browser/src/test-native/` | TypeScript/JavaScript | 1744 | keep | ardia-app / domain crates (lib, platform, entry) | analyze | todo |
| mac | `engine/ardiacore/src/ardia/core/ac_app_urls.*` | Objective-C++ | 125 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_autofill_bubble_handler.*` | Objective-C++ | 114 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_autofill_prompt.*` | Objective-C++ | 475 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_autofill_trigger.*` | Objective-C++ | 180 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_browser.*` | Objective-C++ | 571 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_browser_internal.*` | Objective-C++ | 14 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_browser_window.*` | Objective-C++ | 824 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_browser_window_exclusive_access.*` | Objective-C++ | 224 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_browser_window_keyboard.*` | Objective-C++ | 106 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_cast_dialog.*` | Objective-C++ | 359 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_content_hooks.*` | Objective-C++ | 171 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_contents_background.*` | Objective-C++ | 243 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_context_menu.*` | Objective-C++ | 486 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_context_menu_share.*` | Objective-C++ | 239 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_desktop_capture.*` | Objective-C++ | 213 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_device_chooser.*` | Objective-C++ | 386 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_devtools_call.*` | Objective-C++ | 216 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_extension_drop.*` | Objective-C++ | 94 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_extension_shortcuts.*` | Objective-C++ | 332 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_extension_view.*` | Objective-C++ | 459 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_external_apps.*` | Objective-C++ | 221 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_fake_media.*` | Objective-C++ | 68 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_find_bar.*` | Objective-C++ | 52 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_hooks.*` | Objective-C++ | 194 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_host_visibility.*` | Objective-C++ | 205 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_icon_fetch.*` | Objective-C++ | 171 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_installed_bubble.*` | Objective-C++ | 145 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_launch_switches.*` | C++/Objective-C++ | 250 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_lifetime.*` | Objective-C++ | 423 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_location_bar.*` | Objective-C++ | 88 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_main_delegate.*` | C++/Objective-C++ | 454 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_modal_dialog_host.*` | Objective-C++ | 225 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_navigation_hold.*` | Objective-C++ | 222 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_page_channel.*` | Objective-C++ | 537 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_page_channel_held.*` | Objective-C++ | 116 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_page_limits.*` | Objective-C++ | 20 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_page_script.*` | C++/Objective-C++ | 63 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_page_view.*` | Objective-C++ | 148 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_password_prompt.*` | Objective-C++ | 146 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_permissions.*` | Objective-C++ | 465 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_picture_in_picture.*` | Objective-C++ | 377 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_prerender_posts.*` | Objective-C++ | 79 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_safe_storage.*` | Objective-C++ | 393 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_seams.*` | Objective-C++ | 205 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_strings.*` | C++/Objective-C++ | 134 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_tab_bridge.*` | Objective-C++ | 433 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_tab_bridge_media.*` | Objective-C++ | 95 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_tab_bridge_navigation.*` | Objective-C++ | 233 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_tab_info.*` | Objective-C++ | 196 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_test_media_route_provider.*` | C++/Objective-C++ | 176 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_test_system_permissions.*` | Objective-C++ | 119 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_video_upscaler.*` | Objective-C++ | 329 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_video_upscaler_color.*` | Objective-C++ | 120 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_video_upscaler_internal.*` | Objective-C++ | 183 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_video_upscaler_job.*` | Objective-C++ | 432 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_video_upscaler_kit.*` | Objective-C++ | 332 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_video_upscaling.*` | Objective-C++ | 307 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_web_contents_delegate.*` | Objective-C++ | 195 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_window_host.*` | Objective-C++ | 370 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_window_host_close.*` | Objective-C++ | 281 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_window_host_dialogs.*` | Objective-C++ | 101 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_window_host_tabs.*` | Objective-C++ | 159 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ac_windowless_page.*` | Objective-C++ | 165 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_api_internal.*` | Objective-C++ | 24 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_engine.*` | Objective-C++ | 427 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_engine_idle_profiles.*` | Objective-C++ | 212 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_engine_tracing.*` | Objective-C++ | 150 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_hooks.*` | Objective-C++ | 187 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_internal.*` | Objective-C++ | 42 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_profile.*` | Objective-C++ | 361 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_tab.*` | Objective-C++ | 512 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_tab_commands.*` | Objective-C++ | 317 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_tab_extensions.*` | Objective-C++ | 133 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_tab_internal.*` | Objective-C++ | 26 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_tab_media.*` | Objective-C++ | 272 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_tab_navigation.*` | Objective-C++ | 118 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_tab_page.*` | Objective-C++ | 171 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_window.*` | Objective-C++ | 380 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/ardiacore_window_picture.*` | Objective-C++ | 45 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/public/ArdiaCore.*` | Objective-C++ | 417 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `engine/ardiacore/src/ardia/core/renderer/ac_content_renderer_client.*` | C++/Objective-C++ | 529 | native | stays non-Rust: engine side of the interface (ADR-C5) | hand | - |
| mac | `packages/ardiacore/ios/` | TypeScript/JavaScript | 2384 | caution | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/ios/ArdiaActivation.*` | Objective-C++ | 512 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaChromeWindowHost.*` | Objective-C++ | 1236 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaChromeWindowInternal.*` | Objective-C++ | 74 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaContentBlocker.*` | Objective-C++ | 571 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaContentBlockerCalls.*` | Objective-C++ | 295 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaContentBlockerInternal.*` | Objective-C++ | 37 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaContentBlockerLite.*` | Objective-C++ | 142 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaContentBlockerPage.*` | Objective-C++ | 106 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaDisplayIdentity.*` | Objective-C++ | 57 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaDisplayPlacementMath.*` | Objective-C++ | 43 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaDownloadMover.*` | Objective-C++ | 511 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaEarlyWindow.*` | Objective-C++ | 857 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaEarlyWindowCovers.*` | Objective-C++ | 226 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaEarlyWindowInternal.*` | Objective-C++ | 88 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaEarlyWindowLayouts.*` | Objective-C++ | 261 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaEngineBridge.*` | Objective-C++ | 224 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaEvents.*` | Objective-C++ | 96 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaExtensionPackage.*` | Objective-C++ | 199 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaFavicons.*` | Objective-C++ | 128 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaFieldTiming.*` | Objective-C++ | 160 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaFocusState.*` | Objective-C++ | 256 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaFreezeArming.*` | Objective-C++ | 70 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaFreezeCapture.*` | Objective-C++ | 579 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaFreezeWatch.*` | Objective-C++ | 332 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaHost.*` | Objective-C++ | 1250 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaHostEngineDelegate.*` | Objective-C++ | 218 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaHostInternal.*` | Objective-C++ | 33 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaHoverProbe.*` | Objective-C++ | 748 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaHoverProbeInternal.*` | Objective-C++ | 76 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaInterfaceScale.*` | Objective-C++ | 254 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaInternal.*` | Objective-C++ | 100 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaLaunchConfig.*` | Objective-C++ | 381 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaLaunchPicture.*` | Objective-C++ | 335 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaLaunchSequence.*` | Objective-C++ | 102 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaLaunchTabHolder.*` | Objective-C++ | 176 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaLittleFitMath.*` | Objective-C++ | 87 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaNativeMessaging.*` | Objective-C++ | 137 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaNavigationDownloads.*` | Objective-C++ | 79 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaNowPlaying.*` | Objective-C++ | 124 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPageCover.*` | Objective-C++ | 125 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPageCursor.*` | Objective-C++ | 224 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPageDrawWait.*` | Objective-C++ | 67 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPaintProbe.*` | Objective-C++ | 141 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPaintProbeMath.*` | Objective-C++ | 69 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPiPController.*` | Objective-C++ | 1402 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPiPHandle.*` | Objective-C++ | 63 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPiPOverlay.*` | Objective-C++ | 39 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPiPResizeCursor.*` | Objective-C++ | 243 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPiPSeekMath.*` | Objective-C++ | 22 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPiPThrowMath.*` | Objective-C++ | 310 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPiPZoom.*` | Objective-C++ | 519 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPiPZoomMath.*` | Objective-C++ | 83 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPictureInPicture.*` | Objective-C++ | 503 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPictureInPictureInternal.*` | Objective-C++ | 218 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPixelGridModule.*` | Objective-C++ | 134 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPointerTrace.*` | Objective-C++ | 123 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPopupOriginStrip.*` | Objective-C++ | 106 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaPopupWindow.*` | Objective-C++ | 496 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaProfiles.*` | Objective-C++ | 242 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaRendererScrollObserver.*` | Objective-C++ | 87 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaReservedKeys.*` | Objective-C++ | 174 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaRevealGuards.*` | Objective-C++ | 160 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaRevealLedger.*` | Objective-C++ | 111 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaRuntime.*` | Objective-C++ | 42 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaSavedDocument.*` | Objective-C++ | 35 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaSavedSession.*` | Objective-C++ | 120 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaServices.*` | Objective-C++ | 566 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaStackSample.*` | Objective-C++ | 35 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaStartup.*` | Objective-C++ | 72 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaSwipe.*` | Objective-C++ | 648 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaSwipeHitTesting.*` | Objective-C++ | 149 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaSwipeInternal.*` | Objective-C++ | 66 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaTabLifecycle.*` | Objective-C++ | 53 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaTabParking.*` | Objective-C++ | 336 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaTabPresses.*` | Objective-C++ | 136 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaTabRegistry.*` | Objective-C++ | 185 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaTabStrip.*` | Objective-C++ | 279 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaTrafficLights.*` | Objective-C++ | 543 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaTransferLedger.*` | Objective-C++ | 136 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaWebView.*` | Objective-C++/Swift | 6535 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaWebViewInternal.*` | Objective-C++ | 431 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaWindowController.*` | Objective-C++ | 428 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaWindowFullScreen.*` | Objective-C++ | 575 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ArdiaWindowSightMath.*` | Objective-C++ | 55 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/AutofillModule.*` | Swift | 36 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/BookmarksModule.*` | Swift | 37 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ChromeTabCyclingPref.*` | Swift | 12 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ChromiumTabView.*` | Swift | 19 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ContentBlockerModule.*` | Swift | 57 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/DownloadsModule.*` | Swift | 32 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/EngineCall.*` | Swift | 137 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/EngineModule.*` | Swift | 62 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ExtensionsModule.*` | Swift | 62 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/FaviconsModule.*` | Swift | 15 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/HistoryModule.*` | Swift | 19 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/InputSourceKeys.*` | Swift | 33 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/MediaModule.*` | Swift | 63 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/NSImage.*` | Swift | 27 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/Origins.*` | Swift | 23 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/PageEngine.*` | Swift | 168 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/PageEvents.*` | Swift | 10 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/PageTab.*` | Swift | 72 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/PageTabHost.*` | Swift | 43 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/PageViewDefinition.*` | Swift | 129 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/Pager.*` | Swift | 427 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/PagerConfig.*` | Swift | 10 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/PasswordsModule.*` | Swift | 79 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/PermissionsModule.*` | Swift | 35 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ProfileData.*` | Swift | 101 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/ProfilesModule.*` | Swift | 31 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/RCTTouchHandler.*` | Objective-C++ | 25 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/RCTUIImageView.*` | Objective-C++ | 8 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/RGBA.*` | Swift | 22 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/SearchModule.*` | Swift | 19 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/SitesModule.*` | Swift | 57 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/SwipeArea.*` | Swift | 54 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/SwipeModule.*` | Swift | 66 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/SystemState.*` | Swift | 67 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/TabsModule.*` | Swift | 73 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitAutoPictureInPicture.*` | Swift | 29 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitAutofill.*` | Swift | 637 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitAutofillDropdown.*` | Swift | 276 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitAutofillScript.*` | Swift | 446 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitCertificatePolicy.*` | Swift | 126 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitContentBlocker.*` | Swift | 575 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitContentBlockerScripts.*` | Swift | 203 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitContentBlockerStrict.*` | Swift | 136 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitContentBlockerUserFilters.*` | Swift | 369 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitContextMenu.*` | Swift | 477 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitCookieImport.*` | Swift | 123 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitDownloadNames.*` | Swift | 23 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitDownloads.*` | Swift | 348 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitExternalApps.*` | Swift | 95 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitFraudWarnings.*` | Swift | 13 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitHub.*` | Swift | 224 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitLogins.*` | Swift | 510 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitMainFrameTarget.*` | Swift | 19 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitMediaFaults.*` | Swift | 212 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitModule.*` | Swift | 76 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitNotifications.*` | Swift | 649 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitPageDialogs.*` | Swift | 96 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitPagePresses.*` | Swift | 36 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitPageScript.*` | Swift | 1180 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitPasskeys.*` | Swift | 31 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitPasswordGate.*` | Swift | 23 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitPermissions.*` | Swift | 229 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitPreloadedHSTS.*` | Swift | 59 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitPrivacyGates.*` | Swift | 43 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitRulePacker.*` | Swift | 113 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitSiteData.*` | Swift | 180 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitSiteStorage.*` | Swift | 196 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitStoreRemovals.*` | Swift | 27 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitStores.*` | Swift | 166 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitStrictProceed.*` | Swift | 13 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitTab.*` | Swift | 3561 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitTabView.*` | Swift | 20 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitTransfers.*` | Swift | 108 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebKitWebView.*` | Swift | 141 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WebViewModule.*` | Swift | 46 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/ios/WindowsModule.*` | Swift | 19 | native | ardia-host-mac (objc2); delete if Rune gives it | hand | todo |
| mac | `packages/ardiacore/scripts/` | TypeScript/JavaScript | 8911 | caution | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/WebView.tsx` | TypeScript/JavaScript | 115 | keep | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/autofill.ts` | TypeScript/JavaScript | 32 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/bookmarks.ts` | TypeScript/JavaScript | 6 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/contentBlocker.ts` | TypeScript/JavaScript | 36 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/downloads.ts` | TypeScript/JavaScript | 19 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/engine/` | TypeScript/JavaScript | 319 | caution | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/engineDomain.ts` | TypeScript/JavaScript | 98 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/extensions.ts` | TypeScript/JavaScript | 148 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/favicons.ts` | TypeScript/JavaScript | 14 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/history.ts` | TypeScript/JavaScript | 77 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/index.ts` | TypeScript/JavaScript | 24 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/media.ts` | TypeScript/JavaScript | 38 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/nativeModules.ts` | TypeScript/JavaScript | 29 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/pageEvents.ts` | TypeScript/JavaScript | 230 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/pageTypes.ts` | TypeScript/JavaScript | 312 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/passwords.ts` | TypeScript/JavaScript | 15 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/permissions.ts` | TypeScript/JavaScript | 21 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/profiles.ts` | TypeScript/JavaScript | 1 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/siteSettings.ts` | TypeScript/JavaScript | 8 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/swipe.tsx` | TypeScript/JavaScript | 101 | keep | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/tabs.ts` | TypeScript/JavaScript | 53 | caution | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/webkit.ts` | TypeScript/JavaScript | 108 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/ardiacore/src/windows.ts` | TypeScript/JavaScript | 22 | ready | ardia-engine / ardia-host-mac | analyze | todo |
| mac | `packages/core/scripts/` | TypeScript/JavaScript | 39 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/anonymousIdentity.ts` | TypeScript/JavaScript | 86 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/appUrls.ts` | TypeScript/JavaScript | 52 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/bangList.ts` | TypeScript/JavaScript | 10706 | caution | ardia-core | analyze | todo |
| mac | `packages/core/src/bangs.ts` | TypeScript/JavaScript | 210 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/calculator.ts` | TypeScript/JavaScript | 293 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/candidatePool.ts` | TypeScript/JavaScript | 172 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/cleanUrl.ts` | TypeScript/JavaScript | 68 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/engines.ts` | TypeScript/JavaScript | 158 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/errors.ts` | TypeScript/JavaScript | 12 | caution | ardia-core | analyze | todo |
| mac | `packages/core/src/extensionRows.ts` | TypeScript/JavaScript | 25 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/fuzzy.reference.ts` | TypeScript/JavaScript | 42 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/fuzzy.ts` | TypeScript/JavaScript | 87 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/idn.ts` | TypeScript/JavaScript | 323 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/index.ts` | TypeScript/JavaScript | 24 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/ingestKeys.ts` | TypeScript/JavaScript | 14 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/mobile.ts` | TypeScript/JavaScript | 121 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/omnibox.ts` | TypeScript/JavaScript | 182 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/openTabs.ts` | TypeScript/JavaScript | 10 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/pageMatch.ts` | TypeScript/JavaScript | 109 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/pageText.ts` | TypeScript/JavaScript | 55 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/providerConfig.ts` | TypeScript/JavaScript | 122 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/providerNetwork.ts` | TypeScript/JavaScript | 90 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/providerRows.ts` | TypeScript/JavaScript | 122 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/quickCreate.ts` | TypeScript/JavaScript | 38 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/siteSearch.ts` | TypeScript/JavaScript | 123 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/suggest.reference.ts` | TypeScript/JavaScript | 470 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/suggest.ts` | TypeScript/JavaScript | 217 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/suggestFetcher.ts` | TypeScript/JavaScript | 73 | caution | ardia-core | analyze | todo |
| mac | `packages/core/src/suggestion.ts` | TypeScript/JavaScript | 50 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/suggestionRows.ts` | TypeScript/JavaScript | 64 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/tabGroups.ts` | TypeScript/JavaScript | 2 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/telemetryWire.ts` | TypeScript/JavaScript | 67 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/textFragment.ts` | TypeScript/JavaScript | 73 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/theme.ts` | TypeScript/JavaScript | 291 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/tlds.ts` | TypeScript/JavaScript | 73 | ready | ardia-core | analyze | todo |
| mac | `packages/core/src/webStore.ts` | TypeScript/JavaScript | 71 | ready | ardia-core | analyze | todo |
| mac | `packages/import/ios/Core/AppIcons.*` | Swift | 39 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ArcExtras.*` | Swift | 211 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ArcSidebar.*` | Swift | 316 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ChromiumAutofill.*` | Swift | 142 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ChromiumBookmarks.*` | Swift | 60 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ChromiumCrypto.*` | Swift | 152 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ChromiumExtensions.*` | Swift | 122 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ChromiumFavicons.*` | Swift | 88 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ChromiumSearchEngine.*` | Swift | 48 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ChromiumSecrets.*` | Swift | 150 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ChromiumSessions.*` | Swift | 315 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/DiaAppleEvents.*` | Swift | 306 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/DiaSidebar.*` | Swift | 442 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/DiaTabs.*` | Swift | 140 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/Discovery.*` | Swift | 380 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/Firefox.*` | Swift | 249 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/FirefoxFavicons.*` | Swift | 288 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/FirefoxLogins.*` | Swift | 319 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/HistoryReader.*` | Swift | 74 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/Importer.*` | Swift | 317 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/Model.*` | Swift | 421 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/NetscapeBookmarks.*` | Swift | 183 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/PasswordsCSV.*` | Swift | 105 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/SQLiteSnapshot.*` | Swift | 208 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/SafariCookies.*` | Swift | 135 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/SafariDirect.*` | Swift | 147 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/SafariExport.*` | Swift | 198 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/SecretBytes.*` | Swift | 137 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/Wire.*` | Swift | 170 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ZenSidebar.*` | Swift | 302 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/Core/ZipArchive.*` | Swift | 88 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/ImportFileKind.*` | Swift | 5 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/ImportKind.*` | Swift | 3 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/ImportModule.*` | Swift | 205 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/ImportRequestOptions.*` | Swift | 13 | native | ardia-import | hand | todo |
| mac | `packages/import/ios/ImportWriter.*` | Swift | 104 | native | ardia-import | hand | todo |
| mac | `packages/import/scripts/` | TypeScript/JavaScript | 183 | ready | ardia-import | analyze | todo |
| mac | `packages/import/src/index.ts` | TypeScript/JavaScript | 282 | ready | ardia-import | analyze | todo |
| mac | `packages/plugin-broker/` | Rust | 9945 | already Rust | moves into the workspace as it is | hand | todo |
| mac | `packages/plugin-sdk/src/api.ts` | TypeScript/JavaScript | 219 | caution | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/componentProps.ts` | TypeScript/JavaScript | 151 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/components.ts` | TypeScript/JavaScript | 63 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/element.ts` | TypeScript/JavaScript | 82 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/fiber.ts` | TypeScript/JavaScript | 51 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/hooks.ts` | TypeScript/JavaScript | 105 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/index.ts` | TypeScript/JavaScript | 78 | caution | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/jsx.ts` | TypeScript/JavaScript | 37 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/jsxRuntime.ts` | TypeScript/JavaScript | 13 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/protocol.ts` | TypeScript/JavaScript | 222 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/quickPick.typecheck.ts` | TypeScript/JavaScript | 18 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/reconciler.ts` | TypeScript/JavaScript | 357 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/rpc.ts` | TypeScript/JavaScript | 85 | caution | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/runtime.ts` | TypeScript/JavaScript | 364 | caution | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/surfaces.ts` | TypeScript/JavaScript | 110 | ready | ardia-plugins | analyze | todo |
| mac | `packages/plugin-sdk/src/types.ts` | TypeScript/JavaScript | 83 | caution | ardia-plugins | analyze | todo |
| mac | `packages/shaders/ios/ArcGradientLayer.*` | Swift | 106 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/EarlyBackdrop.*` | Swift | 40 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/GrainLayer.*` | Swift | 84 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/MeadowMessages.*` | Swift | 35 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/MeadowModule.*` | Swift | 19 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/MeadowPoster.*` | Swift | 41 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/MeadowView.*` | Swift | 186 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/MetalSurface.*` | Swift | 207 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/PowerUpView.*` | Swift | 436 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/ShadersModule.*` | Swift | 86 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/ThemeGradient.*` | Swift | 27 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/ThemeSwatchView.*` | Swift | 89 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/WindowBackdropView.*` | Swift | 157 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/ios/WindowTreatmentView.*` | Swift | 147 | native | ardia-shaders | hand | todo |
| mac | `packages/shaders/scripts/` | TypeScript/JavaScript | 196 | blocked | ardia-shaders | analyze | todo |
| mac | `packages/shaders/src/index.tsx` | TypeScript/JavaScript | 93 | keep | ardia-shaders | analyze | todo |
| mac | `packages/shaders/src/meadow.tsx` | TypeScript/JavaScript | 10 | keep | ardia-shaders | analyze | todo |
| mac | `packages/shaders/src/meadow.windows.tsx` | TypeScript/JavaScript | 39 | keep | ardia-shaders | analyze | todo |
| mac | `packages/shaders/src/meadowPage/` | TypeScript/JavaScript | 256 | ready | ardia-shaders | analyze | todo |
| mac | `packages/shaders/windows/meadow/` | TypeScript/JavaScript | 4 | ready | ardia-shaders | analyze | todo |
| mac | `packages/shell/ios/ActivitySpinner.*` | Swift | 71 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ActivitySpinnerModule.*` | Swift | 11 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/AppIcons.*` | Swift | 126 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/AppKitPatches.*` | Swift | 121 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/AppModule.*` | Swift | 153 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/AppUpdater.*` | Swift | 348 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/AppWindowsModule.*` | Swift | 113 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ArdiaHermesFatal.*` | Objective-C++ | 50 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ArdiaIsolation.*` | Objective-C++ | 52 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ArdiaPeekMesh.*` | Objective-C++ | 262 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ArdiaTryCatch.*` | Objective-C++ | 17 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/AutomationModule.*` | Swift | 34 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/BangListFetch.*` | Swift | 100 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/BlurredIcon.*` | Swift | 41 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/CalendarModule.*` | Swift | 150 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/CenteredResize.*` | Swift | 42 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ChannelUserDriver.*` | Swift | 159 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ChromeWindows.*` | Swift | 45 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/CommandItem.*` | Swift | 74 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/CommandTable.*` | Swift | 336 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ConfigFolder.*` | Swift | 93 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ConfigModule.*` | Swift | 21 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ConfigWatcher.*` | Swift | 168 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ConfirmOptions.*` | Swift | 14 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ContextMenuArea.*` | Swift | 111 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ContextMenuAreaModule.*` | Swift | 15 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ControlConnection.*` | Swift | 247 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ControlOutput.*` | Swift | 221 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ControlSocket.*` | Swift | 231 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/CrashReports.*` | Swift | 343 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DevDragSession.*` | Swift | 113 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DevEngineNotificationDelegate.*` | Swift | 7 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DevFreeze.*` | Swift | 14 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DevModule.*` | Swift | 499 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DialogsModule.*` | Swift | 118 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DockSelection.*` | Swift | 213 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DockSelectionModule.*` | Swift | 37 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DockTile.*` | Swift | 48 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DocumentStore.*` | Swift | 132 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DocumentsModule.*` | Swift | 21 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DownloadPrompt.*` | Swift | 45 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DragPreview.*` | Swift | 367 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/DroppedLinks.*` | Swift | 85 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FadeLabel.*` | Swift | 181 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FadeLabelModule.*` | Swift | 154 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FieldGlow.*` | Swift | 73 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FieldGlowCore/FieldGlowGeometry.*` | Swift | 102 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FieldGlowCore/FieldGlowHub.*` | Swift | 74 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FieldGlowCore/FieldGlowLayers.*` | Swift | 330 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FieldGlowCore/FieldGlowModel.*` | Swift | 136 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FieldGlowModule.*` | Swift | 39 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FileDragModule.*` | Swift | 15 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FileDragView.*` | Swift | 52 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FileDropModule.*` | Swift | 36 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FileDropView.*` | Swift | 89 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FocusHandoff.*` | Swift | 476 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FocusModule.*` | Swift | 131 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FolderArt.*` | Swift | 82 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FolderIconModule.*` | Swift | 18 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FolderIconView.*` | Swift | 114 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FolderPalette.*` | Swift | 8 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FullScreenLifecycle.*` | Swift | 117 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/FullScreenRestore.*` | Swift | 77 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/GlassEffect.*` | Swift | 123 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/GlassEffectModule.*` | Swift | 19 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/GlassGlow.*` | Swift | 44 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/GlassGroup.*` | Swift | 117 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/GlassGroupModule.*` | Swift | 11 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/GzipData.*` | Swift | 36 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/Handoff.*` | Swift | 33 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/HapticLog.*` | Swift | 30 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/HermesFatal.*` | Swift | 28 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/IconAccent.*` | Swift | 75 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/IconSource.*` | Swift | 45 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/IconTheme.*` | Swift | 78 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/InlineCompletionModule.*` | Swift | 57 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/IntroMusic.*` | Swift | 355 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/KeepOpenMenuRow.*` | Swift | 81 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/KeyEquivalentMatch.*` | Swift | 26 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/KeyEquivalents.*` | Swift | 319 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/KeychainModule.*` | Swift | 120 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/LeavingOverPiP.*` | Swift | 50 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/LinkActivationOrder.*` | Swift | 333 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/LiquidGlass.*` | Swift | 25 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/LittleArdiaHandoff.*` | Swift | 122 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/LittleArdiaHotKey.*` | Swift | 177 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/LittleLandingFit.*` | Swift | 51 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/LocationAccess.*` | Swift | 80 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/LocationModule.*` | Swift | 20 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MagnifyArea.*` | Swift | 78 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MagnifyAreaModule.*` | Swift | 22 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MainMenu.*` | Swift | 746 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ManagedWindow.*` | Swift | 88 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuBookmark.*` | Swift | 8 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuBookmarks.*` | Swift | 8 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuChoice.*` | Swift | 43 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuEntry.*` | Swift | 9 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuExtension.*` | Swift | 7 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuItem.*` | Swift | 22 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuState.*` | Swift | 59 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuStateUpdate.*` | Swift | 21 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenuTarget.*` | Swift | 164 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MenusModule.*` | Swift | 175 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ModifierName.*` | Swift | 28 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ModuleLifetime.*` | Swift | 45 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MountWatcher.*` | Swift | 9 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/MouseShortcuts.*` | Swift | 65 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/NSScriptCommand.*` | Swift | 14 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/NotificationCore/CategoryLedger.*` | Swift | 33 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/NotificationHub.*` | Swift | 201 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/NotificationIcon.*` | Swift | 6 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/NotificationOptions.*` | Swift | 15 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/NotificationPostResult.*` | Swift | 15 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/NotificationsModule.*` | Swift | 40 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/OfficialPluginFetch.*` | Swift | 226 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/OpenSurfaces.*` | Swift | 233 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/OpenURLInbox.*` | Swift | 63 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/OpenWindowOptions.*` | Swift | 18 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/OutsidePressArea.*` | Swift | 39 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/OutsidePressAreaModule.*` | Swift | 13 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PasteRefusal.*` | Swift | 70 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekAffine.*` | Swift | 34 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekContainer.*` | Swift | 27 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekFrame.*` | Swift | 18 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekGeometry.*` | Swift | 54 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekInputRoute.*` | Swift | 62 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekMotion.*` | Swift | 525 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekSpring.*` | Swift | 46 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekStage.*` | Swift | 520 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PeekStageModule.*` | Swift | 120 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PendingChoice.*` | Swift | 16 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/AfterUpdateRuns.*` | Swift | 357 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/OfficialArchive.*` | Swift | 458 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/OfficialInstaller.*` | Swift | 52 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/OfficialPin.*` | Swift | 131 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/OfficialStore.*` | Swift | 249 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/OfficialTrust.*` | Swift | 54 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginBase64.*` | Swift | 23 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginCertificateCache.*` | Swift | 97 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginConnectAttempts.*` | Swift | 33 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginFetchPolicy.*` | Swift | 226 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginFiles.*` | Swift | 214 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginHttpCodec.*` | Swift | 297 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginLegacyInstalls.*` | Swift | 195 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginLifecycle.*` | Swift | 37 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginLoginPath.*` | Swift | 69 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginPageAttachment.*` | Swift | 30 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginPageData.*` | Swift | 8 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginPageGate.*` | Swift | 56 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginPageMessages.*` | Swift | 73 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginPageOffline.*` | Swift | 25 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginPageOutbox.*` | Swift | 60 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginProcessTree.*` | Swift | 152 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginProgramLinking.*` | Swift | 160 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginProgramRegistry.*` | Swift | 104 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginProgramSealing.*` | Swift | 89 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginPrograms.*` | Swift | 583 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginRuns.*` | Swift | 47 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginSandbox.*` | Swift | 420 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginStorage.*` | Swift | 67 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginTimerQueue.*` | Swift | 97 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginCore/PluginURLParser.*` | Swift | 148 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginFolders.*` | Swift | 200 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginHost.*` | Swift | 184 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginHostFetch.*` | Swift | 253 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginPageView.*` | Swift | 297 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginProcess.*` | Swift | 349 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginProgramLinks.*` | Swift | 337 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginSchemeHandler.*` | Swift | 67 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PluginsModule.*` | Swift | 191 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PointerWatcher.*` | Swift | 28 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PopoverBubble.*` | Swift | 213 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PopoverBubbleModule.*` | Swift | 27 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PopoverBubbleOverlay.*` | Swift | 8 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/PromptOptions.*` | Swift | 10 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/QuitFlow.*` | Swift | 134 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/Rebindable.*` | Swift | 8 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/RecordingPill.*` | Swift | 116 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ReportCore/ReportOverlayLayout.*` | Swift | 131 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ReportFiles.*` | Swift | 82 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ReportLayerCache.*` | Swift | 77 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ReportModule.*` | Swift | 202 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ReportProcessUsage.*` | Swift | 54 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ReportRecorder.*` | Swift | 548 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ReportUploader.*` | Swift | 69 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ReportVideoView.*` | Swift | 54 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SavedKeyResponders.*` | Swift | 52 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptCreateCommand.*` | Swift | 28 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptProfile.*` | Swift | 25 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptProfileState.*` | Swift | 9 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptState.*` | Swift | 11 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptTab.*` | Swift | 61 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptTabState.*` | Swift | 15 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptWindow.*` | Swift | 51 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptWindowState.*` | Swift | 17 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ScriptingModule.*` | Swift | 24 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ServerWindowDrag.*` | Swift | 416 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ShareMenu.*` | Swift | 48 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SharePicker.*` | Swift | 167 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ShellApp.*` | Swift | 12 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ShellEvents.*` | Swift | 60 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ShellModule.*` | Swift | 115 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ShellScripting.*` | Swift | 100 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ShortcutRecorder.*` | Swift | 89 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SidebarEdgeWatcher.*` | Swift | 353 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SkippedUpdateDefaults.*` | Swift | 14 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/StagedUpdateCopies.*` | Swift | 63 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/StandardItem.*` | Swift | 46 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SummonedWindow.*` | Swift | 42 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/Surface.*` | Swift | 142 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SurfaceModule.*` | Swift | 26 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SwitcherHold.*` | Swift | 18 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SwitcherPointer.*` | Swift | 32 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SymbolModule.*` | Swift | 18 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SymbolView.*` | Swift | 154 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SyntheticInput.*` | Swift | 55 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SystemModule.*` | Swift | 108 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/SystemSignalsModule.*` | Swift | 137 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TelemetryLocalModule.*` | Swift | 30 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TelemetryLocalStore.*` | Swift | 297 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TemplateIcon.*` | Swift | 24 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/ThinScrollBars.*` | Swift | 29 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TranslateCore/LinkedPieces.*` | Swift | 37 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TranslateCore/PassageDispatch.*` | Swift | 60 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TranslateCore/SentenceRuns.*` | Swift | 71 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TranslateCore/SharedWork.*` | Swift | 46 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TranslateError.*` | Swift | 15 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TranslateModule.*` | Swift | 74 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TranslationSessions.*` | Swift | 169 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/TypeAheadHold.*` | Swift | 84 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdateChannel.*` | Swift | 6 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdatePrewarm.*` | Swift | 184 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/ApplicationIconName.*` | Swift | 14 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/ChannelSwitch.*` | Swift | 249 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/FatalMatch.*` | Swift | 16 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/FolderWatch.*` | Swift | 79 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/GlobalChordRule.*` | Swift | 22 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/HoldReleaseKey.*` | Swift | 15 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/IconStyle.*` | Swift | 40 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/KeyEpochs.*` | Swift | 21 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/LinkActivationHold.*` | Swift | 24 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/LocalReportLines.*` | Swift | 25 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/MouseShortcutTable.*` | Swift | 34 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/ScreenEdgeCrossing.*` | Swift | 29 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/SingleLineInk.*` | Swift | 33 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/StagedSurvey.*` | Swift | 155 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/SummonedLanding.*` | Swift | 23 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/SwitchCheck.*` | Swift | 58 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/SymbolInk.*` | Swift | 54 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterCore/WindowDragPacing.*` | Swift | 44 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UpdaterModule.*` | Swift | 17 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UserScripts.*` | Swift | 225 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UserScriptsModule.*` | Swift | 23 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/UtilityContentController.*` | Swift | 17 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/VisualEffect.*` | Swift | 41 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/VisualEffectModule.*` | Swift | 17 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowDragRegion.*` | Swift | 159 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowGeometry.*` | Swift | 98 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowHost.*` | Swift | 6 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowKind.*` | Swift | 10 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowManager.*` | Swift | 600 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowMotion.*` | Swift | 136 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowMove.*` | Swift | 214 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowMovePacer.*` | Swift | 41 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowMoveProbe.*` | Swift | 165 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowProfileModule.*` | Swift | 17 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowProfileView.*` | Swift | 48 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/ios/WindowResizeProbe.*` | Swift | 103 | native | ardia-host-mac / ardia-ui-*; delete if Rune gives it (ADR-C10) | hand | todo |
| mac | `packages/shell/src/app.ts` | TypeScript/JavaScript | 121 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/automation.ts` | TypeScript/JavaScript | 12 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/browserCommand.ts` | TypeScript/JavaScript | 114 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/cli.ts` | TypeScript/JavaScript | 17 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/clipView.ts` | TypeScript/JavaScript | 1 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/clipView.windows.ts` | TypeScript/JavaScript | 3 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/commands.ts` | TypeScript/JavaScript | 10 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/config.ts` | TypeScript/JavaScript | 10 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/dev.ts` | TypeScript/JavaScript | 50 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/dialogPages.ts` | TypeScript/JavaScript | 2 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/dialogs.ts` | TypeScript/JavaScript | 28 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/documents.ts` | TypeScript/JavaScript | 8 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/externalApps.ts` | TypeScript/JavaScript | 2 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/focus.ts` | TypeScript/JavaScript | 19 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/index.tsx` | TypeScript/JavaScript | 37 | keep | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/inlineCompletion.ts` | TypeScript/JavaScript | 10 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/keptFollows.ts` | TypeScript/JavaScript | 2 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/launchLinks.ts` | TypeScript/JavaScript | 1 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/lifecycle.ts` | TypeScript/JavaScript | 55 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/live.ts` | TypeScript/JavaScript | 49 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/location.ts` | TypeScript/JavaScript | 7 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/menus.ts` | TypeScript/JavaScript | 115 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/motion.ts` | TypeScript/JavaScript | 2 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/nativeModules.ts` | TypeScript/JavaScript | 60 | caution | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/notifications.ts` | TypeScript/JavaScript | 38 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/pageCoverGate.ts` | TypeScript/JavaScript | 3 | keep | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/pageCoverGate.windows.ts` | TypeScript/JavaScript | 3 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/peekStage.tsx` | TypeScript/JavaScript | 78 | keep | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/plugins.ts` | TypeScript/JavaScript | 100 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/report.ts` | TypeScript/JavaScript | 45 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/scripting.ts` | TypeScript/JavaScript | 28 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/selfCheck.ts` | TypeScript/JavaScript | 2 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/system.tsx` | TypeScript/JavaScript | 104 | keep | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/systemSignals.ts` | TypeScript/JavaScript | 12 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/telemetryLocal.ts` | TypeScript/JavaScript | 11 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/translation.ts` | TypeScript/JavaScript | 16 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/updater.ts` | TypeScript/JavaScript | 30 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/userScripts.ts` | TypeScript/JavaScript | 14 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/views.tsx` | TypeScript/JavaScript | 189 | keep | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/shell/src/windows.ts` | TypeScript/JavaScript | 65 | ready | ardia-host-mac (and ardia-plugins, ardia-updater) | analyze | todo |
| mac | `packages/sync/ios/Core/DocumentRewriter.*` | Swift | 58 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/KeychainAccessGroups.*` | Swift | 44 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/LegacyDockTile.*` | Swift | 246 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/LegacyMigration.*` | Swift | 302 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/LegacyMigrator.*` | Swift | 535 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/MigrationError.*` | Swift | 7 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/MigrationFiles.*` | Swift | 27 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/PreferencesDomain.*` | Swift | 23 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/RecoveryKit.*` | Swift | 127 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/RecoveryPhrase.*` | Swift | 60 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/SecretStore.*` | Swift | 139 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/SharedFolderDirectory.*` | Swift | 550 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/SyncCrypto.*` | Swift | 127 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/SyncKeyStore.*` | Swift | 123 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/SyncVault.*` | Swift | 290 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/Core/Wordlist.*` | Swift | 175 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/SharedFolderWatcher.*` | Swift | 78 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/SharedFoldersModule.*` | Swift | 65 | native | ardia-sync | hand | todo |
| mac | `packages/sync/ios/SyncModule.*` | Swift | 546 | native | ardia-sync | hand | todo |
| mac | `packages/sync/scripts/` | TypeScript/JavaScript | 713 | caution | ardia-sync | analyze | todo |
| mac | `packages/sync/src/applied.ts` | TypeScript/JavaScript | 40 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/canon.ts` | TypeScript/JavaScript | 15 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/companion/` | TypeScript/JavaScript | 1126 | caution | ardia-sync | analyze | todo |
| mac | `packages/sync/src/extensions.ts` | TypeScript/JavaScript | 184 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/fileHealth.ts` | TypeScript/JavaScript | 25 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/folders.ts` | TypeScript/JavaScript | 26 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/hlc.ts` | TypeScript/JavaScript | 48 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/index.ts` | TypeScript/JavaScript | 12 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/native.ts` | TypeScript/JavaScript | 55 | caution | ardia-sync | analyze | todo |
| mac | `packages/sync/src/order.ts` | TypeScript/JavaScript | 52 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/records.ts` | TypeScript/JavaScript | 4 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/scope.ts` | TypeScript/JavaScript | 343 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/scopeKeys.ts` | TypeScript/JavaScript | 84 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/scopeState.ts` | TypeScript/JavaScript | 104 | ready | ardia-sync | analyze | todo |
| mac | `packages/sync/src/sharedFolders.ts` | TypeScript/JavaScript | 28 | ready | ardia-sync | analyze | todo |
| out | `apps/dashboard/server.mjs` | TypeScript/JavaScript | 51 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/components/` | TypeScript/JavaScript | 2591 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/components/ruixen/` | TypeScript/JavaScript | 497 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/components/triage/` | TypeScript/JavaScript | 539 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/components/ui/` | TypeScript/JavaScript | 1652 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/hooks/` | TypeScript/JavaScript | 15 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/lib/` | TypeScript/JavaScript | 813 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/router.tsx` | TypeScript/JavaScript | 21 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/routes/` | TypeScript/JavaScript | 89 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/routes/_app/` | TypeScript/JavaScript | 2271 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/routes/_app/feedback/` | TypeScript/JavaScript | 281 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/routes/_app/people/` | TypeScript/JavaScript | 427 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/routes/_app/replays/` | TypeScript/JavaScript | 443 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/routes/_app/triage/` | TypeScript/JavaScript | 768 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/routes/files/feedback/$id/` | TypeScript/JavaScript | 46 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/server/` | TypeScript/JavaScript | 1765 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/server/sections/` | TypeScript/JavaScript | 1960 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/dashboard/src/server/triage/` | TypeScript/JavaScript | 632 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/calendarWording.ts` | TypeScript/JavaScript | 94 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/credentialWording.ts` | TypeScript/JavaScript | 79 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/inputAdapters.ts` | TypeScript/JavaScript | 19 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/linuxWording.ts` | TypeScript/JavaScript | 256 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/scripts/` | TypeScript/JavaScript | 628 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/scripts/acceptance/` | TypeScript/JavaScript | 114 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/scripts/acceptance/lib/` | TypeScript/JavaScript | 1514 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/scripts/acceptance/scenarios/` | TypeScript/JavaScript | 1785 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/shortcutText.ts` | TypeScript/JavaScript | 11 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/AppMenuButton.tsx` | TypeScript/JavaScript | 78 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/AutoHideToolbarRegion.tsx` | TypeScript/JavaScript | 43 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/CredentialNotice.tsx` | TypeScript/JavaScript | 33 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/DownloadsPopover.tsx` | TypeScript/JavaScript | 34 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/ExtensionActionPopup.tsx` | TypeScript/JavaScript | 100 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/GlassPanel.tsx` | TypeScript/JavaScript | 10 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/InWindowPlayer.tsx` | TypeScript/JavaScript | 416 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/LeadingWindowControls.tsx` | TypeScript/JavaScript | 30 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/LinuxAutofillPane.tsx` | TypeScript/JavaScript | 20 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/LinuxPasswordsPane.tsx` | TypeScript/JavaScript | 13 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/LinuxUnlockStep.tsx` | TypeScript/JavaScript | 76 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/PageOverlayRegion.tsx` | TypeScript/JavaScript | 14 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/PageOverlays.tsx` | TypeScript/JavaScript | 28 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/PeekOverlay.tsx` | TypeScript/JavaScript | 15 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/ProfileIndicator.tsx` | TypeScript/JavaScript | 52 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/SelectionPopover.tsx` | TypeScript/JavaScript | 13 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/SettingsInstallDialog.tsx` | TypeScript/JavaScript | 12 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/SidebarAddressBar.tsx` | TypeScript/JavaScript | 31 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/SidebarHoverCard.tsx` | TypeScript/JavaScript | 12 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/SidebarPeekMark.tsx` | TypeScript/JavaScript | 23 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/SiteConnection.tsx` | TypeScript/JavaScript | 13 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/TileFace.tsx` | TypeScript/JavaScript | 63 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/TopStripPeekRegion.tsx` | TypeScript/JavaScript | 10 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/TrafficLights.tsx` | TypeScript/JavaScript | 39 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/WindowControls.tsx` | TypeScript/JavaScript | 36 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/WindowOverlays.tsx` | TypeScript/JavaScript | 4 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/appMenu.ts` | TypeScript/JavaScript | 265 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/appModuleFallbacks.ts` | TypeScript/JavaScript | 121 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/appServices.ts` | TypeScript/JavaScript | 249 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/bookmarksHtml.ts` | TypeScript/JavaScript | 56 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/browserCommandRoute.ts` | TypeScript/JavaScript | 109 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/calendarIcs.ts` | TypeScript/JavaScript | 555 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/canonicalLiveDocument.ts` | TypeScript/JavaScript | 5 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/captionControls.ts` | TypeScript/JavaScript | 7 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/clickModifiers.ts` | TypeScript/JavaScript | 19 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/clusterAppMenu.ts` | TypeScript/JavaScript | 11 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/clusterMenu.ts` | TypeScript/JavaScript | 53 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/commandCatalog.ts` | TypeScript/JavaScript | 46 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/credentialPaneData.ts` | TypeScript/JavaScript | 30 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/desktopButtonLayout.ts` | TypeScript/JavaScript | 62 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/documentFlush.ts` | TypeScript/JavaScript | 13 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/downloadsDismissal.ts` | TypeScript/JavaScript | 59 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/dragWebviews.ts` | TypeScript/JavaScript | 7 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/editCommands.ts` | TypeScript/JavaScript | 75 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/editTargets.ts` | TypeScript/JavaScript | 28 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/elementPickerHost.ts` | TypeScript/JavaScript | 50 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/extensionInstalls.ts` | TypeScript/JavaScript | 10 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/extensionPrompts.ts` | TypeScript/JavaScript | 13 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/extensionsStore.ts` | TypeScript/JavaScript | 18 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/fieldTiming.ts` | TypeScript/JavaScript | 7 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/fileCapabilities.ts` | TypeScript/JavaScript | 4 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/fileDrag.ts` | TypeScript/JavaScript | 32 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/fileDrop.ts` | TypeScript/JavaScript | 67 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/glassRegions.ts` | TypeScript/JavaScript | 256 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/glassUi.tsx` | TypeScript/JavaScript | 20 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/host.ts` | TypeScript/JavaScript | 70 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostAppearance.ts` | TypeScript/JavaScript | 34 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostBookmarksExport.ts` | TypeScript/JavaScript | 75 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostCalendar.ts` | TypeScript/JavaScript | 290 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostCapabilities.ts` | TypeScript/JavaScript | 111 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostCast.ts` | TypeScript/JavaScript | 35 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostCli.ts` | TypeScript/JavaScript | 58 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostConfig.ts` | TypeScript/JavaScript | 70 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostCrashReports.ts` | TypeScript/JavaScript | 87 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostCredentials.ts` | TypeScript/JavaScript | 183 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostData.ts` | TypeScript/JavaScript | 12 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostDialogs.ts` | TypeScript/JavaScript | 141 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostDocuments.ts` | TypeScript/JavaScript | 56 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostDownloads.ts` | TypeScript/JavaScript | 117 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostEngines.ts` | TypeScript/JavaScript | 16 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostEvents.ts` | TypeScript/JavaScript | 28 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostExtensionScripts.ts` | TypeScript/JavaScript | 44 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostExtensionView.ts` | TypeScript/JavaScript | 133 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostFiles.ts` | TypeScript/JavaScript | 139 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostFolders.ts` | TypeScript/JavaScript | 26 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostHistoryImports.ts` | TypeScript/JavaScript | 8 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostImport.ts` | TypeScript/JavaScript | 376 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostLaunchConfig.ts` | TypeScript/JavaScript | 16 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostLive.ts` | TypeScript/JavaScript | 86 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostModules.ts` | TypeScript/JavaScript | 96 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostPageEvents.ts` | TypeScript/JavaScript | 213 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostPageHooks.ts` | TypeScript/JavaScript | 20 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostPageLifecycle.ts` | TypeScript/JavaScript | 82 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostPageScripts.ts` | TypeScript/JavaScript | 58 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostPolicies.ts` | TypeScript/JavaScript | 142 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostPrivateWindows.ts` | TypeScript/JavaScript | 40 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostReport.ts` | TypeScript/JavaScript | 63 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostSession.ts` | TypeScript/JavaScript | 505 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostSessionMerge.ts` | TypeScript/JavaScript | 74 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostSessionState.ts` | TypeScript/JavaScript | 52 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostSmallWindows.ts` | TypeScript/JavaScript | 141 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostSuggestions.ts` | TypeScript/JavaScript | 77 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostSync.ts` | TypeScript/JavaScript | 401 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostSystem.ts` | TypeScript/JavaScript | 43 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostTab.ts` | TypeScript/JavaScript | 204 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostTabLifecycle.ts` | TypeScript/JavaScript | 50 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostTabPresentations.ts` | TypeScript/JavaScript | 71 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostTabQueue.ts` | TypeScript/JavaScript | 28 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostTelemetry.ts` | TypeScript/JavaScript | 188 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostTelemetryLocal.ts` | TypeScript/JavaScript | 409 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostUpdates.ts` | TypeScript/JavaScript | 367 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostUserScripts.ts` | TypeScript/JavaScript | 21 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostViewport.ts` | TypeScript/JavaScript | 90 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostWindowControls.ts` | TypeScript/JavaScript | 9 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/hostWindowProfiles.ts` | TypeScript/JavaScript | 149 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/iconAccent.ts` | TypeScript/JavaScript | 67 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/importApply.ts` | TypeScript/JavaScript | 54 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/importHandles.ts` | TypeScript/JavaScript | 40 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/importUnlock.ts` | TypeScript/JavaScript | 131 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/importedPasswordSave.ts` | TypeScript/JavaScript | 5 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/inputRegions.ts` | TypeScript/JavaScript | 227 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/installRequestSheet.ts` | TypeScript/JavaScript | 36 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/introMusic.ts` | TypeScript/JavaScript | 80 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/introMusicTrack.ts` | TypeScript/JavaScript | 1 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/linkShare.ts` | TypeScript/JavaScript | 44 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/linkShareSurface.ts` | TypeScript/JavaScript | 17 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/linuxUpdates.ts` | TypeScript/JavaScript | 98 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/linuxWindowMenu.ts` | TypeScript/JavaScript | 9 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/liveCompletionTickets.ts` | TypeScript/JavaScript | 41 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/liveDocumentClient.ts` | TypeScript/JavaScript | 229 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/liveDocumentIntent.ts` | TypeScript/JavaScript | 117 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/liveDocumentTypes.ts` | TypeScript/JavaScript | 32 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/liveHooks.ts` | TypeScript/JavaScript | 18 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/liveNet.ts` | TypeScript/JavaScript | 15 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/livePersistence.ts` | TypeScript/JavaScript | 22 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/liveSources.ts` | TypeScript/JavaScript | 171 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/main.tsx` | TypeScript/JavaScript | 201 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/menus.tsx` | TypeScript/JavaScript | 184 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/minidump.ts` | TypeScript/JavaScript | 256 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/notificationPermission.ts` | TypeScript/JavaScript | 8 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/openUrls.ts` | TypeScript/JavaScript | 8 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/pageFocus.ts` | TypeScript/JavaScript | 33 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/pageHoles.ts` | TypeScript/JavaScript | 275 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/pickConfirmation.ts` | TypeScript/JavaScript | 1 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/pipGeometry.ts` | TypeScript/JavaScript | 124 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/pipPlayer.ts` | TypeScript/JavaScript | 88 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/playerHoles.ts` | TypeScript/JavaScript | 126 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/policyPerformance.ts` | TypeScript/JavaScript | 6 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/policyToasts.ts` | TypeScript/JavaScript | 7 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/popoverEscape.ts` | TypeScript/JavaScript | 35 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/popupDismissal.ts` | TypeScript/JavaScript | 21 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/profileIndicatorLayout.ts` | TypeScript/JavaScript | 28 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/quit.ts` | TypeScript/JavaScript | 72 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/rawBookmarkActions.ts` | TypeScript/JavaScript | 4 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/reauthCounts.ts` | TypeScript/JavaScript | 8 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/reportProblem.ts` | TypeScript/JavaScript | 37 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/sandboxNotice.ts` | TypeScript/JavaScript | 34 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/session.ts` | TypeScript/JavaScript | 28 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/sessionClose.ts` | TypeScript/JavaScript | 8 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/settingsForm.tsx` | TypeScript/JavaScript | 31 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/settingsIndex.ts` | TypeScript/JavaScript | 17 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/settingsWindows.ts` | TypeScript/JavaScript | 25 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shellAbove.ts` | TypeScript/JavaScript | 102 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shellGlass.ts` | TypeScript/JavaScript | 198 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shellMethods.ts` | TypeScript/JavaScript | 135 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shellPainted.ts` | TypeScript/JavaScript | 14 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shims/` | TypeScript/JavaScript | 632 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shims/web/` | TypeScript/JavaScript | 1623 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shims/web/gl/` | TypeScript/JavaScript | 1085 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shims/web/motion/` | TypeScript/JavaScript | 1241 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shims/web/symbols/` | TypeScript/JavaScript | 1098 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shortcutCommands.ts` | TypeScript/JavaScript | 7 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shortcutFormat.ts` | TypeScript/JavaScript | 18 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shortcutOverrides.ts` | TypeScript/JavaScript | 21 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shortcutState.ts` | TypeScript/JavaScript | 32 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shortcuts.ts` | TypeScript/JavaScript | 95 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/shortcutsRuntime.ts` | TypeScript/JavaScript | 114 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/sidebarHeaderLayout.ts` | TypeScript/JavaScript | 51 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/suggestFetcher.ts` | TypeScript/JavaScript | 21 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/syncAuthority.ts` | TypeScript/JavaScript | 270 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/syncEngine.ts` | TypeScript/JavaScript | 329 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/syncForward.ts` | TypeScript/JavaScript | 34 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/tabSwitchAhead.ts` | TypeScript/JavaScript | 41 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/telemetryChoice.ts` | TypeScript/JavaScript | 31 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/telemetryChoiceApply.ts` | TypeScript/JavaScript | 28 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/telemetryLeader.ts` | TypeScript/JavaScript | 179 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/telemetrySettle.ts` | TypeScript/JavaScript | 16 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/tilingDesktop.ts` | TypeScript/JavaScript | 3 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/typeahead.ts` | TypeScript/JavaScript | 62 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/unclaimedTabs.ts` | TypeScript/JavaScript | 57 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/updateReleases.ts` | TypeScript/JavaScript | 38 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/utility.ts` | TypeScript/JavaScript | 50 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/utilityClose.ts` | TypeScript/JavaScript | 79 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/utilityExtensions.ts` | TypeScript/JavaScript | 14 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/utilityNavigation.ts` | TypeScript/JavaScript | 16 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowCloseCoordinator.ts` | TypeScript/JavaScript | 215 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowCloseDecider.ts` | TypeScript/JavaScript | 7 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowControlsRules.ts` | TypeScript/JavaScript | 22 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowDecorations.ts` | TypeScript/JavaScript | 68 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowDeparture.ts` | TypeScript/JavaScript | 20 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowDrag.ts` | TypeScript/JavaScript | 51 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowFrame.ts` | TypeScript/JavaScript | 6 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowMaximized.ts` | TypeScript/JavaScript | 54 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowMenu.ts` | TypeScript/JavaScript | 79 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/src/windowSweep.ts` | TypeScript/JavaScript | 8 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/syncWording.ts` | TypeScript/JavaScript | 92 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/linux/vite.shared.ts` | TypeScript/JavaScript | 114 | blocked | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/index.js` | TypeScript/JavaScript | 6 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/scripts/` | TypeScript/JavaScript | 1652 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/App.tsx` | TypeScript/JavaScript | 290 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/browsing/` | TypeScript/JavaScript | 3933 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/duo/` | TypeScript/JavaScript | 783 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/library/` | TypeScript/JavaScript | 1469 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/onboarding/` | TypeScript/JavaScript | 743 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/perf/` | TypeScript/JavaScript | 31 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/search/` | TypeScript/JavaScript | 2761 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/settings/` | TypeScript/JavaScript | 4315 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/sync/` | TypeScript/JavaScript | 1406 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/tabs/` | TypeScript/JavaScript | 4027 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/toasts/` | TypeScript/JavaScript | 178 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/ui/` | TypeScript/JavaScript | 5884 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/components/upgrade/` | TypeScript/JavaScript | 43 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/dev/` | TypeScript/JavaScript | 1659 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/lib/` | TypeScript/JavaScript | 2859 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/lib/material/` | TypeScript/JavaScript | 32 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/platform/` | TypeScript/JavaScript | 279 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/render/` | TypeScript/JavaScript | 46 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/app/` | TypeScript/JavaScript | 572 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/browse/` | TypeScript/JavaScript | 1593 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/browseForMe/` | TypeScript/JavaScript | 89 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/history/` | TypeScript/JavaScript | 99 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/legacy/` | TypeScript/JavaScript | 471 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/library/` | TypeScript/JavaScript | 289 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/links/` | TypeScript/JavaScript | 267 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/onboarding/` | TypeScript/JavaScript | 57 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/page/` | TypeScript/JavaScript | 976 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/pageSummary/` | TypeScript/JavaScript | 85 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/pages/` | TypeScript/JavaScript | 173 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/search/` | TypeScript/JavaScript | 421 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/settings/` | TypeScript/JavaScript | 801 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/store/` | TypeScript/JavaScript | 257 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/sync/` | TypeScript/JavaScript | 807 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/tabs/` | TypeScript/JavaScript | 804 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/telemetry/` | TypeScript/JavaScript | 89 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/toasts/` | TypeScript/JavaScript | 120 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/phone/src/state/voice/` | TypeScript/JavaScript | 164 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/scripts/` | TypeScript/JavaScript | 429 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/components/` | TypeScript/JavaScript | 239 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/data/` | TypeScript/JavaScript | 1365 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/layouts/` | TypeScript/JavaScript | 9 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/lib/` | TypeScript/JavaScript | 208 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/pages/` | TypeScript/JavaScript | 42 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/pages/press/` | TypeScript/JavaScript | 32 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/pages/release-notes/` | TypeScript/JavaScript | 109 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/scripts/` | TypeScript/JavaScript | 595 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/scripts/meadow/` | TypeScript/JavaScript | 5092 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/site/src/scripts/telemetry/` | TypeScript/JavaScript | 1303 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/scripts/` | TypeScript/JavaScript | 114 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/scripts/capture/` | TypeScript/JavaScript | 355 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/scripts/capture/scenes/` | TypeScript/JavaScript | 307 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/scripts/capture/sites/` | TypeScript/JavaScript | 24 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/scripts/fixes/` | TypeScript/JavaScript | 1678 | blocked | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/scripts/fixes/scenes/` | TypeScript/JavaScript | 3263 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/Root.tsx` | TypeScript/JavaScript | 3 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/entries/` | TypeScript/JavaScript | 15 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/index.ts` | TypeScript/JavaScript | 4 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/lib/ac-launch/` | TypeScript/JavaScript | 1447 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/lib/fixes-film/` | TypeScript/JavaScript | 975 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/lib/sf-film/` | TypeScript/JavaScript | 1790 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/lib/studio-objects-v5/` | TypeScript/JavaScript | 486 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/lib/studio-objects-v6/` | TypeScript/JavaScript | 58 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/` | TypeScript/JavaScript | 63 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/dia-look/` | TypeScript/JavaScript | 596 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0243/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0243n1/` | TypeScript/JavaScript | 455 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0243n3/` | TypeScript/JavaScript | 669 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0243n4/` | TypeScript/JavaScript | 673 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0243n5/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0243n7/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0243n8/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0243n9/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0244n1/` | TypeScript/JavaScript | 229 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0244n2/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0244n3/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0244n4/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0244n7/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0245n1/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0245n3/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0245n4/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0246-about-blank/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0246n2/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0246n6/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0246n7/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0247n3/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0247n6/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0247n7/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-0247n8/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-atc-pattern/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-custom-lists/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-extension-order/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-meet-auto-pip/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/fixes-page-focus/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/launch-16x9/` | TypeScript/JavaScript | 6 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/launch-9x16/` | TypeScript/JavaScript | 6 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/open-tabs-sync/` | TypeScript/JavaScript | 412 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/release-0244/` | TypeScript/JavaScript | 718 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/release-0245/` | TypeScript/JavaScript | 839 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/release-0246/` | TypeScript/JavaScript | 5 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/scroll-perf/` | TypeScript/JavaScript | 434 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/shared-folders-16x9-novo/` | TypeScript/JavaScript | 6 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/shared-folders-16x9/` | TypeScript/JavaScript | 6 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/shared-folders-9x16/` | TypeScript/JavaScript | 6 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/teaser-16x9/` | TypeScript/JavaScript | 6 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/teaser-9x16/` | TypeScript/JavaScript | 6 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/theme-look-16x9/` | TypeScript/JavaScript | 574 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `apps/videos/src/videos/theo-fixes/` | TypeScript/JavaScript | 428 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `engine/ardiacore-linux/content-blocker/` | TypeScript/JavaScript | 319 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `engine/ardiacore-linux/src/ardia/linux/` | TypeScript/JavaScript | 128 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `extras/raycast-ardia/src/ardia.ts` | TypeScript/JavaScript | 52 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `extras/raycast-ardia/src/new-incognito-window.ts` | TypeScript/JavaScript | 11 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `extras/raycast-ardia/src/new-tab.ts` | TypeScript/JavaScript | 11 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `extras/raycast-ardia/src/new-window.ts` | TypeScript/JavaScript | 12 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `extras/raycast-ardia/src/scripts.ts` | TypeScript/JavaScript | 136 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `extras/raycast-ardia/src/search-profiles.tsx` | TypeScript/JavaScript | 59 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `extras/raycast-ardia/src/search-tabs.tsx` | TypeScript/JavaScript | 77 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `infra/` | TypeScript/JavaScript | 18 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `infra/dashboard/` | TypeScript/JavaScript | 26 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `infra/feedback/hexclave.deploy.ts` | TypeScript/JavaScript | 33 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `infra/feedback/lib/` | TypeScript/JavaScript | 882 | blocked | none (out of scope, ADR-C3) | analyze | - |
| out | `infra/feedback/server.mjs` | TypeScript/JavaScript | 100 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `infra/site/` | TypeScript/JavaScript | 21 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `infra/telemetry/` | TypeScript/JavaScript | 44 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `infra/telemetry/collector/` | TypeScript/JavaScript | 36 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/scripts/` | TypeScript/JavaScript | 480 | blocked | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/scripts/icons/` | TypeScript/JavaScript | 1511 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/Customize.tsx` | TypeScript/JavaScript | 35 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/customizeFlow.ts` | TypeScript/JavaScript | 31 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/embed.ts` | TypeScript/JavaScript | 77 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/frameable.ts` | TypeScript/JavaScript | 36 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/localSuggest.ts` | TypeScript/JavaScript | 55 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/main.tsx` | TypeScript/JavaScript | 54 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/nudge.ts` | TypeScript/JavaScript | 70 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/pages.ts` | TypeScript/JavaScript | 61 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/seed.ts` | TypeScript/JavaScript | 95 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/shims/` | TypeScript/JavaScript | 480 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/shims/web/` | TypeScript/JavaScript | 1097 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/shims/web/gl/` | TypeScript/JavaScript | 801 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/shims/web/motion/` | TypeScript/JavaScript | 1195 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/demo/src/shims/web/symbols/` | TypeScript/JavaScript | 1095 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/animation/` | TypeScript/JavaScript | 169 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/appKeyboard.ts` | TypeScript/JavaScript | 62 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/defaults.ts` | TypeScript/JavaScript | 42 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/engine/` | TypeScript/JavaScript | 1690 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/engine/domains/` | TypeScript/JavaScript | 1318 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/events.ts` | TypeScript/JavaScript | 42 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/fiber.ts` | TypeScript/JavaScript | 1 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/import/` | TypeScript/JavaScript | 2956 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/index.ts` | TypeScript/JavaScript | 42 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/inlineCompletion.ts` | TypeScript/JavaScript | 13 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/introMusic.ts` | TypeScript/JavaScript | 9 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/launchLinks/` | TypeScript/JavaScript | 93 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/meadow/` | TypeScript/JavaScript | 369 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/missing.ts` | TypeScript/JavaScript | 14 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/modules.ts` | TypeScript/JavaScript | 89 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/nativeModuleTypes.ts` | TypeScript/JavaScript | 108 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/pager/` | TypeScript/JavaScript | 407 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/selfCheck/` | TypeScript/JavaScript | 5312 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/symbol/` | TypeScript/JavaScript | 229 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/sync/` | TypeScript/JavaScript | 734 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/textFocus.ts` | TypeScript/JavaScript | 77 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/updater/` | TypeScript/JavaScript | 220 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/views/` | TypeScript/JavaScript | 1815 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/windowDark.ts` | TypeScript/JavaScript | 6 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/windowDragRegion.tsx` | TypeScript/JavaScript | 178 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/expo-windows/src/windowRoot.tsx` | TypeScript/JavaScript | 25 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/android/scripts/` | TypeScript/JavaScript | 150 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/android/src/main/assets/ardia-browser/` | TypeScript/JavaScript | 342 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/android/src/main/resources/com/ardia/blocking/` | TypeScript/JavaScript | 156 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/ios/ArdiaKit/Sources/ArdiaShared/Scripts/` | TypeScript/JavaScript | 15830 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/PageView.tsx` | TypeScript/JavaScript | 80 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/accessibility.tsx` | TypeScript/JavaScript | 43 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/back.ts` | TypeScript/JavaScript | 73 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/blocking.ts` | TypeScript/JavaScript | 34 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/browseForMe.tsx` | TypeScript/JavaScript | 69 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/clipboard.ts` | TypeScript/JavaScript | 9 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/completingField.tsx` | TypeScript/JavaScript | 135 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/defaultBrowser.ts` | TypeScript/JavaScript | 15 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/downloads.ts` | TypeScript/JavaScript | 29 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/fade.tsx` | TypeScript/JavaScript | 16 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/fadeProps.ts` | TypeScript/JavaScript | 11 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/favicons.ts` | TypeScript/JavaScript | 8 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/index.ts` | TypeScript/JavaScript | 146 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/keyboard.ts` | TypeScript/JavaScript | 19 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/legacy.ts` | TypeScript/JavaScript | 4 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/legacyApi.ts` | TypeScript/JavaScript | 159 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/look.tsx` | TypeScript/JavaScript | 104 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/lookBudget.ts` | TypeScript/JavaScript | 93 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/lookBudgetRules.ts` | TypeScript/JavaScript | 196 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/material.ts` | TypeScript/JavaScript | 55 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/materialSearchBar.tsx` | TypeScript/JavaScript | 195 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/menus.tsx` | TypeScript/JavaScript | 62 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/motion.tsx` | TypeScript/JavaScript | 63 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/network.ts` | TypeScript/JavaScript | 14 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/normalize.ts` | TypeScript/JavaScript | 22 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/pageTypes.ts` | TypeScript/JavaScript | 242 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/pages.ts` | TypeScript/JavaScript | 218 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/pasteButton.tsx` | TypeScript/JavaScript | 17 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/scan.ts` | TypeScript/JavaScript | 42 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/screenLayout.tsx` | TypeScript/JavaScript | 32 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/searchStage.tsx` | TypeScript/JavaScript | 107 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/settingsUI.tsx` | TypeScript/JavaScript | 95 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/symbol.tsx` | TypeScript/JavaScript | 19 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/sync.ts` | TypeScript/JavaScript | 136 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/syncLab.ts` | TypeScript/JavaScript | 13 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/system.ts` | TypeScript/JavaScript | 72 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/systemLook.ts` | TypeScript/JavaScript | 47 | keep | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/systemLookRules.ts` | TypeScript/JavaScript | 46 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `packages/phone/src/voice.ts` | TypeScript/JavaScript | 29 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `scripts/` | TypeScript/JavaScript | 653 | caution | none (out of scope, ADR-C3) | analyze | - |
| out | `scripts/ci/` | TypeScript/JavaScript | 130 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `scripts/lib/` | TypeScript/JavaScript | 907 | blocked | none (out of scope, ADR-C3) | analyze | - |
| out | `scripts/linux/` | TypeScript/JavaScript | 1754 | blocked | none (out of scope, ADR-C3) | analyze | - |
| out | `scripts/security/` | TypeScript/JavaScript | 280 | ready | none (out of scope, ADR-C3) | analyze | - |
| out | `scripts/stats/` | TypeScript/JavaScript | 113 | ready | none (out of scope, ADR-C3) | analyze | - |
