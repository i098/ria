# Parity rig

Measures Ardia (the reference) and later Ria with the same tools, then compares the numbers.
Frozen reference: Ardia commit `551d3c13c1c9ede130724ea9f1056d4b586fbe55` (Debug build, ad hoc signed, prebuilt engine `9673f9e9b5e9`).
Phase 0 does no Ria UI work. The Ria side of every command below is for later phases.

## What is here

| Path | Use |
|---|---|
| `capture-states.mjs` | Drives Ardia to each reference state and writes `window.png`, `geometry.json`, `socket.jsonl` |
| `record-animation.mjs` | Triggers one animation, records frames with times, writes `<name>.json` |
| `measure-frames.mjs` | Turns a `framegrab` recording of any app into the same JSON (for Ria) |
| `compare.mjs` | Compares a candidate with the reference. No Ardia needed. Exit code 1 means a number is out of tolerance |
| `analyze.mjs`, `lib.mjs` | Shared measurement and launch code |
| `tools/framegrab.swift` | ScreenCaptureKit frame recorder with host-clock times |
| `patches/` | Local patch that Ardia needs to build with Xcode 26.6 (Swift 6.3) |
| `captures/states/` | The 10 reference states (in the repository) |
| `captures/animations/` | 4 animation recordings (JSON in the repository, frames named below) |

## Where the reference lives

- States and animation JSON: this directory, `captures/`. `captures/states/SHA256SUMS` lists every state file.
- Animation frames (3513 PNG files, 132 MB): the firstmate data folder `…/p0-parity-rig/animation-frames/` (the reference host keeps them; they are not in git).
  Their checksums are in `captures/animations/frames.SHA256SUMS` (paths relative to that directory).
- Verify: `(cd captures/states && sha256sum -c SHA256SUMS)`.

## Run Ardia (macOS only)

The rig needs a Mac with Xcode, Node, pnpm and CocoaPods. The working copy used here is on the Mac `mac` in `/tmp/ria-parity`.

```sh
git clone --filter=blob:none https://github.com/ardia-browser/ardia.git ardia
git -C ardia checkout 551d3c13c1c9ede130724ea9f1056d4b586fbe55
cd ardia && pnpm install --frozen-lockfile
packages/ardiacore/scripts/fetch-engine.sh --nearest   # prebuilt engine, no Chromium build
(cd apps/browser/macos && pod install)
git apply /path/to/ria/rust/parity/patches/field-glow-geometry-swift-6.3.patch   # only with Swift 6.3
# Debug build with ad hoc signing (no team needed). Metro must run: (cd apps/browser && pnpm dev --port 8399)
ARDIACORE_FRAMEWORK=$PWD/apps/browser/build-ardiacore/prebuilt/9673f9e9b5e9/Chromium\ Framework.framework \
  xcodebuild -workspace apps/browser/macos/Ardia.xcworkspace -scheme Ardia-macOS -configuration Debug \
  -derivedDataPath apps/browser/build-parity -xcconfig apps/browser/macos/AdHoc.xcconfig build
swiftc -O rust/parity/tools/framegrab.swift -o framegrab
```

The built app is `apps/browser/build-parity/Build/Products/Debug/Ardia.app`.
The rig starts it hidden and on a scratch data directory through `scripts/lib/instance.mjs`.
It never touches your real profile. Screen capture works on a locked screen.

## Capture a state

```sh
export ARDIA_DIR=<ardia checkout> METRO_PORT=8399
node capture-states.mjs <Ardia.app> <out dir> [--only tab-dark,split-dark]
```

Each state is a fixed seeded session (two profiles, pinned tabs, a group, local pages) in a 1280x800 window (the PNG is 1280x800 pixels).
States: `ntp-dark`, `ntp-light`, `tab-dark`, `tab-light`, `pinned-active-dark`, `group-collapsed-dark`, `sidebar-closed-dark`, `command-bar-dark`, `downloads-dark`, `split-dark`.
Output per state:

- `window.png`: the window alone, from `screencapture -l`.
- `geometry.json`: window bounds, sidebar regions, the tabs, panes and panels as the app reports them.
- `socket.jsonl`: the control-socket transcript (`hello`, `windows.list`, `tabs.list`, `app.state` and others), one JSON message per line with direction and time.

`states.json` lists the PNG hash of each state. The capture is deterministic: two captures give byte-identical `window.png` files.

## Record an animation

```sh
node record-animation.mjs <Ardia.app> <out dir> --grab ./framegrab --frames <frames dir> --runs 3 [--only sidebar-open]
```

Animations: `sidebar-open`, `sidebar-close`, `find-bar-open`, `downloads-open`.
Each run starts from the same baseline, sets up the start state, marks the trigger time, fires one action through the dev harness, and records until the window is still.
`framegrab` delivers a frame only when the window changes and stamps it with the host clock, so it also works for covered windows.

`<name>.json` holds, per run and summarised:

- `durationMs`: from the first changed frame to the first frame equal to the end state. Resolution is one frame (about 16.7 ms).
- `latencyMs`: trigger to first changed frame.
- `t10Ms`, `t50Ms`, `t90Ms`: time to reach 10, 50, 90 percent of the change.
- `curve`: samples `{tMs, progress}` with progress from 0 to 1. Progress is the mean pixel difference to the first frame divided by the sum of that and the difference to the last frame.
- `endState`: last frame file with SHA-256 and a 16x10 colour signature.
- `reference`: the run that `compare.mjs` uses (the one whose duration is the median).

Measured on the frozen build (3 runs each, median):

| Animation | Duration | t50 | Note |
|---|---|---|---|
| `sidebar-open` | 190.7 ms | 97.2 ms | token `duration.slow` 250 ms with arc |
| `sidebar-close` | 190.5 ms | 91.9 ms | |
| `find-bar-open` | 120.7 ms | 34.0 ms | token `FIND_APPEAR` 150 ms, curve `enter` |
| `downloads-open` | 104.5 ms | 37.0 ms | |

Spread between runs is one to two frames (for example `sidebar-open` 188 to 209 ms), so use the tolerances below, not exact equality.

## Compare a Ria capture with numbers

States: capture Ria into the same layout (`<state>/window.png` and `<state>/geometry.json`), then

```sh
node compare.mjs states captures/states <ria states dir>
```

It prints one line per state: mean absolute pixel delta (0 to 255), share of differing pixels, and the largest geometry difference in points.
A state passes when mean delta <= 2.0, differing pixels <= 2 percent, and every geometry value is within 1 pt. A missing state fails.

Animations: record Ria's window with `framegrab` (send `mark` on stdin at the trigger, then `stop`), then

```sh
./framegrab <windowID> <dir> < marks        # or drive stdin from the Ria test harness
node measure-frames.mjs <dir> sidebar-open ria-sidebar-open.json
node compare.mjs animation captures/animations/sidebar-open.json ria-sidebar-open.json
```

It prints the duration, t50, curve RMSE and end-state difference, each with its tolerance.
An animation passes when duration and t50 are within 33.4 ms (two frames), curve RMSE <= 0.05 and the end signature differs by <= 4 (0 to 255).
Change tolerances with `--pixel-tolerance`, `--geometry-tolerance`, `--duration-tolerance`, `--curve-tolerance`.
Both commands exit 1 on failure, so CI can gate on them.

Self-test of the comparer (no Ardia, no Mac): `node rust/parity/compare.selftest.mjs`. An equal pair passes; a pair above each threshold of check P fails (mean pixel delta, differing pixels, geometry, duration, t50, curve RMSE, end signature). It also checks the socket compare below.

## Socket transcript (check K)

`socket/requests.jsonl` has one request for each of the 157 methods of `rust/research/control-socket.md`, and 7 read-only requests with real results. A method with a required parameter gets `{}` (a `bad_request` reply). A changing method with no required parameter gets an unknown key (also `bad_request`). No request changes the app.

```sh
node rust/parity/socket/compare.mjs live --ardia <Ardia data dir> --ria <Ria data dir> [--write <dir>]
node rust/parity/socket/compare.mjs files <ardia transcript.jsonl> <ria transcript.jsonl>
```

Ids, times and data-folder paths are removed before the compare. Exit 1 when the transcripts differ. The `live` mode was not run against a live Ardia in slice 0.5; slice 6.2 runs it.

## Known limits

- The prebuilt engine is a fixed release; rendering of web content follows it. Pages are local files, so no network is used.
- The Xcode 26.6 build needs the patch in `patches/` (a Swift type-checker timeout in `FieldGlowGeometry.swift`). It changes no behaviour.
- Frame times are host-clock times of a delivered frame, not display refresh times. Error is about one frame.
- Only the main window is measured. Menus and native popovers outside the window are not in the PNG.
