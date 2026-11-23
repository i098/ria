# Glossary: Ardia terms to Rust names

Ardia names come from `ardia/` at `551d3c13c`. Rust names follow the crate layout in the port plan, section 6 (plan version 5).
A name marked [INFERENCE] is not yet in code.

| Ardia term | Where in Ardia | Meaning | Rust name |
| --- | --- | --- | --- |
| ArdiaCore | `packages/ardiacore`, `engine/ardiacore` | Window and tab layer between the app and Chromium (Objective-C++). | `ria-host-mac` (calls), `ria-engine` (typed C calls) |
| `ArdiaCore.h` | `engine/ardiacore` | Public Objective-C header: 417 lines, 180 method lines, 3 delegate protocols and the `ArdiaCorePageScrollObserver` protocol. | `ria-host-mac` `objc2` bindings |
| `ac_engine_call` | `engine/chromium` (`ac_engine.h`) | C entry point on Linux only. On the Mac each service is its own `_ac_*` symbol, found with `dlsym`. | `ria-engine-sys` (symbol loader); `ria-engine` typed functions |
| `ac_engine_set_event_sink` | `engine/chromium` | C entry point that registers the event callback. It sends 13 topics. | `ria-engine` event enum (with the delegate callbacks) |
| engine | `engine/` | Chromium 158 with Helium and Ardia's patches. Stays non-Rust. | not ported |
| `ArdiaCoreTab.view` | `ArdiaCore.h:276` | Engine `NSView` that shows a page. | `ria-host-mac` `PageView` [INFERENCE] |
| tab | `state/tabs` | One page in a window. | `ria-state::tabs` |
| pin | `state/pins` | A tab kept in the sidebar. | `ria-state::pins` |
| group | `state/groups` | Folder of tabs. | `ria-state::groups` |
| space | `state/sidebar` | A named set of tabs in the sidebar. | `ria-state::sidebar` |
| split | `state/splits` | Two or more pages side by side. | `ria-state::splits` |
| profile | `state/profiles` | One user data directory and its settings. | `ria-state::profiles` |
| window | `state/windows` | One browser window. | `ria-state::windows` |
| omnibox | `packages/core`, `components/omnibox` | Address and search field with suggestions. | `ria-core::omnibox` (logic), `ria-ui::omnibox` (view) |
| bang | `packages/core/src/bangList.ts` | `!` shortcut to a search site. | `ria-core::bangs` |
| Peek | `state/peek`, `components/peek` | Preview card above a page. | `ria-ui::peek` |
| sidebar | `state/sidebar`, `components/sidebar` | Vertical list of spaces, pins and tabs. | `ria-state::sidebar`, `ria-ui::sidebar` |
| tab strip | `components/tabstrip` | Horizontal tab row. | `ria-ui::tabstrip` |
| NTP | `components/ntp` | New tab page. | `ria-ui::ntp` |
| control socket | `state/automation` | Unix socket with JSON-line requests. `rust/research/control-socket.md` has the method list. | `ria-state::automation` (methods), `ria-app` (server) |
| `ardia` command | `docs/cli.md` | Command-line client of the control socket. | unchanged |
| plugin | `packages/plugin-sdk` | JavaScript extension of the browser. The SDK stays JavaScript. | `ria-plugins` (host only) |
| plugin broker | `packages/plugin-broker` | Rust jail that runs plugins. | moves into the workspace as it is |
| plugin host | `packages/shell/plugin-host` | Swift host of plugins. | `ria-plugins` |
| shell | `packages/shell` | Swift window chrome, menus and native views. The updater is not ported (ADR-D3). | `ria-host-mac`, `ria-services-mac`, `ria-plugins` |
| shaders | `packages/shaders` | Metal shader views. | `ria-shaders` |
| sync | `packages/sync`, `state/sync` | Sync between devices. | `ria-sync` |
| import | `packages/import`, `state/import` | Import data from other browsers. | `ria-import` |
| telemetry | `state/telemetry` | Usage events with consent rules. The export address is empty (ADR-D2). | `ria-state::telemetry` |
| Sparkle | `ardia/NOTICE` | Update framework. Not ported (ADR-D3). | none |
| Little Ardia | `state/littleArdia` | Small separate window mode. [INFERENCE: meaning from the folder name] | `ria-state::little_ardia`, `ria-ui::little_ardia` |
| Developer Mode bar | `apps/browser/assets/developer-bar` | Bar for developer tools. | `ria-ui::dev` [INFERENCE] |
| Talk to Me | `ardia/NOTICE` | iPhone voice feature. Out of scope. | none |
| React Native macOS host | `apps/browser/macos` | `main.swift` and `AppDelegate.swift`. | `ria-app` |
| Zustand store | `apps/browser/src/state` | TypeScript state. | one module in `ria-state` with `State`, `Action` and `apply` |
| Expo module | `packages/ardiacore` | Bridge between React Native and the engine. | deleted; no bridge in Rune |
| Rune | `anomalyco/rune` | UI framework used for all views. | `rune`, `embed::LayerHost` |
| WebKit page engine | `packages/ardiacore/ios/WebKit*` | Second page engine, used for some sites. | `ria-webkit` |
| replay driver | none (new) | A pure `replay(actions[])` function for one state folder, used to record cases. | `rust/replay/<folder>/` |
| platform call | native modules (`packages/shell/src`, `packages/ardiacore/src`) | One native call that a store asks for. A value of the enum `PlatformCall`; the trait `Platform` runs it. | `ria-state::platform` |
| action queue | none (new) | The one queue for deferred entries (engine callbacks, input events, UI actions, ticks, platform results). A `CFRunLoopSource` in the common modes drains it (ADR-S5). | `ria-app` |
| probe | none (new) | A read or an action on a socket method that only the probe build has. The test scripts use it. 3 general probes (`state`, `dispatch`, `engine.call`), and named probes for the other cases. | `ria-app` `src/probes/` |
| surface host | none (new) | `WindowHost`: the 2 Rune layers of a window with input and accessibility. | `ria-surface` |
| deferred entry, direct entry | none (new) | The 2 kinds of entry into Rust on the main thread (ADR-S5). | `ria-app`, `ria-surface` |
| trigger | none (new) | One file that starts one animation row on Ardia and on the port, with no `eval` call. | `rust/parity/triggers/<row id>.mjs` |
| gate record | none (new) | The section of one gate in `rust/GATES.md`. | `rust/GATES.md` |
| probe build, use build | none (new) | The 2 builds of slice 6.1. Only the probe build has probes. | `rust/bundle/` |
