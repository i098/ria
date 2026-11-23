# Rune embed host on macOS: `LayerHost`

Scope: how Rune draws into a layer that another program owns. Research only. No code.

## Source and pin

- Rune: `anomalyco/rune`, pinned at rev `ada31dc5bf33bdf73d56df54696131b7bcd41ee6` (commit date 2026-11-22). Short name `rune@ada31dc`.
  I cloned it with `git clone --depth 1` and read `HEAD` with `git log -1`.
- Path prefix `R/` below means `crates/rune/src/` in that clone. `G/` means `crates/rune-gpu/src/`. `E/` means `crates/rune-edit/src/`.
- Ardia: frozen at `551d3c13c1c9ede130724ea9f1056d4b586fbe55` (`git rev-parse HEAD` in the read-only clone). Short name `ardia@551d3c1`.
- Anything without a line cite is marked `[INFERENCE]`.

## 1. What `LayerHost` is

- Rune ships an embed host for UIKit, SwiftUI and AppKit views that own their own window and loop. `R/platform/embed.rs:1-2`.
- It is compiled for Apple targets only: `pub mod embed` under `#[cfg(target_vendor = "apple")]` at `R/platform.rs:7-8`.
- Rune added it in the changelog entry `changelog/unreleased/2026-10-30-ios-embed.md:1`. The file says it mounts a `Runtime` and a Metal `Renderer` onto an existing `CAMetalLayer`.
- A second changelog entry makes the frame run "the way a window does": it takes the drawable before it steps, so a frame with no drawable does not advance animations (`changelog/unreleased/2026-11-19-apple-embed-frame-pipeline.md:1`).
- The only in-tree user is the example `crates/rune/examples/embed_cost.rs:109-136`. It runs on a `CAMetalLayer` that is in no window. It is a cost bench, not an app shell. Nothing in the repo embeds `LayerHost` beside a foreign view.

## 2. The API

| Item | Signature (file:line) | Meaning |
| --- | --- | --- |
| `LayerHost::from_core_animation_layer` | `R/platform/embed.rs:212-220` | `unsafe`. Takes `layer: *mut c_void`, `&AppSpec<S>`, `state: S`, `viewport: (f32, f32)` in points, `scale: f32`, `dark: bool`, `wake: Waker`. Returns `Result<Self, String>`. |
| Safety contract | `R/platform/embed.rs:209-211` | `layer` is a valid `CAMetalLayer`. It outlives the host. All calls run on the UI thread. |
| `with_compositor` | `R/platform/embed.rs:230-234` | Attach a `Compositor` for `El::effect` shader passes. Sets `track_marks`. |
| `runtime()` / `runtime_mut()` | `R/platform/embed.rs:236-242` | Input, a11y and settings go through the `Runtime`. |
| `resize(viewport, scale)` | `R/platform/embed.rs:244-248` | Points plus scale. Reconfigures the swap chain only when the pixel size changes. |
| `frame() -> EmbedFrame` | `R/platform/embed.rs:250-254` | One step plus one present. |
| `EmbedFrame` | `R/platform/embed.rs:42-51` | `drew`, `animating`, `wake_after_secs`, `cpu_us`. |
| `embed::prewarm()` | `R/platform/embed.rs:29-38` | Warms the shared Metal device on a thread named `rune-embed-prewarm`. |
| `Waker` | `crates/rune-core/src/lib.rs:50` | `Arc<dyn Fn() + Send + Sync>`. It runs on any thread. See the frame clock section. |

`AppSpec` and `runtime_for` come from `R/tooling` (`R/platform/embed.rs:16`). The view function is the same one a window app uses (`crates/rune/examples/embed_cost.rs:20-47`).

How `LayerHost` starts (`R/platform/embed.rs:82-115`):

1. It rejects a null layer (line 83).
2. It takes one process-wide shared `Gpu` from a `OnceLock` (lines 19-25, 86).
3. It calls `create_surface_unsafe(CoreAnimationLayer(layer))` (lines 88-92).
4. It picks the surface config with the same `surface_config` the window path uses (line 98 and `R/window/surface.rs:53-78`).
5. It forces sRGB blend on (lines 97, 103, 118).

## 3. GPU backend

- On macOS the default backend is Rune's own Metal backend (`metal-backend` feature on). `G/backend.rs:29-30`.
- The default of the enum is `Wgpu`, and `GpuBackend::Metal` opts in to the native one (`G/backend.rs:12-19`).
- A host that embeds `LayerHost` must call `prewarm_gpu_on(...)` before its first host if it wants to choose. The doc says this for iOS (`R/render/device.rs:319-326`). The macOS case [INFERENCE]: `choose_gpu_backend` restarts the prewarm thread on macOS when the backend changes (`R/render/device.rs:336-341`). So a late choice costs a restart, not an error.
- The native Metal surface sets these layer properties in `configure` (`G/metal/surface.rs:99-125`):
  - pixel format (line 106) and `framebufferOnly` (line 107);
  - `setOpaque` only for `Opaque` or `PostMultiplied` alpha (lines 108-112);
  - `maximumDrawableCount` of 2 or 3 (line 115);
  - `drawableSize` in device pixels (line 116);
  - on macOS, `allowsNextDrawableTimeout = false` and `displaySyncEnabled` from the present mode (lines 120-124).
- It never sets `contentsScale`, `frame` or `bounds` of the layer. The host owns them. [INFERENCE] from the same function: no such call exists in `configure`.
- On macOS `get_current_texture` returns `Occluded` when the hosting window is fully covered or hidden (`G/metal/surface.rs:74-81, 133`). The occlusion check walks the layer, its delegate and superlayers to find the `NSWindow` (`G/metal/surface.rs:60-69`). A layer in no window never reports occluded.
- Present mode is `AutoVsync` and the frame latency is 2 on non-Windows (`R/window/surface.rs:49, 76-77`).

## 4. Input

`LayerHost` has no input method of its own. The host forwards events to `runtime_mut()`. All coordinates are points in the Rune view, top-left origin. [INFERENCE] for top-left: the example uses `viewport` in points and the material host in `R/system_material/macos.rs:21-22` says Rune's points are top-left.

| Need | `Runtime` call (`R/runtime/input/api.rs`) |
| --- | --- |
| pointer move, down, up, leave | `pointer_move` 21, `pointer_down` 45, `pointer_up` 51, `pointer_leave` 64 |
| other buttons | `button_down` 109, `button_up` 114 |
| wheel (line and pixel) | `wheel` 74, `wheel_xy` 137, `wheel_began` 139, `wheel_ended` 141, `set_wheel_glide` 76 |
| trackpad fling | `fling` 78, `fling_xy` 143 |
| pinch, rotate, smart magnify | `magnify_event` 101, `rotate_event` 103, `smart_magnify` 105 |
| keys | `key` 155, `repeat_key` 157, `key_up` 119. `KeyEvent { key: Key, mods: Mods }` is at `E/keys.rs:202-205`. `Key::Text(String)` is the typed text (`E/keys.rs:115-116`). |
| paste and drop | `paste` 163, `paste_content` 165, `drop_files` 220 |
| window focus | `window_focus(bool)` at `R/runtime/api.rs:86` |
| read the cursor shape | `cursor()` at line 72. The host applies it to `NSCursor`. |
| hit test for foreign views | `hit_regions()` at line 70: "native overlay regions" in window points (`R/runtime/input/pointer.rs:461-463`). |

What the host must do itself [INFERENCE: no embed code reads these]:

- Own an `NSView` that hosts the layer and forwards `NSEvent` to the calls above.
- Translate `NSEvent` to `Key` and `Mods`, and decide Cmd-key routing.
- Apply `cursor()` to `NSCursor`.

Chromium reserved keys: Ardia asks the engine delegate first (`ArdiaCore.h:31-34`, `engineIsReservedKeyEvent`). The Rune host must run that check before `key()`.

## 5. IME

What Rune offers:

- The `Ime` enum is `Preedit(String, Option<(usize, usize)>, Vec<Clause>)`, `Commit(String)`, `Select(Range<usize>)`, `Enabled`, `Disabled` (`E/ime.rs:13-30`, `Preedit` at line 19, `Select` at line 25).
- `Runtime::ime(Ime)` and `Runtime::finish_composition()` accept them (`R/runtime/input/api.rs:159-161`).

What is missing for an embed host:

- On a window, the macOS IME runs through a vendored `winit` `NSTextInputClient`. Rune reads the clauses with `winit::platform::macos::take_ime_clauses` and the reconversion range with `take_ime_replacement`, and publishes the field text with `set_ime_text` (`R/platform/ime.rs:15-46`). There is no `winit` view in an embed, so none of this runs.
- The caret rect for the candidate window is set by `window.set_ime_cursor_area(...)` in `sync_ime` (`R/input/host_events.rs:507-533`). `sync_ime` takes a `&Window`, so an embed cannot call it. The values it uses (`ime_area`, `ime_text`) are fields or `pub(crate)` functions: `ime_text` is `pub(crate)` at `R/runtime/input/keys.rs:378`, and the `ime_area` field is set in `R/runtime/paint/build.rs:498-499`. I found no public getter. So the host cannot read the caret rect. This is a real gap.
- The window path turns IME on and off with `set_ime_allowed` (`R/input/host_events.rs:513-516`). `Ime::Enabled` and `Ime::Disabled` are only informational (`E/ime.rs:26-30`). The host must decide when its `NSTextInputClient` is active. [INFERENCE]: with no getter it cannot know when a Rune text field has focus.
- Option-as-Alt follows the focused field (`R/input/host_events.rs:518-525`). Same gap.

Net: the host has to implement `NSTextInputClient` on its own view: `setMarkedText:`, `insertText:`, `markedRange`, `selectedRange`, `firstRectForCharacterRange:`. It feeds `Ime::Preedit` and `Ime::Commit`. It has no way to read the caret rect until Rune adds a public getter.

## 6. Resize and scale

- `LayerHost::resize` takes points and scale. It clamps pixels to the device limit, and it sets `suspended` for a zero size (`R/platform/embed.rs:126-144`).
- When the pixel size changes, it reconfigures the surface (lines 136-142). A refusal is printed, not returned (line 140).
- It then calls `rt.set_viewport` with the clamped size (line 143).
- The host must update the `CAMetalLayer` `frame` and `contentsScale` itself. [INFERENCE]: nothing in `embed.rs` or `configure` touches them.
- A live resize with a layer in the AppKit tree can show stretched content for one frame. `presentsWithTransaction` is the usual fix. In a window, Rune turns it on for native materials and off when the live resize ends (`R/system_material/macos.rs:5-9, 280`). `LayerHost` does neither. [INFERENCE] open question for proof B.

## 7. Frame clock and pacing

- `frame()` calls `run_embed_frame` (`R/platform/embed.rs:252-254`), which runs the shared `host_step` sequence (`R/platform/embed_frame.rs:153-163`).
- `LayerHost` has no display link. The host calls `frame()` from its own link. The example loops without one (`crates/rune/examples/embed_cost.rs:117-127`).
- The Rune window path has a `CVDisplayLink` wrapper, but it is `pub(crate)` and gated to macOS windows (`R/platform.rs:5-6`, `R/platform/display_link.rs:1-6`). Its doc says Metal's drawable pool is not a frame clock, and one submission per tick is the rule. An embed host must follow the same rule on its own. [INFERENCE]: on a 120 Hz panel in Low Power Mode, `CVDisplayLink` still reports 120 Hz while the cap is 60 Hz (the doc at `display_link.rs` lines 1-12 says so).
- Embed passes no presentation timestamp: `presents_at: None` at `R/platform/embed_frame.rs:89`. Time steps from `crate::time::now()` (line 78-79).
- The step rule: running motion gets the real gap, at most `MAX_GAP = 0.25 s`. New motion gets at most `RESUME_STEP = 1/120 s`. (`R/frame/clock.rs:22-32, 52-57`.)
- `EmbedFrame.animating` tells the host to keep ticking. `wake_after_secs` tells it the next timer wake (`R/platform/embed.rs:45-47`). With nothing animating, the host should stop its link and restart it on `wake`. [INFERENCE]: that is the purpose of the two fields. The `Waker` runs on any thread (`rune-core/src/lib.rs:50`), so it must hop to the main thread.
- A frame with no drawable does not advance animation (`changelog/unreleased/2026-11-19-apple-embed-frame-pipeline.md:1`; code at `R/platform/embed.rs:146-163`, `R/platform/embed_frame.rs:112-122`).
- The embed also drops work it cannot serve: popup menus, file dialogs, minimize and window-frame requests (`R/platform/embed_frame.rs:165-176`). A Rune app inside Ria cannot open a native menu or dialog by itself.

## 8. Accessibility

- Rune builds an AccessKit tree and, with the `accessibility` feature, a macOS adapter (`crates/rune/Cargo.toml:76`, `R/a11y/macos.rs:1-2`).
- That adapter needs a `winit::window::Window` and subclasses winit's content view (`R/a11y/macos.rs:14-16`; `R/a11y/mod.rs:222-227`). An embed has no `winit` window. So `LayerHost` has no accessibility adapter.
- Public hooks exist: `Runtime::accessibility_tree() -> accesskit::TreeUpdate` (`R/runtime/api.rs:417`) and `Runtime::accessibility_action(key, action, data)` (`R/runtime/api.rs:401`).
- Plan: the host runs `accesskit_macos` on its own `NSView` and wires the two calls. [INFERENCE]: nothing in the tree does this. It is needed for proof C. Open question: how the tree update gets pushed when Rune state changes. The window path does this inside the frame loop (`R/a11y/mod.rs:252-256` `send`), and `run_embed_frame` does not call it.
- `Runtime::accessibility_json`/`a11y_json` also exist (`R/runtime/api.rs:419-421`) and are usable for tests without VoiceOver.

## 9. Translucency, glass and layering

- Rune's translucent window path picks an alpha mode with `render::translucency::mode` (`R/window/surface.rs:78`). `LayerHost` throws that result away: `let (config, _) = crate::window::surface_config(...)` at `R/platform/embed.rs:98`.
- The embed config is `alpha_mode: Auto`, from wgpu's default config (`wgpu-30.0.1/src/api/surface.rs:98`, in the cargo registry). On Rune's Metal surface, `Auto` falls into the `_ => {}` arm and does not touch `setOpaque` (`G/metal/surface.rs:108-112`). [INFERENCE] result: the layer keeps whatever `opaque` the host set. `CALayer` defaults to not opaque.
- The clear color is `[0;4]` only when `RuntimeCore.transparent` is true (`R/runtime/paint/blend.rs:42-43`; field at `R/runtime.rs:339-340`). The window path sets it from the app options (`R/window/host/create.rs:159, 359`). I found no public setter on `LayerHost`. [INFERENCE] open question: whether `Runtime` exposes `transparent`. Without it, the clear color is the root fill colour, which is opaque unless the app fills with alpha.
- Rune's own "native material" view path (glass under the layer, in a window) uses a `RuneMaterialHost` `NSView` under winit's layer, with `presentsWithTransaction` (`R/system_material/macos.rs:1-35`). It needs a winit content view. `LayerHost` does not call it.
- Ardia's current host already embeds one foreign root view in the Chromium window. `embedRootView:inWindow:` sets `autoresizingMask` to width and height sizable, sets the frame to `hostView.bounds`, and adds the root with `addSubview:positioned:NSWindowBelow relativeTo:cover` (or plain `addSubview:`) (`ardia@551d3c1:packages/ardiacore/ios/ArdiaChromeWindowHost.mm:377-388`). `hostView` is the `ArdiaCoreWindow` host view (`ArdiaCore.h:187`). A page view is a subview that `ArdiaWebView` adds with `addSubview:page` (`ardia@551d3c1:packages/ardiacore/ios/ArdiaWebView+Tab.mm:109`). Ria's Rune view takes the place of that root. The z-order of Rune against the page views is a design choice that proof C has to settle.

## 10. What is missing (list)

| # | Gap | Evidence | Needed for |
| --- | --- | --- | --- |
| 1 | No IME caret rect or "text field focused" getter | `R/runtime/input/keys.rs:378` is `pub(crate)`; no `pub fn ime_*` in `R/runtime/input/api.rs` | B |
| 2 | No `NSTextInputClient`, clause or reconversion path | `R/platform/ime.rs:15-46` needs winit | B |
| 3 | No accessibility adapter for embed; no tree push in `frame()` | `R/a11y/mod.rs:222-227`, `R/platform/embed_frame.rs:78-91` | C |
| 4 | No translucent or premultiplied alpha choice | `R/platform/embed.rs:98` drops it | C |
| 5 | No presentation timestamp, no display link | `R/platform/embed_frame.rs:89`, `R/platform/display_link.rs` is `pub(crate)` | B |
| 6 | No `NSEvent` to `Key` translation | `E/keys.rs:115-205` | B |
| 7 | No popups, file dialogs, native menus, window frame | `R/platform/embed_frame.rs:165-176` | B (later phases) |
| 8 | No native glass or material | `R/system_material/macos.rs:3-9` needs winit | C |
| 9 | Layer `frame`, `contentsScale`, live-resize transaction | `G/metal/surface.rs:99-125` | B |
| 10 | Dark mode, accent, reduced motion are not wired on embed | `LayerHost::new` takes `dark` once (`R/platform/embed.rs:218`). Setters exist: `set_system_dark` etc. (`R/runtime/api.rs:206-238`). The window path refreshes them on focus (`R/input/host_events.rs:537-545`). | B |

## 11. Open risks

### Proof A (Rust starts the prebuilt engine, one window, one tab, quit clean)

- Rune is not involved in A. The risk is the host loop: Chromium runs `[NSApp run]` itself (`docs/ardiacore-spike.md:40-42`), so Rust must call `runWithArgc` and run inside that loop. See `engine-threads.md`.

### Proof B (LayerHost beside the page view: pointer, wheel, keys, IME, resize, frame times)

1. **IME is the largest gap** (gaps 1 and 2). Without a caret rect getter the candidate window cannot follow the caret. Test early with Japanese Hiragana and a Chinese pinyin input.
2. **Frame pacing.** No display link and no timestamps (gap 5). Measure frame times at 60 and 120 Hz with `EmbedFrame.cpu_us`, and check for duplicate or dropped drawables. The Mac stop rule applies to frame-time claims.
3. **Live resize.** Test a drag resize for tearing. Check whether `presentsWithTransaction` is needed (gap 9).
4. **Event routing.** Rune view and Chromium page views are siblings. Pointer events must reach the right one (`hit_regions`, hit test order). `ArdiaCorePageView.takeSwipes:` and the scroll observer exist for the page side (`ArdiaCore.h:401-415`).
5. **Text shaping.** Rune uses Parley. Ardia uses the system text engine. Check text first (plan risk R5, `report.md:539`).
6. **Threading of `Waker`.** It runs on any thread. It must hop to the main thread before `frame()`.
7. **Shared GPU.** `LayerHost` shares one `Gpu` per process (`R/platform/embed.rs:19-25`). Chromium has its own GPU process, so there is no clash. `[INFERENCE]`: the Rune Metal device is in the browser process and costs memory. Measure the footprint with `embed_cost`.

### Proof C (Rune layer above the page view, glass under, VoiceOver reads one Rune button)

1. **Transparency may be impossible without a Rune patch** (gap 4, section 9). The test is the first thing to run: render a half-transparent fill and look at what shows through. If it fails, the plan stops (`report.md:387`) unless Rune adds the option.
2. **Glass.** Rune's material path needs winit (gap 8). Plan: a host-owned `NSVisualEffectView` below the layer. [INFERENCE] I did not check how Ardia builds glass today.
3. **VoiceOver.** The host runs `accesskit_macos` itself (gap 3). `accesskit_macos` is vendored and patched in Rune (`vendor/accesskit_macos.patch`, size 46.4K in `ls vendor`). Rune's patch may be needed. Check the version: `Cargo.toml:153` pins `accesskit_macos = 0.27.2`.
4. **Z-order with Chromium views.** A Rune layer above a page view needs real alpha (item 1). A Rune layer below it needs the page view to cover only its own rect. Ardia adds page views with `addSubview:page` (`ArdiaWebView+Tab.mm:109`) and the root with `NSWindowBelow` a cover (`ArdiaChromeWindowHost.mm:384`). Check the real hierarchy before design.
