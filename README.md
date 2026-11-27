# Ria

A native Rust and [Rune](https://github.com/anomalyco/rune) port of the [Ardia](https://github.com/ardia-browser/ardia) browser for macOS.

**State: early.** Phase 0 (proofs) and Phase 1 (core logic) passed their gates. There is no app to download yet.

## What works today

The macOS app starts the prebuilt Chromium engine and opens a native window with a tab strip.
It uses its own profile and restores the tabs of the last window after a restart: their URLs, order, pins and selected tab.
Use the address field to load a page.
Use the toolbar to go back, go forward, reload, or open a tab.
There is no settings screen yet.
Another app can send it a page URL.

The Mac proofs of Phase 0 (Rust host on the Ardia engine, Rune layers, accessibility) and the Linux proof of the port loop passed. Results: [`rust/GATES.md`](rust/GATES.md).

## Build and test

Start with [Setup](setup.md) to clone the repository and install the patched [Blaze](https://github.com/indexable-inc/blaze) build tool.
On macOS, follow the [macOS launch guide](rust/crates/ria-app/src/launch/README.md) to fetch the engine, build `Ria.app` and start it.
Use `blaze check -p ria-app` from `rust/` on macOS. The workspace-wide check includes a Linux-only target.

## Phases

Plan version 5: 7 phases, 137 slices, 213 PRs. State and gate results: [`rust/GATES.md`](rust/GATES.md).

| Phase | Work | Slices | PRs | Gate | Gate state | Phase state |
| --- | --- | ---: | ---: | --- | --- | --- |
| 0 | Decide and prove | 10 | 10 | G0: go or no-go with evidence | passed | done |
| 1 | Core logic | 8 | 8 | G1: all phase checks pass in one run | passed | done |
| 2 | State and data | 49 | 62 | G2: all phase checks pass in one run, with all store diffs | open | todo |
| 3 | Rust host on the engine | 16 | 24 | G3: all phase checks pass in one run | open | [in work](rust/MAP.md) |
| 4 | UI in Rune | 32 | 75 | G4: all phase checks pass in one run; resource numbers in the limit | open | todo |
| 5 | Native services | 17 | 29 | G5: all phase checks pass in one run | open | todo |
| 6 | Prove and cut over | 5 | 5 | G6: full parity run and resource check pass; owner go | open | todo |
| Total | | 137 | 213 | | | |

Build order:

| Lane | Starts at | Work |
| --- | --- | --- |
| Linux | G0 | Phase 1; phase 2 starts at G1; slice 2.29 comes before slice 3.12 |
| Linux | G2 | Phase 4: slice 4.1 first, then the other UI slices in parallel |
| Mac | G0 | Slices 3.1 to 3.11 and 4.31; Mac work shares one machine, one slice at a time |

Gate slices are Linux work, except G4 (slice 4.32), which is Mac work.

The goal is equality with Ardia at commit `551d3c13c` in behaviour, data formats and animation, within the tolerances of the parity rig. Out of scope: Linux, Windows and phone apps; changes to the Chromium engine; new features.

## Repository map

| Path | Content |
| --- | --- |
| `ardia/` | Ardia, a git submodule frozen at `551d3c13c` |
| `rust/` | Cargo workspace, proofs, parity rig, blaze patches |
| `rust/MAP.md` | Unit map: each Ardia unit with its target |
| `rust/DECISIONS.md` | Decision records |
| `rust/GATES.md` | Gate results and [Phase 1 replay setup](rust/GATES.md#phase-1-check-u) |
| `rust/ANIMATIONS.md` | Animation inventory |

## Contribute

Read [`CONTRIBUTING.md`](CONTRIBUTING.md). Report a vulnerability as [`SECURITY.md`](SECURITY.md) says.

## License

Apache-2.0, see [`LICENSE`](LICENSE). Ria is a port of Ardia, copyright 2026 Mantra Patel; see [`NOTICE`](NOTICE).
