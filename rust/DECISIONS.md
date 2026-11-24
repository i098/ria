# Ria decisions

Each decision is an ADR. Status of all: accepted on the captain's approval of the port plan.
Source: the port plan, section 9 (choices C1 to C10), and the captain's hard rule on animations.
Ardia means the original at `ardia/`, frozen at commit `551d3c13c`.

## ADR-C1: Rune is `anomalyco/rune`

- Context: two projects use the name "Rune". `anomalyco/rune` is a UI framework. `rune-rs/rune` is a script language.
- Decision: use `anomalyco/rune`, at one pinned revision.
- Consequences: only this one has an embed host (`embed::LayerHost`) for a native layer. Every UI crate depends on it through the pin. A change of revision is a reviewed change.

## ADR-C2: The port lives in the private repository `i098/ria`

- Context: rust-anything puts the port beside the original. A new repository keeps the original untouched.
- Decision: Ardia is a git submodule at `ardia/`, pinned at `551d3c13c`. The port's code is in `rust/`.
- Consequences: `record` and `diff` run the original from `ardia/`. We never push, open an issue or comment on `ardia-browser/ardia`. Each change to Ria starts as an issue in `i098/ria`, and its PR body has `Closes #N`.

## ADR-C3: First scope is the Mac desktop app only

- Context: Rune's embed host is Apple only. The Linux and Windows hosts are different designs.
- Decision: port the Mac desktop app. Linux, Windows, phone, site, dashboard and videos are out of scope.
- Consequences: `rust/MAP.md` lists only the Mac scope.

## ADR-C4: One process, Chromium owns the loop, Rune draws into a layer

- Context: the other option is two processes, with page surfaces that cross a process boundary.
- Decision: the Rust binary is the app's main executable. It runs the engine, as `main.swift` does today. Rune draws into a `CAMetalLayer` on an `NSView`. Pages are sibling engine views.
- Consequences: no new focus, input or latency problem from a process boundary. The host layer owns all unsafe code. Engine callbacks must never unwind into C++.

## ADR-C5: Rust calls the existing C and Objective-C engine interfaces with `objc2`

- Context: a new C interface to ArdiaCore needs an engine patch and a full Chromium build.
- Decision: on the Mac, each engine service is its own exported C symbol (`_ac_*`); `ac_engine_call` is built only on Linux. Find each symbol with `dlsym(RTLD_DEFAULT, name)`, as `packages/ardiacore/ios/ArdiaEngineBridge.mm:49-51` does. Call the `ArdiaCore.h` classes (Objective-C) with `objc2`. Use the prebuilt engine that `prebuilt-engines.tsv` pins.
- Consequences (added): `ria-engine` ports the name and argument mapping that the Swift and Objective-C++ host code does today (184 `api.ts` calls onto the 79 symbols that the Mac host uses, of 106 defined). The C event sink sends 13 topics; the other events come from `ArdiaCore.h` delegate callbacks.
- Consequences: no engine fork and no Chromium build. The engine stays non-Rust and unchanged.

## ADR-C6: Logic first with recorded proof, then UI

- Context: the other option is a new Rune app from zero with a parity check at the end. That gives no proof until the end.
- Decision: follow the rust-anything method. Record the original, port one unit at a time, and call a unit done only when `gate` passes. Port the UI after the logic.
- Consequences: each unit has recorded cases as proof. `rust/MAP.md` tracks each unit.

## ADR-C7: The original is frozen at `551d3c13c`

- Context: the Mac scope grew by about 170,000 lines in 7 days. Recordings go stale at each change.
- Decision: hold Ardia at `551d3c13c` until gate G6. Each gate writes a drift report (changed units and lines) in `rust/GATES.md`. A sync with a newer commit is a new plan after G6.
- Consequences: the submodule pin does not move before G6, so there is one parity target. If more than 407 of the 814 Mac units changed (50 %), the gate goes to the owner: continue to G6 on `551d3c13c`, or stop and plan again on a new commit.
- Changed in review round 2 (before: "Sync again at the end of each phase"). Reason: a sync at each phase gave 2 parity targets and had no hours.

## ADR-C8: Our own version, named Ria

- Context: Apache-2.0 section 6 gives no right to the Ardia name or logo.
- Decision: the name is Ria (owner, 2026-11-23), with a placeholder icon of its own. `NOTICE` credits Ardia. The Ardia name and icon are never the brand. The repository is `i098/ria`.
- Consequences: user-visible strings, bundle identifiers and icons use the Ria brand. Each crate has the prefix `ria-`. Names of the original keep the word Ardia (`ardia/`, `ArdiaCore.h`, the `ardia` command).

## ADR-C9: The port's own files are Apache-2.0

- Context: the other option is GPL-3.0-only for all files. Ardia's own source files are Apache-2.0.
- Decision: Ria's own files are Apache-2.0 (`LICENSE`).
- Consequences: the shipped app is GPL-3.0 in both cases, because the engine uses Helium's GPL-3.0 patches (`ardia/NOTICE`).

## ADR-C10: Swift native modules are ported to Rust with `objc2`

- Context: the other option keeps the Swift modules behind a C interface. That leaves two languages and a second build.
- Decision: port the Swift modules to Rust with `objc2`. Delete what Rune already gives; do not port it.
- Consequences: `rust/MAP.md` marks each Swift and Objective-C++ unit as port or delete.

## ADR-A1: Every animation matches the original

- Context: the captain's hard rule: "get all the animations right".
- Decision: each animation is recorded from Ardia and from Ria. Parity is proven with numbers: duration, curve, frame times and end state. A look-alike is not proof.
- Consequences: no UI surface is done until its animations have a recorded pair and a passing numeric comparison. The parity rig (`rust/parity/`) and the animation list (`rust/ANIMATIONS.md`) hold the method and the cases.

## ADR-P1: No build work before the owner approves the plan

- Context: Phase 0 started before the implementation plan was written.
- Decision: no worker starts build work before the owner approves the plan on the board. A slice starts only when each slice or gate in its `Needs` column is done.
- Consequences: a slice with an open `Needs` entry is not dispatched.

## ADR-P2: The port is a second app; Ardia stays the runnable app

- Context: the rust-anything whole-app tools (`app record`, `lift`) record a command-line program or an HTTP server. Ardia is a desktop app with a Unix socket.
- Decision: use rust-anything for each TypeScript unit only (`bridge`, `record`, `diff`, `gate`, `fuzz`). The app-level checks are ours: the replay drivers (`rust/replay/`), the parity rig (`rust/parity/`), the control-socket transcript and the harness of slice 3.13. There is no mixed app and no fallback to the original.
- Consequences: Ardia in `ardia/` is never changed. The cut-over is gate G6.

## ADR-S1: All state is one crate with no Rune type

- Context: the 41 state folders import each other in cycles (`rust-anything analyze`, field `internal_deps`). Rust crates cannot have cycles. Rune is 1 month old and has no release.
- Decision: one crate, `ria-state`, with one module for each state folder. Each module has `State`, `Action` and `apply(app, action, fx)`. The process has one `AppState` on the main thread, and `ria-app` owns it. The fields of `AppState` are private: `get::<M>()` reads a module, and `get_mut::<M>()` changes it and adds 1 to the change counter of that module. `apply` never calls native code: it puts `PlatformCall` values in `Effects`, and `ria-app` runs them through the one trait `Platform` after `apply` returns. A result of a platform call comes back as a second action. Only `ria-ui`, `ria-shaders` and `ria-surface` depend on Rune. `ria-ui` does not depend on `ria-surface`, so it compiles on Linux.
- Consequences: phases 1 and 2 do not depend on Rune, so a later change of the UI framework does not change their code. A replay driver uses a fake `Platform` that answers with the recorded results.

## ADR-S2: Two Rune layers for each window

- Context: page views are engine `NSView` objects beside the Rune layer. Some UI must draw above a page (omnibox list, find bar, Peek).
- Decision: each window has one `WindowHost` in `ria-surface`. It owns one `LayerHost` below the page views for the chrome and one overlay `LayerHost` above them, each in a `RefCell`. The overlay does not draw when no overlay shows. Each `LayerHost` has its own Rune runtime, and each runtime holds only a handle to the one `AppState`. `ria-surface` owns both layers, the input and IME routing, the display link (`NSView.displayLink(target:selector:)`, at most one tick in the queue for each window) and one accessibility adapter for each layer view. The overlay host view overrides `hitTest:`, calls `Runtime::element_at(x, y)` and returns `nil` when the result is `None`.
- Consequences: proofs B and C must pass before any UI slice starts. Proof C tests the 2-layer case and measures the load case with 2 layers in each of 3 windows before gate G0. Fallback: an overlay `LayerHost` that exists only while an overlay shows (alternative 11 of the plan). Fallback for the display link: `CVDisplayLink` (alternative 17). Rejected: one layer with a transparent hole (needs the alpha choice that the embed host does not have), a child `NSPanel` (changes the key window), and a state copy in each runtime (alternative 14).

## ADR-S3: A Rust panic never enters C++

- Context: one process holds the engine and the Rust app.
- Decision: each callback from C or Objective-C runs in `catch_unwind`. After a caught panic, the callback writes a crash report and calls `std::process::abort()`. The process never goes back into the engine, because `AppState` can be half changed. The saved session opens again on the next launch. A borrow conflict is not a panic path: the direct entries use `try_borrow` (ADR-S5).
- Consequences: the workspace keeps the Cargo default `panic = "unwind"` in written form, and no profile may set `abort`. `ria-host-mac`, `ria-engine-sys` and `ria-surface` have a test for this rule. Rejected: drop the frame and continue after a panic in `frame()` (alternative 16); gate G4 looks at the crash reports and can open this choice again.

## ADR-S4: The UI is one crate

- Context: the component folders import each other in cycles (`sidebar` and `tabstrip`, `ntp` and `onboarding`). Rust crates cannot have cycles.
- Decision: one crate, `ria-ui`, with one module for each of the 29 component folders.
- Consequences: there are no `ria-ui-*` crates.

## ADR-S5: Two kinds of entry into Rust; one action queue

- Context: an engine callback can arrive while Rust code is inside an engine call: a reply before the `_ac_*` call returns, a `loadProfile:completion:` block that runs at once, and tab delegate callbacks (`rust/research/engine-threads.md`). A reply can also arrive after the Rust timeout of its call. AppKit also calls into Rust and needs some answers at once (`hitTest:`, the `NSTextInputClient` queries, the accessibility tree queries, the size callbacks). Chromium owns the main loop.
- Decision: each entry into Rust on the main thread is of one of 2 kinds. A deferred entry (engine event, reply, completion block, delegate callback, `NSEvent`, IME commit, UI action, display link tick, Rune `Waker`, accessibility action) only copies its data and puts one item in the action queue of `ria-app`. A direct entry (`hitTest:`, IME queries, accessibility tree queries, size callbacks) reads and returns: it uses `try_borrow`, never calls `apply`, never calls the engine, and on a failed borrow gives a safe answer and adds 1 to the counter `reentry_miss`. The queue has one `CFRunLoopSource` on the main run loop in `kCFRunLoopCommonModes`, so it also drains during a live resize, menu tracking and a modal panel. No code holds an `AppState` borrow or a `LayerHost` borrow during an `_ac_*` call, an `ArdiaCore.h` call or an AppKit call. Only the reply trampoline frees a call context; the timeout only sets a shared settled cell. A secret call for a private profile is refused before the engine call, and each call uses the timeout value of Ardia's `ArdiaEngineBridge.mm`.
- Consequences: slice 3.1 has a test for a reply before the call returns and a test for a reply after the timeout. Slice 3.9 has one test for each direct entry, and `reentry_miss` is 0 in the tests. Proof B shows the drain during a live resize and during menu tracking.

## ADR-D1: Own identity and own data; ad hoc signature

- Context: the port has a new name. A second app that opens an Ardia profile can damage it. No Apple developer account is used.
- Decision: Ria uses its own bundle ID and data folder. It never opens an Ardia profile folder. Parity runs use a temporary profile. Ria always passes `--use-mock-keychain` to the engine. Ardia uses this switch in builds that do not have a team signature. Ria builds use an ad hoc signature under this plan.
- Consequences: the port and Ardia share no keychain item. The passwords and cookies in the data folder are encrypted with a fixed mock key; only the file permissions and the disk encryption of the Mac protect them. Passkeys through iCloud Keychain do not work. A team-signed build is not in this plan. A later team-signed build cannot read the passwords and cookies that the mock key encrypted (an inference, not tested), and the plan has no migration step.

## ADR-D2: No call to the original owner's servers

- Context: Ardia calls 5 `ardia.dev` addresses (telemetry export, update feed, feedback API, switches, release notes).
- Decision: one file, `ria-core/src/endpoints.rs`, has 5 constants (`TELEMETRY_EXPORT`, `UPDATE_FEED`, `FEEDBACK_API`, `SWITCHES`, `RELEASE_NOTES`). Each constant is `""`, with the original address in a comment. Each of the 4 senders (all in `ria-state`) makes no platform call when its address is empty. `UPDATE_FEED` has no sender, because the updater is not ported.
- Consequences: an address is set only by a later owner decision. Pages, search suggestions, sync and plugins use the network as in Ardia. The telemetry text that says data goes to `ardia.dev` changes: it says that Ria sends no telemetry. The setup code text that names `ardia.dev` stays, because the app sends no request there. The queue label `ardia.dev-applescript` becomes `ria-applescript`.

## ADR-D3: The updater is not ported

- Context: the owner builds the app from source, and the update feed is empty (ADR-D2).
- Decision: the 25 updater units (1,726 lines) have the disposition `later`.
- Consequences: there is no crate `ria-updater`. The feature row "Auto-updates" has the status `not ported` at gate G6. A later plan can add it.

## ADR-D4: Two builds from one source

- Context: the test scripts need probes that read app state. The app that the owner uses must not have them.
- Decision: slice 6.1 makes a probe build (parity runs) and a use build. The probe build has 3 general probes (`state`, `dispatch`, `engine.call`), and a named probe only where no general probe can serve. The use build refuses each probe method with `unknown_method`, and a Rust test checks this.
- Consequences: gate G6 runs the parity checks on the probe build and the resource check on the use build.

## ADR-W1: The WebKit page engine is in scope

- Context: Ardia has 2 page engines (`PageEngine = "chromium" | "webkit"`). The WebKit path is 40 units and 14,172 lines in `packages/ardiacore/ios/`.
- Decision: port it to the crate `ria-webkit` in Phase 5 (slice 5.1).
- Consequences: parity includes the sites that Ardia opens in WebKit.

## ADR-Q1: Sentrux baseline of 2026-11-24

- Context: the baseline of slice 1.x (cycles 2, god files 0, complex functions 120) was stale. At `origin/main` 01e369e `sentrux gate .` gave DEGRADED: cycles 3, god files 1, complex functions 210. CI did not run the gate.
- Decision: (1) The new cycle was a structure fault of Ria: `engine.rs` called `host::hard_exit` while `host.rs` imports `engine`. `hard_exit` now lives in `stderr.rs` and the `TERMINATE` flag in `exit.rs`, with no change of behavior; cycles are 2 again. (2) The god file is `ria-state/src/kernel/modules.rs`, the single list of the 41 state modules (plan section 6.5). Its fan-out is its purpose, and a split would only move the imports, so it stays. (3) The complex functions are faithful ports of Ardia functions, kept one to one with the source. Complex functions by crate (counts removed when the crate is left out of the scan): `ria-core` 30, `ria-host-mac` 29, `ria-plugin-broker` 11, `rust/proofs` 7, `ria-engine` 4, `ria-surface` 2, `rust/tools` 2, `rust/replay` 1; the other crates 0. (4) The baseline is saved with `sentrux gate --save .`: cycles 2 to 2, god files 0 to 1, complex functions 120 to 210, quality 5297 to 5518.
- Consequences: from this baseline, any rise of cycles, god files or complex functions fails the `sentrux` CI job. Save a new baseline only with a new entry here.
- Update 2026-11-24 (after PR 118, window gaps, and PR 122): the gate at `origin/main` 864d7a3 plus this change reported complex functions 210 to 213. The 3 new ones come from three files that PR 118 grew. Removing each file and scanning again gives the counts. `ria-host-mac/src/windows/ardia_window_full_screen.rs` (port of `ArdiaWindowFullScreen.mm`): 4 complex functions in the file; its new Ardia-port functions `sync_page`, `picture_page` and `notified` are the cause. `ria-host-mac/src/windows/ardia_chrome_window_host_dev.rs` (port of `ArdiaChromeWindowHost+Dev.mm`): 1, the dev command `run`. `ria-host-mac/tests/live_windows.rs` (Ria's own live check, no Ardia source): 1, `check_windows`, which was already long before PR 118 and runs the checks in order. The two ported files are one to one with Ardia, so they stay. The names are from a count of branches in the diff, not from a sentrux list (sentrux prints no function names) [INFERENCE]. The baseline is saved again: complex functions 213, quality 5520.

## ADR-V1: An inline `data:` favicon is limited to 2 MiB (deviation from Ardia)

- Context: `ArdiaFavicons.mm:97-101` (at `551d3c13c`) reads a `data:image/` URL with `dataWithContentsOfURL:` and no byte limit. Only the HTTP path (`Download`, line 80) passes `kMaxBytes`. Issue 58.
- Decision: `favicons.rs` refuses a `data:` body longer than the largest encoding of `MAX_BYTES` before it allocates, and refuses a decoded result over `MAX_BYTES` (2 MiB), the budget of the HTTP path. The decoder also refuses an image wider or taller than `MAX_SIDE` (4096 px): exactly 4096 decodes, 4097 is refused. The owner's order to resolve every open issue approves this deviation.
- Consequences: an oversized inline icon gives `None`, as an oversized download does. Ardia keeps the fault.

## ADR-V2: Content blocker start keeps a `Built-in Extensions` folder (deviation from Ardia)

- Context: `ArdiaContentBlocker.mm:46-48` (at `551d3c13c`) makes `StartPreparing` delete `Built-in Extensions` next to the data directory on a utility queue, even when the bundle lookup fails. No tracked Ria code creates that folder. Issue 68.
- Decision: `content_blocker.rs` `extension_path` only finds the bundled extension and deletes nothing. The owner's order to resolve every open issue approves this deviation.
- Consequences: an old `Built-in Extensions` folder stays on disk. Ardia keeps the fault.

## ADR-V3: A released private session no longer serves its cached favicon (deviation from Ardia)

- Context: `ArdiaFavicons.mm:90` (at `551d3c13c`) stores the image in the process-wide `Seen()` cache under `SeenKey(profile, url)`, and line 98 returns it before any live-profile check. `ArdiaHost.mm:143-155` (`releaseProfile:`) does not clear that cache. Issue 57.
- Decision: `Favicons::fetch` in `favicons.rs` skips the `Seen()` cache read for a private session the app released (`released_private` in the profile registry), and `note_image` does not store an icon that arrives late for such a session. `data:` and download paths are unchanged. The owner's order to resolve every open issue approves this deviation.
- Consequences: after release, a cached `favicons.fetch` for that session misses the cache. An image cached before the release stays in memory until evicted, but nothing reads it; a late icon is not stored. Ardia keeps the fault.

## ADR-V4: A private window cannot restore a hidden element (deviation from Ardia)

- Context: `ArdiaContentBlocker.mm:534-539` (at `551d3c13c`) makes `restoreElements:inProfile:` call `OriginalProfileName(profile)`, so a private session removes a filter from its regular profile and saves the filter file. The add path refuses private writes. Issue 56.
- Decision: `restore_elements` in `content_blocker.rs` refuses a private profile with the same error as `hide_elements`. The owner's order to resolve every open issue approves this deviation.
- Consequences: a private window cannot remove a saved hidden element; reading regular-profile filters is unchanged. Ardia keeps the fault.
