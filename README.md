# Ria

A native Rust + Rune browser for Mac.
Ria is a port of [Ardia](https://github.com/ardia-browser/ardia) (Copyright 2026 Mantra Patel, Apache-2.0).
The goal is equality with Ardia at commit `551d3c13c` in behaviour, data formats and every animation, within the tolerances of check P (plan section 8.4). The Chromium engine of Ardia stays as it is.

## Scope

In: the Mac app of Ardia, ported to Rust with the Rune UI framework (`anomalyco/rune`); the Ardia engine used as it is; all animations equal to the original; the plan holds Ardia at `551d3c13c` until G6.
Out: Linux, Windows and phone apps; changes to the Chromium engine; new features; contributions to `ardia-browser/ardia`; a public release; a team-signed or notarized build; an updater; a sync with a newer Ardia commit; a real app icon.

## Plan

The approved plan is version 5: 7 phases, 137 slices, 213 PRs.

| Phase | Slices | PRs | Gate |
| --- | ---: | ---: | --- |
| 0 Decide and prove | 10 | 10 | G0 |
| 1 Core logic | 8 | 8 | G1 |
| 2 State and data | 49 | 62 | G2 |
| 3 Rust host on the engine | 16 | 24 | G3 |
| 4 UI in Rune | 32 | 75 | G4 |
| 5 Native services | 17 | 29 | G5 |
| 6 Prove and cut over | 5 | 5 | G6 |
| Total | 137 | 213 | |

G0 is a go or no-go on the port with evidence. G1 to G5 pass when all checks of the phase pass in one run. G6 is the owner's go on use of the app.

## Build order

Two lanes run in parallel: Linux and Mac. Both lanes start at G0. Phase 1 starts at G0, and phase 2 starts at G1.
On the Mac lane, slices 3.1 to 3.11 and 4.31 start at G0. On the Linux lane, the Linux parts of phase 4 start at G2: slice 4.1 first, then the other UI slices in parallel.
Slice 2.29 comes before slice 3.12. G2 comes before slice 4.1.
Gate slices are Linux work, except G4 (slice 4.32), which is Mac work. Mac work shares one machine, one slice at a time.

## Phase 0 state

| Slice | Work | State |
| --- | --- | --- |
| 0.1–0.4 | base, data notes, engine notes, proof D | merged (PRs 3, 6, 7, 8) |
| 0.5 | parity rig | in work |
| 0.6–0.8 | proofs A to C | in work |
| 0.9 | apply plan v5 review | merged ([PR 11](https://github.com/i098/ria/pull/11)) |
| 0.10 | gate G0 | todo |

## Where the detail is

- `ardia/`: the original, a git submodule frozen at `551d3c13c`.
- `rust/MAP.md`: the unit map: 1,255 units, 814 in the Mac scope and 441 out, with a slice and a disposition for each unit.
- `rust/DECISIONS.md`: the decision records (ADRs).
- `rust/GLOSSARY.md`: the words the plan and the repository use.
- `rust/GATES.md`: the plan approval and the result of each gate.
- `rust/ANIMATIONS.md`: the 356 animation rows: 278 in React components (part A), 26 in state, lib and platform code (part B), 52 native (part C).
- `LICENSE` and `NOTICE`: Apache-2.0, with credit to Ardia.
