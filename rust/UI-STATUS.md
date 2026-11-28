# Ardia UI status

Every user-facing screen of Ardia, with the Ria issue that builds it. The captain ordered the UI first (2026-11-28): each screen is built in Rune now, against a placeholder provider, before the store and engine slices.

The work list is [milestone 1](https://github.com/i098/ria/milestone/1). Each issue names the Ardia source, the features, states and interactions to build, how a user opens it, the placeholder data, the Ria files its worker owns (no file in two issues), and the proof. The shared scaffold is #217.

`State` is `not built` while no user path opens the screen in Ria. `partial` means Ria shows a stand-in: the plain tab strip and toolbar of `rust/crates/ria-host-mac/src/launch/strip_layer.rs` (controls at `:85` and `:110`) are replaced by the scaffold shell and the window issues. `library` is a shared part with no screen of its own.

## Screens and surfaces

| Screen or surface | Ardia source | Ria route | Issue | State |
| --- | --- | --- | --- | --- |
| **Shared scaffold and kit** | | | | |
| Build the shared Rune UI scaffold: ria-ui crate, routing, theme seed, placeholder providers | `components/ui/` | KitGallery, IconPicker, SettingsFormGallery, SettingsThemeGallery | #217 | partial: the plain strip and toolbar of `strip_layer.rs` are the stand-in |
| Port the Ardia theme: color roles, type scale, radii, theme scopes | `components/ui/` | none | #218 | library, not built |
| Port Ardia motion: curves, springs, press scale, hover fill | `components/ui/` | none | #220 | library, not built |
| Port Ardia kit primitives: Text, Box, Symbol, Badge, rows, swatches | `components/ui/` | none | #221 | library, not built |
| Port Ardia glyph, favicon and unread badge parts | `components/favicon/`, `components/glyph/`, `components/unread/` | none | #222 | library, not built |
| Port Ardia kit controls: buttons, switches, radios, segments, fields | `components/ui/` | none | #223 | library, not built |
| Port Ardia floating surfaces: popovers, panels, menus, dialogs, toasts | `components/ui/` | none | #226 | library, not built |
| Build the icon picker panel for tab, folder and profile icons | `components/iconPicker/` | IconPicker | #231 | not built |
| **Window, toolbar and tab strip** | | | | |
| Build the window card: frame, panes, chrome and theme scope | `components/window/` | WindowCard | #313 | not built |
| Build page overlays: HUDs, loading pill, notices, frozen page, drops | `components/window/` | PageOverlays | #314 | not built |
| Build the sidebar dock: slide, peek, edge reveal and lights | `components/window/` | WindowDock | #315 | not built |
| Build the top tab strip surface: scroller, fit, edge fade, drag engine | `components/tabstrip/` | TopTabStrip | #316 | not built |
| Build swipe overlays: back/forward, history, tab and profile swipe | `components/window/` | SwipeOverlays | #317 | not built |
| Build tab chips: selected tab, close, mute, hover card, new tab button | `components/tabstrip/` | TabStripChips | #318 | not built |
| Build the toolbar: address field, nav buttons, split buttons | `components/toolbar/` | Toolbar | #319 | not built |
| Build tab groups: header, name, members, collapse and split chips | `components/tabstrip/` | TabStripGroups | #320 | not built |
| Build the pinned dock: tiles, folder cells, pin-out and tab-in drag | `components/tabstrip/` | TabStripPinned | #321 | not built |
| Build toolbar auto-hide, developer bar and field load fills | `components/toolbar/` | ToolbarField | #322 | not built |
| Build the window controls, close/minimize/maximize and error screens | `components/window/` | WindowControls | #323 | not built |
| Build the tab pane: page slot, bars, warm panes, crash states | `components/window/` | TabPane | #324 | not built |
| **Sidebar and split view** | | | | |
| Build the sidebar kit: tokens, geometry, rows, icons and hover | `components/sidebar/` | none | #224 | not built |
| Build the sidebar address bar and sidebar header tools | `components/sidebar/` | SidebarAddressBar | #227 | not built |
| Build the sidebar drag engine with drop targets and profile switching | `components/sidebar/` | none | #228 | not built |
| Build the Search Tabs floating panel with fuzzy ranking | `components/sidebar/` | SearchTabs | #232 | not built |
| Build the tab and group hover card with folder peek and actions | `components/sidebar/` | SidebarHoverCard | #233 | not built |
| Build the pinned tile grid, folder tile and folder panel | `components/sidebar/` | SidebarPinned | #234 | not built |
| Build the sidebar tab row, split row, New Tab row and rows kit | `components/sidebar/` | SidebarTabRows | #235 | not built |
| Build split view dividers, focus ring, drop zones and empty pane | `components/split/` | SplitView | #236 | not built |
| Build the Tab Switcher overlay with preview cards | `components/sidebar/` | TabSwitcher | #237 | not built |
| Build the sidebar Change Icon picker wrapper and overlay host | `components/sidebar/` | SidebarIconPicker | #253 | not built |
| Build live folder blocks, rows, stacks and completed footer | `components/sidebar/` | SidebarLiveFolders | #254 | not built |
| Build the sidebar tab list with groups, motion, ghosts and upsell | `components/sidebar/` | SidebarTabList | #267 | not built |
| Build the sidebar frame with header, footer, paging, pill and resize | `components/sidebar/` | Sidebar | #291 | not built |
| **Command bar, new tab page, find, peek, small window** | | | | |
| Build the Find bar in Rune, with replace and result counts | `components/find/` | Find | #238 | not built |
| Build the command bar frame, field and scope chips in Rune | `components/omnibox/` | none | #239 | not built |
| Build the command bar suggestion list and inline completion | `components/omnibox/` | none | #255 | not built |
| Build the stateful command bar and its floating panel in Rune | `components/omnibox/` | CommandBar | #268 | not built |
| Build the Little Ardia window chrome, bar and gestures in Rune | `components/littleArdia/` | LittleArdia | #292 | not built |
| Build the New Tab page, its mark and its centred bar in Rune | `components/ntp/` | NewTab | #293 | not built |
| Build the New Tab cards, Personalize panel and release postcard | `components/ntp/` | NewTab | #305 | not built |
| Build the Peek overlay card, its side buttons and its geometry | `components/peek/` | Peek | #306 | not built |
| **Settings window** | | | | |
| Port the Settings index (pane list, rows, aliases) to Rust | `components/settings/` | none | #219 | library, not built |
| Build the shared Settings form components in Rune | `components/settings/` | none | #240 | library, not built |
| Build the profile theme editor and swatches in Rune | `components/settings/` | none (gallery `SettingsThemeGallery`) | #256 | library, not built |
| Build the Settings window (sidebar, search, history, sheets) in Rune | `components/settings/` | Settings | #257 | not built |
| Build the link routing rule sheet in Rune | `components/settings/` | SettingsAirTraffic | #269 | not built |
| Build the Settings Appearance pane in Rune | `components/settings/` | SettingsAppearance | #270 | not built |
| Build the Saved addresses and Saved cards pages in Rune | `components/settings/` | SettingsAddresses, SettingsCards | #271 | not built |
| Build the live folder Connect Account sheet in Rune | `components/settings/` | SettingsConnectAccount | #272 | not built |
| Build the Settings crash report details page in Rune | `components/settings/` | SettingsCrashDetails | #273 | not built |
| Build the Settings Filter lists page in Rune | `components/settings/` | SettingsFilterLists | #274 | not built |
| Build the Search section and Other search engines page in Rune | `components/settings/` | SettingsOtherEngines | #275 | not built |
| Build the Passwords section and Passwords page in Rune | `components/settings/` | SettingsPasswords | #276 | not built |
| Build the Settings Plugins pane and plugin page in Rune | `components/settings/` | SettingsPlugins, SettingsPlugin | #277 | not built |
| Build the Settings profile extensions section and page in Rune | `components/settings/` | SettingsExtension | #278 | not built |
| Build the Settings Shortcuts pane in Rune | `components/settings/` | SettingsShortcuts | #279 | not built |
| Build the Settings site settings pages in Rune | `components/settings/` | SettingsSites, SettingsSite | #280 | not built |
| Build the Settings Tabs pane in Rune | `components/settings/` | SettingsTabs | #281 | not built |
| Build the Settings Automations pane in Rune | `components/settings/` | SettingsAutomations | #294 | not built |
| Build the Settings General pane in Rune | `components/settings/` | SettingsGeneral | #295 | not built |
| Build the Settings Profiles pane and profile switcher in Rune | `components/settings/` | SettingsProfiles | #296 | not built |
| Build the Settings Advanced pane in Rune | `components/settings/` | SettingsAdvanced | #307 | not built |
| **Setup, import and profiles** | | | | |
| Port the onboarding preview drawing kit and style stage | `components/onboarding/` | none | #229 | library, not built |
| Build the browser import window and Safari access pages | `components/import/` | Import | #241 | not built |
| Build the animated scenes for onboarding choices | `components/onboarding/` | none | #242 | library, not built |
| Build the profile dots, header indicator and menus | `components/profiles/` | none | #243 | not built |
| Build the browser import selection, progress and result steps | `components/import/` | Import | #258 | not built |
| Build the onboarding mock window and development gallery | `components/onboarding/` | OnboardingPreviewGallery | #259 | not built |
| Build the onboarding card frame, step layout and choice widgets | `components/onboarding/` | Onboarding | #282 | not built |
| Build the create profile sheet and the profile icon picker | `components/profiles/` | CreateProfile, ProfileIconPicker | #283 | not built |
| Build the onboarding default browser and external link questions | `components/onboarding/` | OnboardingHome | #297 | not built |
| Build the onboarding intro curtain with wordmark, tagline and music | `components/onboarding/` | OnboardingIntro | #298 | not built |
| Build the onboarding layout and tab-habit question screens | `components/onboarding/` | OnboardingLayout, OnboardingTabs | #299 | not built |
| Build the onboarding pinned apps step | `components/onboarding/` | OnboardingPins | #300 | not built |
| Build the onboarding plugin offer and consent sheet | `components/onboarding/` | OnboardingPlugins | #301 | not built |
| Build the onboarding source picker and ardia.dev setup-code steps | `components/onboarding/` | OnboardingSource, OnboardingSetupCode | #302 | not built |
| Build the guided tool tour with a movable spotlight | `components/onboarding/` | ToolTour | #303 | not built |
| Build the onboarding appearance, style, frame and colour questions | `components/onboarding/` | OnboardingLook | #308 | not built |
| Build the onboarding welcome postcard and setup-code summary | `components/onboarding/` | OnboardingOutro, OnboardingSetupCodeOutro | #309 | not built |
| Build the onboarding search engine and crash-report privacy step | `components/onboarding/` | OnboardingPrivacy | #310 | not built |
| **Plugins, extensions, site, live and media surfaces** | | | | |
| Build the toolbar extension controls in Rune | `components/extensions/` | ToolbarExtensions | #244 | not built |
| Build calendar and live item cards in Rune | `components/live/` | MeetingAlert | #245 | not built |
| Build the media players and tab capture badges | `components/media/` | MediaPlayers | #246 | not built |
| Build the plugin consent sheet in Rune | `components/plugins/` | PluginConsent | #247 | not built |
| Build plugin surface actions and keyboard shortcuts | `components/plugins/` | PluginSurfaceActions | #248 | not built |
| Build extension installation and extension views | `components/extensions/` | ExtensionSurfaces | #260 | not built |
| Build screen sharing and Cast controls in Rune | `components/media/` | SharePicker | #261 | not built |
| Build the plugin surface renderer in Rune | `components/plugins/` | PluginSurfaceGallery | #262 | not built |
| Build plugin controls in the browser chrome | `components/plugins/` | PluginChrome | #284 | not built |
| Build plugin quick picks and floating views | `components/plugins/` | PluginQuickPick | #285 | not built |
| Build the plugin page in Rune | `components/plugins/` | PluginPage | #286 | not built |
| Build the plugin side panel in Rune | `components/plugins/` | PluginSidePanel | #287 | not built |
| Build Site Controls and page settings in Rune | `components/site/` | SiteControls | #304 | not built |
| Build hidden elements and selection search overlays | `components/site/` | SitePageOverlays | #311 | not built |
| Build page prompts and the device chooser in Rune | `components/site/` | SitePrompts | #312 | not built |
| **Pages, bookmarks, downloads, menus and dev surfaces** | | | | |
| Wire developer tools and build the page source view | `components/dev/` | DevTools | #225 | not built |
| Build the shared internal page layout | `components/pages/` | none | #230 | library, not built |
| Build the app menu and dispatch browser shortcuts | `App.tsx`, `state/commands/`, `windows/Ardia/Menu*.h` | AppMenu | #249 | not built |
| Build the bookmarks bar | `components/bookmarks/` | BookmarksBar | #250 | not built |
| Build page, tab, sidebar, and link context menus | `App.tsx`, `state/commands/`, `windows/Ardia/Menu*.h` | ContextMenus | #251 | not built |
| Build the Report a Problem sheet | `components/report/` | ReportSheet | #252 | not built |
| Build the bookmark dialogs and folder picker | `components/bookmarks/` | BookmarkDialog | #263 | not built |
| Build the cascading bookmark folder menus | `components/bookmarks/` | BookmarkFolderMenus | #264 | not built |
| Build the downloads popover, button, and magnet | `components/downloads/` | DownloadsPopover | #265 | not built |
| Build the internal History page | `components/pages/` | HistoryPage | #266 | not built |
| Build the internal Bookmarks page | `components/pages/` | BookmarksPage | #288 | not built |
| Build the Clear Browsing Data dialog | `components/pages/` | ClearDataDialog | #289 | not built |
| Build the internal Downloads page | `components/pages/` | DownloadsPage | #290 | not built |

## Coverage of every Ardia component folder

| Ardia folder | Issues |
| --- | --- |
| `bookmarks/` | #250, #263, #264 |
| `dev/` | #225 |
| `downloads/` | #265 |
| `extensions/` | #244, #260 |
| `favicon/` | #222 |
| `find/` | #238 |
| `glyph/` | #222 |
| `iconPicker/` | #231 |
| `import/` | #241, #258 |
| `littleArdia/` | #292 |
| `live/` | #245 |
| `media/` | #246, #261 |
| `ntp/` | #293, #305 |
| `omnibox/` | #239, #255, #268 |
| `onboarding/` | #229, #242, #259, #282, #297, #298, #299, #300, #301, #302, #303, #308, #309, #310 |
| `pages/` | #230, #266, #288, #289, #290 |
| `peek/` | #306 |
| `plugins/` | #247, #248, #262, #284, #285, #286, #287 |
| `profiles/` | #243, #283 |
| `report/` | #252 |
| `settings/` | #219, #240, #256, #257, #269, #270, #271, #272, #273, #274, #275, #276, #277, #278, #279, #280, #281, #294, #295, #296, #307 |
| `sidebar/` | #224, #227, #228, #232, #233, #234, #235, #237, #253, #254, #267, #291 |
| `site/` | #304, #311, #312 |
| `split/` | #236 |
| `tabstrip/` | #316, #318, #320, #321 |
| `toolbar/` | #319, #322 |
| `ui/` | #217, #218, #220, #221, #223, #226 |
| `unread/` | #222 |
| `window/` | #313, #314, #315, #317, #323, #324 |

Surfaces that no component folder holds: the macOS main menu and its shortcuts (issue `app-menu-shortcuts`), right-click context menus (`context-menus`), the window root and overlay order (`App.tsx`, scaffold #217), and the secondary windows (`state/settings/settingsWindows.ts`, scaffold #217).

## How a screen worker proves its screen

1. `blaze check -p ria-ui` and `cargo clippy -p ria-ui --all-targets -- -D warnings` on Linux.
2. Bundle on the Mac under the Mac lock, run with `RIA_BACKGROUND=1`, a fresh `RIA_DATA_DIR` and `RIA_OPEN=<route>`, and screenshot the screen. Nothing comes to the front.
3. The PR body has the screenshot as a hosted image, next to the same Ardia screen when one exists, and every checklist line of the issue marked built or not built. The PR says `Refs #<issue>`.