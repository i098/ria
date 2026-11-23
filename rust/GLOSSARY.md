# Glossary: Ardia terms to Rust names

Ardia names come from `ardia/` at `551d3c13c`. Rust names follow the crate layout in the port plan, section 6.
A name marked [INFERENCE] is not yet in code.

| Ardia term | Where in Ardia | Meaning | Rust name |
| --- | --- | --- | --- |
| ArdiaCore | `packages/ardiacore`, `engine/ardiacore` | Window and tab layer between the app and Chromium (Objective-C++). | `ardia-host-mac` (calls), `ardia-engine` (typed C calls) |
| `ArdiaCore.h` | `engine/ardiacore` | Public Objective-C header, 180 methods. | `ardia-host-mac` `objc2` bindings |
| `ac_engine_call` | `engine/chromium` (`ac_engine.h`) | C entry point: name, profile directory, JSON arguments. | `ardia-engine-sys` `extern "C"`; `ardia-engine` typed functions |
| `ac_engine_set_event_sink` | `engine/chromium` | C entry point that registers the event callback. | `ardia-engine` event enum |
| engine | `engine/` | Chromium 158 with Helium and Ardia's patches. Stays non-Rust. | not ported |
| `ArdiaCoreTab.view` | `ArdiaCore.h:276` | Engine `NSView` that shows a page. | `ardia-host-mac` `PageView` [INFERENCE] |
| tab | `state/tabs` | One page in a window. | `ardia-tabs`, `ardia-model::Tab` |
| pin | `state/pins` | A tab kept in the sidebar. | `ardia-model::Pin` |
| group | `state/groups` | Folder of tabs. | `ardia-model::Group` |
| space | `state/sidebar` | A named set of tabs in the sidebar. | `ardia-model::Space` |
| split | `state/splits` | Two or more pages side by side. | `ardia-tabs::Split` [INFERENCE] |
| profile | `state/profiles` | One user data directory and its settings. | `ardia-profiles`, `ardia-model::Profile` |
| window | `state/windows` | One browser window. | `ardia-windows`, `ardia-model::Window` |
| omnibox | `packages/core`, `components/omnibox` | Address and search field with suggestions. | `ardia-core` (logic), `ardia-ui-omnibox` (view) |
| bang | `packages/core/src/bangList.ts` | `!` shortcut to a search site. | `ardia-core::bangs` |
| Peek | `state/peek`, `components/peek` | Preview card above a page. | `ardia-ui-overlays` |
| sidebar | `state/sidebar`, `components/sidebar` | Vertical list of spaces, pins and tabs. | `ardia-sidebar`, `ardia-ui-sidebar` |
| tab strip | `components/tabstrip` | Horizontal tab row. | `ardia-ui-tabstrip` |
| NTP | `components/ntp` | New tab page. | `ardia-ui-ntp` |
| control socket | `state/automation` | Unix socket with 129 methods. | `ardia-automation` |
| `ardia` command | `docs/cli.md` | Command-line client of the control socket. | unchanged |
| plugin | `packages/plugin-sdk` | JavaScript extension of the browser. | `ardia-plugins` |
| plugin broker | `packages/plugin-broker` | Rust jail that runs plugins. | moves into the workspace as it is |
| plugin host | `packages/shell/plugin-host` | Swift host of plugins. | `ardia-plugins` |
| shell | `packages/shell` | Swift window chrome, menus, native views, updater. | `ardia-host-mac`, `ardia-updater` |
| shaders | `packages/shaders` | Metal shader views. | `ardia-shaders` |
| sync | `packages/sync`, `state/sync` | Sync between devices. | `ardia-sync` |
| import | `packages/import`, `state/import` | Import data from other browsers. | `ardia-import` |
| telemetry | `state/telemetry` | Usage events with consent rules. | `ardia-telemetry` |
| Sparkle | `ardia/NOTICE` | Update framework. | `ardia-updater` (unsafe) |
| Little Ardia | `state/littleArdia` | Small separate window mode. [INFERENCE: meaning from the folder name] | `ardia-windows` [INFERENCE] |
| Developer Mode bar | `apps/browser/assets/developer-bar` | Bar for developer tools. | `ardia-ui-panels` [INFERENCE] |
| Talk to Me | `ardia/NOTICE` | iPhone voice feature. Out of scope. | none |
| React Native macOS host | `apps/browser/macos` | `main.swift` and `AppDelegate.swift`. | `ardia-app` |
| Zustand store | `apps/browser/src/state` | TypeScript state. | one owner on the main thread in each domain crate |
| Expo module | `packages/ardiacore` | Bridge between React Native and the engine. | deleted; no bridge in Rune |
| Rune | `anomalyco/rune` | UI framework used for all views. | `rune`, `embed::LayerHost` |
