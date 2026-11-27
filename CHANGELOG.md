# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Ria has no release yet.

## [Unreleased]

### Added

- [Native macOS launch](rust/crates/ria-app/src/launch/README.md).
- Restore the browser window's tabs after a restart: URLs, order, pins and the selected tab are saved to `session.json`.
- [Surface host](rust/crates/ria-surface/README.md): AppKit window host for Rune chrome and engine pages, with AX press and text input.
- Browser domain routing in the Rust engine, including unpacked extension inspection and shared extension management calls for the Mac host.
- Mac host ports of Ardia swipe (back swipe, native pager) and diagnostics (reveal guards, hover probe, freeze capture, paint probe).
- Phase 0 proofs: Rust host on the Ardia engine with Rune layers, port loop on `fuzzy.ts`, parity rig, unit and animation maps, decision records.
- [State replay kit](rust/replay/kit/README.md) with a find sample.
- [Shared helpers](rust/recorded/2.2/README.md) of `apps/browser/src/lib` in `ria-state::shared` (slice 2.2 part 1: leaf helpers, url, emoji data), checked against Ardia.
- Shared helpers part 2: plugin manifest helpers of `apps/browser/src/lib` in `ria-state::shared` (slice 2.2 part 2), checked against Ardia.
- Shared helpers part 3: theme and window helpers of `apps/browser/src/lib` in `ria-state::shared` (slice 2.2 part 3), checked against Ardia.
- Phase 1 core logic: Rust ports of the Phase 1 units, checked against Ardia. All checks passed for [gate G1](rust/GATES.md#g1-record).
- [Import logic](rust/crates/ria-import/src/logic/index.rs): Rust port of `packages/import/src`, checked against Ardia by the [recorded cases of slice 2.47](rust/recorded/2.47/README.md).
- Open-source community files, issue forms and PR template.
