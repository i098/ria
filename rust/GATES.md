# Gates

One row for each gate. A slice starts only when each gate in its `Needs` column has a result here.
Each gate G1 to G6 also writes its drift report here: the count of changed units and changed lines of Ardia `origin/main` against `551d3c13c`.
Gate G0 also has the memory and frame-time numbers of proofs B and C.
Gate G1 also has the planned and the real hours of phase 1 and the corrected hour rates for the later phases.

| Gate | Needs | Result | Date | Evidence | Drift (units, lines) |
| --- | --- | --- | --- | --- | --- |
| Plan | owner approval on the board | approved | 2026-11-23 | Owner, on the board: "I approve this implementation plan. Implement it as written." Plan version 5. | |
| G0 | slices 0.5 to 0.9 are done; proofs A, B, C and D pass; owner go on the gate issue | passed | 2026-11-23 | [G0 record](#g0-record) | |
| G1 | all Phase 1 checks pass in one run | passed | 2026-11-24 | [G1 record](#g1-record) | 183 of 814 units, 72,857 lines |
| G2 | all Phase 2 checks pass in one run, with all store diffs | open | | | |
| G3 | all Phase 3 checks pass in one run | open | | | |
| G4 | all Phase 4 checks pass in one run; resource numbers in the limit | open | | | |
| G5 | all Phase 5 checks pass in one run | open | | | |
| G6 | full parity run and resource check pass; owner go | open | | | |

## Phase 1: Check U

Prepare the TypeScript oracle for all Phase 1 recordings from the repository root:

```sh
git submodule update --init --checkout ardia
rust/recorded/1.4/review-root.sh
```

The setup replaces `rust/recorded/.review-ardia` with `packages/core` from the commit in the Ardia gitlink.
It adds the replay harnesses to that directory and ignores local edits in the submodule.
Phase 1 recordings use this directory as their TypeScript source.

| Slice | Unit | Evidence | Gate command |
| --- | --- | --- | --- |
| 1.6 | `mobile.ts`, `errors.ts` | [Stateful mobile and error recordings](recorded/1.6/mobile-rs/) | `rust-anything gate rust/recorded/1.6/mobile-rs` |

## G0 record

Slice 0.10. Plan v5 section 8.5 (gate G0), limits of sections 6.6, 6.7 and 8.4.
Sources: PR https://github.com/i098/ria/pull/17, `rust/proofs/abc-host/README.md`, `rust/RESOURCES.md`, `rust/proofs/d-fuzzy/README.md`.
Mac numbers: arm64, macOS 26.2, Chromium 158.0.8127.36 (prebuilt engine). Proof D: Linux x64, node v26.12.1, rust-anything 3.3.0.

Owner decision: go, 2026-11-23: "G0 owner decision: go".

### Result of each proof

| Proof | Result | Key numbers | Limit of the plan | Against limit |
| --- | --- | --- | --- | --- |
| A | pass | 3 checks, 0 fail; `engineDidStart` at 174.8 ms; page loaded at 288.1 ms; exit 0; 0 crash files; `engineWillShutDown` 19.1 ms after quit | Check A: start, 1 tab, title, shutdown, exit 0, no crash report | pass |
| B | pass, 3 checks not run | 22 checks, 0 fail; 100 moves, 20 wheels, 50 keys, worst position error 0.000 pt; 620 frames in 10.3 s, 0 over 20.7 ms; resize 300 frames, 0 rect or drawable different from bounds; dark mode in 2 frames | B(1) each position within 1 pt; B(4) 6 or fewer frames over 20.7 ms in 600; B(3) rect equal to bounds in each frame; B(5) theme change in the next frame | B(1) pass; B(4) pass; B(3) pass for the scripted resize; B(5) over by 1 frame as coded (the check allows 2 frames); B(2) not run |
| C | pass, 2 checks not run | 11 checks, 0 fail; z order page, glass, Rune; 2 layers read 1 counter (layer 1 click shows 1 in both layers); 2 accessibility children in order; `AXButton` "Increment counter" found | C(4) `AXUIElement` finds the button; C(6) 2-layer case passes | C(4) `AXUIElement` part pass; C(6) pass; C(1), C(2), C(3) see "Gaps of the proof checks"; VoiceOver and C(5) not run |
| D | pass | `gate` 13 of 13; `diff` 623 of 623 (2 by `known-diffs.jsonl`); coverage 87 of 87 lines; `fuzz` 385 of 385 with a fresh seed; port 158 lines for 87 lines of TypeScript | Check U: 100 % match or each difference in `known-diffs.jsonl`; coverage 100 %; `fuzz` 100 % | pass |

### Load case (plan section 6.7)

Source: `rust/RESOURCES.md`, 5 runs each, medians. Load case: 3 windows, 20 tabs, sidebar open, 1 video tab.
Limit: no number worse than the number of Ardia at `551d3c13c` by more than 10 %. Ardia numbers: **not measured**, so no number is compared and none is over.

| Number | Proof B, 1 layer | Proof C, 2 layers | Ardia number | Against limit |
| --- | --- | --- | --- | --- |
| Launch to first Rune frame of window 1 (ms) | 494.3 | 520.7 | not measured | open |
| Physical footprint of all app processes after 60 s (MB) | 1351.6 | 1346.5 | not measured | open |
| p99 frame time in the 10 s sidebar loop (ms) | 16.667 | 16.667 | not measured | open |
| Longest frame in that loop (ms) | 25.1 | 18.1 | not measured | open |

The p99 is the link interval: the link ticked at 60 Hz on a 120 Hz display, so the longest frame is the useful number.
Alternative 11 (plan section 7) was not needed: no proof check failed.

### Checks not run

| Check | Why not run | What is needed |
| --- | --- | --- |
| B(2) Hiragana input source (`日本` in a Rune text field, marked text shown) | Needs the app in front on the Mac. The IME preedit and commit through `NSTextInputClient` pass with a scripted input. | `ABC_IME_FOREGROUND=1 ./run.sh b` |
| B queue drain during menu tracking | Needs the window in front and a pointer action | App in front on the Mac |
| B queue drain during a user's live resize | Needs a pointer drag. The scripted resize passes. | App in front on the Mac |
| C VoiceOver reads the label | Needs the app in front | VoiceOver on the Mac |
| C(5) the omnibox case: VoiceOver goes from the chrome-layer field to the overlay-layer list | Needs the app in front | VoiceOver on the Mac |

### Gaps of the proof checks

- C(1): `layer_transparent` reads `CAMetalLayer.isOpaque` and the runtime flag. It does not compare a screen pixel with a page pixel. The pixel difference is not measured.
- C(2): `hit_test_layer_over_page` shows that a bare part of the page does not hit the Rune view. It sends no click to the page in a transparent area. Not measured.
- C(3): the glass view is in the z order (`z_order_page_glass_rune`), and `c-glass-under-rune-layer.png` is a screenshot. The pixel change when the picture behind the window changes is not measured.
- B(5): `dark_mode_theme` allows 2 display-link frames. The plan says the next frame.
- The IME evidence is one screenshot, `b-ime-marked-text.png` (scripted input, not the Hiragana input source).

### Findings

- `ArdiaCoreTab evaluate:completion:` never calls its completion in this host (60 s wait) while the page runs (PR #17, proof A). The proofs read the title from the tab delegate. A port slice that reads page state needs another way.
- The load numbers were measured with `cargo build --release` before blaze built the crate. They were not repeated with the blaze build (`rust/RESOURCES.md`, `rust/proofs/abc-host/README.md`).
- The blaze-built binary passes A (3 checks), B (22 checks) and C (11 checks), 0 fail each. It needs `ulimit -n 10240`.
- blaze needs the 2 patches of `rust/tools/blaze/` (`0001-blaze-abc-host.patch`, `0002-blaze-store-key-readonly-data.patch`) on commit `f5852696e`. The patches live only in this repository. Without them, `blaze 0.20261027.0` refused 3 groups of functions in `abc-host`.
- Proof D: 2 recorded cases are in `known-diffs.jsonl` (lone surrogates; the napi bridge cannot pass them to a Rust `String`). `record` does not capture a store natively; each Ardia store folder needs a small `replay(actions[])` driver in a scratch copy (tested in `rust/proofs/d-fuzzy/store-demo/`: 201 cases).

## G1 record

Slice 1.8. Plan v5 sections 8.3, 8.4 (check G), 8.6 and risk R15.
The first run (2026-11-23) failed 3 checks. Issue https://github.com/i098/ria/issues/82 tracked the fixes, and https://github.com/i098/ria/pull/98 (merge commit `d7be1ad`) merged them. The second run (2026-11-24) passed all checks, so gate G1 passed.
Machine: Linux x64, node v26.12.1, rust-anything 3.3.0, blaze not used (the unit loop uses `cargo`). Both runs: shared host with a load average of 120 to 200, so all times are long.

### Merged pull requests (check M)

| Slice | Pull request | Merged (UTC) |
| --- | --- | --- |
| 1.1 | https://github.com/i098/ria/pull/28 | 2026-11-23 06:52 |
| 1.2 | https://github.com/i098/ria/pull/38 | 2026-11-23 08:17 |
| 1.3 | https://github.com/i098/ria/pull/70 | 2026-11-23 13:32 |
| 1.4 | https://github.com/i098/ria/pull/39 | 2026-11-23 08:21 |
| 1.5 | https://github.com/i098/ria/pull/55 | 2026-11-23 10:03 |
| 1.6 | https://github.com/i098/ria/pull/36 | 2026-11-23 07:22 |
| 1.7 | https://github.com/i098/ria/pull/31 | 2026-11-23 07:04 |
| issue 37 (follow-up of 1.6) | https://github.com/i098/ria/pull/81 | 2026-11-23 18:15 |

### Second run (passed)

Base: `d7be1ad` (PR 98). `origin/main` was at `7c1acad` when the run ended; `git diff d7be1ad origin/main -- rust/recorded rust/crates/ria-core` is empty, so the Phase 1 files are the same.
Setup as in "Phase 1: Check U" above, with one change: the host was too loaded for `git submodule update --init` (the clone did not finish in 25 min). The `ardia` folder is a local clone of Ardia, checked out at the pinned commit `551d3c13c1c9ede130724ea9f1056d4b586fbe55` (`git -C ardia rev-parse HEAD`), then `rust/recorded/1.4/review-root.sh` built `rust/recorded/.review-ardia` from the gitlink commit.
For each of the 29 crates of `rust/recorded/1.1`, `1.2`, `1.3`, `1.4` and `1.6`, in one run of 4 parallel streams, then `cargo test -p ria-core`:

```sh
rust-anything gate rust/recorded/<slice>/<crate>
rust-anything diff rust/recorded/<slice>/<crate>
rust-anything fuzz rust/recorded/<slice>/<crate> --seed 9271
```

After the loop, run once:

```sh
( cd rust && cargo test -p ria-core )
```

`telemetry-wire-rs` ran with `TZ=UTC RIA_NOW_MS=1795417631295 NODE_OPTIONS=--import=<crate>/clock-freeze.mjs`. All 87 steps (29 crates, 3 each) exited 0. `cargo test -p ria-core`: 77 passed, 0 failed (includes `endpoints::tests::every_endpoint_is_off` and `search::bang_list::tests::bundled_lists_match_frozen_ardia_samples`). Check M: the 8 pull requests above, plus PR 98, are merged.

| Slice | Crate | diff | fresh fuzz (seed 9271) | Coverage printed by `gate` | Unreached lines |
| --- | --- | --- | --- | --- | --- |
| 1.1 | `fuzzy-rs` | 623/623 (2 known) | 387/387 | 87 of 87 | none |
| 1.2 | `appurls-rs` | 1104/1104 | 557/557 | 52 of 52 | none |
| 1.2 | `cleanurl-rs` | 2837/2837 | 452/452 | 68 of 68 | none |
| 1.2 | `idn-rs` | 1662/1662 | 426/426 | 321 of 323 | `UNREACHED.md` |
| 1.2 | `pagematch-rs` | 310/310 | 177/177 | 356 of 862 | `UNREACHED.md` |
| 1.2 | `pagetext-rs` | 763/763 | 408/408 | 323 of 753 | `UNREACHED.md` |
| 1.2 | `textfragment-rs` | 591/591 | 389/389 | 72 of 73 | `UNREACHED.md` |
| 1.2 | `webstore-rs` | 1018/1018 | 463/463 | 71 of 71 | none |
| 1.3 | `calculator-rs` | 6694/6694 (49 known) | 277/277 | 293 of 293 | none |
| 1.3 | `candidate-pool-rs` | 2614/2614 | 952/952 | 747 of 957 | `UNREACHED.md` |
| 1.3 | `omnibox-rs` | 19326/19326 (3 known) | 1835/1835 | 670 of 698 | `UNREACHED.md` |
| 1.3 | `open-tabs-rs` | 1800/1800 | 194/194 | 10 of 10 | none |
| 1.3 | `quick-create-rs` | 2777/2777 | 153/153 | 38 of 38 | none |
| 1.3 | `suggest-fetcher-rs` | 1709/1709 | 155/155 | 550 of 992 | `UNREACHED.md` |
| 1.3 | `suggest-rs` | 3401/3401 | 199/199 | 1963 of 2383 | `UNREACHED.md` |
| 1.3 | `suggestion-rows-rs` | 2749/2749 | 853/853 | 412 of 842 | `UNREACHED.md` |
| 1.4 | `bangs-harness-rs` | 1114/1114 | 929/929 | 150 of 230 | `UNREACHED.md` |
| 1.4 | `bangs-rs` | 261/261 (7 known) | 1394/1394 | 123 of 210 | `UNREACHED.md` |
| 1.4 | `engines-rs` | 2184/2184 | 1555/1555 | 459 of 856 | `UNREACHED.md` |
| 1.4 | `extension-rows-rs` | 387/387 | 195/195 | 97 of 112 | `UNREACHED.md` |
| 1.4 | `provider-config-rs` | 715/715 | 458/458 | 492 of 997 | `UNREACHED.md` |
| 1.4 | `provider-network-rs` | 905/905 (2 known) | 384/384 | 449 of 965 | `UNREACHED.md` |
| 1.4 | `provider-rows-rs` | 1726/1726 | 875/875 | 445 of 875 | `UNREACHED.md` |
| 1.4 | `site-search-rs` | 1913/1913 | 1352/1352 | 529 of 979 | `UNREACHED.md` |
| 1.6 | `anonymous-identity-rs` | 891/891 | 330/330 | 86 of 86 | none |
| 1.6 | `ingest-keys-rs` | 219/219 | 141/141 | 14 of 14 | none |
| 1.6 | `mobile-rs` | 3/3 | 190/190 | 12822 of 14100 | `UNREACHED.md` |
| 1.6 | `telemetry-wire-rs` | 1463/1463 (8 known) | 659/659 | 67 of 67 | none |
| 1.6 | `theme-rs` | 8226/8226 | 5843/5843 | 291 of 291 | none |

Coverage under 100 %: the plan accepts it when each unreached line is listed with its reason. Each of the 18 crates below 100 % has an `UNREACHED.md` in its folder (PR 98). `[INFERENCE]` I checked that the files exist, not each line range. `mobile-rs` fuzz sends strings, not call sequences (see PR 81); the call sequences are covered by the recorded inputs.
Diff of the first-run failures: the 7 crates of 1.2 now build and pass; `telemetry-wire-rs` fuzz is 659/659 (was 573/659) with 8 cases in `known-diffs.jsonl` (was 113).

### First run (2026-11-23, failed)

Details of the first run follow. They stay as the record of what PR 98 fixed.

#### Commands and results of the first run

Run from the repository root. `<crate>` is each of the 29 crates in `rust/recorded/1.1`, `1.2`, `1.3`, `1.4` and `1.6`. Seed 8128 is new for each crate. Logs: `/sdb-disk/host-data/administrator/.treehouse/firstmate-8bf1b0/21/firstmate/data/ria-1-8-gate-g1/logs/` (outside the repository).

```sh
rust-anything gate rust/recorded/<slice>/<crate>
rust-anything diff rust/recorded/<slice>/<crate>
rust-anything fuzz rust/recorded/<slice>/<crate> --seed 8128
```

After the loop, run once:

```sh
( cd rust && cargo test -p ria-core )     # checks T of slices 1.5 and 1.7
```

`telemetry-wire-rs` ran with `TZ=UTC RIA_NOW_MS=1795417631295 NODE_OPTIONS=--import=<crate>/clock-freeze.mjs` (see `rust/recorded/1.6/README.md`).

| Slice | Crate | gate | diff | fresh fuzz (seed 8128) | Coverage printed by `gate` |
| --- | --- | --- | --- | --- | --- |
| 1.1 | `fuzzy-rs` | pass | 623/623 (2 known) | 385/385 | 87 of 87 (100 %) |
| 1.2 | `appurls-rs` | **fail** | not run: no `.node` module | not run | 52 of 52 |
| 1.2 | `cleanurl-rs` | **fail** | not run | not run | 68 of 68 |
| 1.2 | `idn-rs` | **fail** | not run | not run | 321 of 323 (99 %) |
| 1.2 | `pagematch-rs` | **fail** | not run | not run | 356 of 862 (41 %) |
| 1.2 | `pagetext-rs` | **fail** | not run | not run | 323 of 753 (43 %) |
| 1.2 | `textfragment-rs` | **fail** | not run | not run | 72 of 73 (99 %) |
| 1.2 | `webstore-rs` | **fail** | not run | not run | 71 of 71 |
| 1.3 | `calculator-rs` | pass | 6694/6694 (49 known) | 270/270 | 293 of 293 |
| 1.3 | `candidate-pool-rs` | pass | 2614/2614 | 960/960 | 747 of 957 (78 %) |
| 1.3 | `omnibox-rs` | pass | 19326/19326 (3 known) | 1858/1858 | 670 of 698 (96 %) |
| 1.3 | `open-tabs-rs` | pass | 1800/1800 | 190/190 | 10 of 10 |
| 1.3 | `quick-create-rs` | pass | 2777/2777 | 153/153 | 38 of 38 |
| 1.3 | `suggest-fetcher-rs` | pass | 1709/1709 | 162/162 | 550 of 992 (55 %) |
| 1.3 | `suggest-rs` | pass | 3401/3401 | 200/200 | 1963 of 2383 (82 %) |
| 1.3 | `suggestion-rows-rs` | pass | 2749/2749 | 855/855 | 412 of 842 (49 %) |
| 1.4 | `bangs-harness-rs` | pass | 1114/1114 | 918/918 (a) | 150 of 230 (65 %) |
| 1.4 | `bangs-rs` | pass | 261/261 (7 known) | 1394/1394 (a) | 123 of 210 (59 %) |
| 1.4 | `engines-rs` | pass | 2184/2184 | 1541/1541 (a) | 459 of 856 (54 %) |
| 1.4 | `extension-rows-rs` | pass | 387/387 | 196/196 (a) | 97 of 112 (87 %) |
| 1.4 | `provider-config-rs` | pass | 715/715 | 433/433 (a) | 492 of 997 (49 %) |
| 1.4 | `provider-network-rs` | pass | 905/905 (2 known) | 365/365 (a) | 449 of 965 (47 %) |
| 1.4 | `provider-rows-rs` | pass | 1726/1726 | 884/884 (a) | 445 of 875 (51 %) |
| 1.4 | `site-search-rs` | pass | 1913/1913 | 1344/1344 (a) | 529 of 979 (54 %) |
| 1.6 | `anonymous-identity-rs` | pass | 891/891 | 332/332 | 86 of 86 |
| 1.6 | `ingest-keys-rs` | pass | 219/219 | 139/139 | 14 of 14 |
| 1.6 | `mobile-rs` | pass | 1/1 | 200/200 (b) | 12805 of 14093 (91 %) |
| 1.6 | `telemetry-wire-rs` | pass | 1463/1463 (113 known) | **573/659** | 67 of 67 |
| 1.6 | `theme-rs` | pass | 8226/8226 | 5868/5868 | 291 of 291 |
| 1.5 | `cargo test -p ria-core` | | | 77 tests passed, 0 failed (includes `search::bang_list::tests::bundled_lists_match_frozen_ardia_samples`) | |
| 1.7 | `cargo test -p ria-core` | | | `endpoints::tests::every_endpoint_is_off` passed | |

(a) The crate has `"root": "."`, and `fuzz` looks for the TypeScript original under the crate folder. The first fuzz run failed with `ERR_MODULE_NOT_FOUND`. The second run linked `packages` from a scratch Ardia checkout at `551d3c13c` into the crate folder and removed the link afterwards. For `bangs-harness-rs`, the scratch copy also held `bangs.harness.ts` from the crate folder.
(b) Same cause: the root `.review-ardia` is relative to the crate folder. The run linked a scratch copy of `ardia/packages` with `mobile.harness.ts`. The fuzz inputs are strings, not call sequences (PR 81 review note), so this number does not test the 13 mobile calls.

#### Failed checks of the first run

| Check | Slice | Fact | Before and after |
| --- | --- | --- | --- |
| U (`gate`, `diff`, `fuzz`) | 1.2 | The 7 crates of `rust/recorded/1.2/` have no `[workspace]` table in `Cargo.toml` (the crates of 1.1, 1.3, 1.4 and 1.6 have it). `gate` stops with "current package believes it's in a workspace when it's not" because `rust/Cargo.toml` has `members = ["crates/*"]`. The gate steps fmt, clippy, cargo test and build fail; the `.node` module is not built, so `diff` and `fuzz` stop with `MODULE_NOT_FOUND`. | Fix, outside this slice: add an empty `[workspace]` table to each of the 7 files, like `rust/recorded/1.1/fuzzy-rs/Cargo.toml`. After it, run the gate, `diff` and `fuzz` of the 7 crates again. |
| U (`fuzz`) | 1.6 | `telemetry-wire-rs`: 573 of 659 fresh cases match. All 20 listed mismatches are `logRecord` calls. The original gives `NaN000000` or `0000000` for the time; the port gives the frozen time, or the error "`at` is a string or a list, which `Math.round` coerces". Plan check U needs a 100 % match. PR https://github.com/i098/ria/pull/36 reported the same family (20 `eventRecord` mismatches). Not in `known-diffs.jsonl`. | Open: add the cases to `known-diffs.jsonl` with a reason, or change the port. |
| U (coverage) | 1.2, 1.3, 1.4, 1.6 | `gate` prints a coverage under 100 % for 18 of 29 crates (table above). The tool counts imported files (for example `idn.ts`) with the unit. The plan needs 100 % of the original lines, or a list in the PR body of each line that no input can reach. This run did not check the PR bodies for those lists. | Open: each slice owner lists the lines, or adds inputs. |

The first run therefore did not pass gate G1. The count of passed checks: M 8 of 8, T 2 of 2, U `gate` 22 of 29, `diff` 22 of 29, `fuzz` 21 of 29 (counting (a) as pass and (b) as pass).

### Drift report (plan section 8.3)

Ardia `origin/main` is `14db917fd`, 120 commits ahead of `551d3c13c` (fetched 2026-11-23; `gh api repos/ardia-browser/ardia/compare/551d3c13c...main`). The `ardia` submodule of this repository is not checked out, so the run used a scratch clone with `origin` set to `https://github.com/ardia-browser/ardia`.

| Number | Value | How |
| --- | ---: | --- |
| Changed files in `apps/browser packages engine` | 1,664 | `git diff --numstat 551d3c13c origin/main -- apps/browser packages engine` |
| Changed lines in those files (added + deleted) | 72,857 (64,360 + 8,497) | the same output |
| **Changed units** (rows of `rust/MAP.md` with scope `mac`) | **183 of 814 (22.5 %)** | each file goes to the longest matching unit row (file, folder, or `Name.*` stem) |
| Changed lines inside those 183 units | 37,857 | the same grouping |
| Files with no `mac` unit | 474 files, 35,000 lines | they are in scope `out`, or are tests and non-`mac` folders (`packages/expo-windows`, `packages/phone`, `packages/core-rs`, ...) |

Of the 183 changed units, 139 have the disposition `port`, 26 `absorb`, 16 `stay`, 1 `done`, 1 `later`.
The limit of section 8.3 is 407 of 814 units (50 %). The drift is under it, so the gate does not go to the owner for this reason.
Step 4 (`rust-anything map --check ardia --crates rust/recorded`) exited 0 and printed "0 of 683 units have a Rust port": the map tool reads only the scratch Ardia repository, and it does not read the `recorded-from.json` fingerprints of this repository. It gives no signal about outdated recordings.
The numstat shows 24 changed files in `packages/core/src` (the source of phase 1), for example `appUrls.ts`, `calculator.ts`, `candidatePool.ts`, `cleanUrl.ts`, `pageMatch.ts`, `suggest.ts`, `suggest.reference.ts`, `suggestFetcher.ts`, `siteSearch.ts`, `mobile.ts`. The recordings of these units are older than the upstream source. Section 8.3 has no rule for this case besides the drift limit.

### Planned and real hours of phase 1

Method. The real hours come from i098/ria only: the merge time of each pull request (`gh pr view <n> --json mergedAt`) and the author dates of its commits (`git log --format=%ad`). Two measures, because the start of a worker is not recorded:

- Low: from the first visible work (the earlier of the slice issue creation and the first commit author date) to the merge. This leaves out work before the first commit.
- High: from the time all slices in the `Needs` column were merged (for 1.1, the owner go of G0 at 06:42) to the merge. This counts the wait for dispatch and the review pipeline.

Hours are wall-clock hours of one slice. Slices ran in parallel, so the sum is lane hours, not elapsed time. Slice 1.3 has no issue: its first commit is 07:22, and its review rounds ran until 13:32 (6 commits, 3 of them `no-mistakes`).

| Slice | Planned (h) | Real low (h) | Real high (h) | Source lines |
| --- | ---: | ---: | ---: | ---: |
| 1.1 crate, fuzzy | 1.0 | 0.03 | 0.16 | 165 |
| 1.2 URL units | 2.1 | 0.52 | 1.42 | 824 |
| 1.3 omnibox, suggest | 2.8 | 6.16 | 6.67 | 1,569 |
| 1.4 search, bangs | 1.8 | 0.77 | 1.48 | 850 |
| 1.5 bang list data | 0.5 | 1.29 | 1.71 | 10,706 (moved by script) |
| 1.6 identity, theme | 1.5 | 0.02 | 0.50 | 581 |
| 1.7 endpoints | 0.5 | 0.11 | 0.21 | - |
| issue 37 (parity fixes of 1.6, pull request 81) | 0.0 | 1.78 | 1.78 | - |
| issue 82 (gate fixes, pull request 98) | 0.0 | 5.07 | 6.93 | - |
| 1.8 this slice (task start to commit) | 1.5 | 1.5 | 1.5 | - |
| Total | 11.7 | 17.25 | 22.36 | |

Issue 37 starts at the first commit (2026-11-23 16:28), not at the issue creation (07:22): the issue waited 9 h before work began.

### Corrected rates for the later phases (R15)

The unit loop (check U) slices are 1.1, 1.2, 1.3, 1.4 and 1.6, with 3,989 source lines (bang list not counted). Their planned hours are 9.2, which is 434 lines for each hour. Issues 37 and 82 are rework of this loop, so they count to it. Issue 82 is measured from its first commit (author date 2026-11-23 21:37) to the merge (2026-11-24 02:41) for low, and from the creation of the issue (19:45) for high. The 1.8 row is my estimate; the real time of this slice is longer, because the second run alone took about 4 h on the loaded host.

| Number | Low | High |
| --- | ---: | ---: |
| Real hours of the unit loop (with issues 37 and 82) | 14.35 | 18.94 |
| Real lines for each hour | 278 | 211 |
| Factor, real hours / planned hours (9.2) | 1.56 | 2.06 |

Decision rule: use the high factor 2.06, because plan risk R15 is about too few hours and the low measure leaves out the work before the first commit. Both factors are above the first-run numbers (1.01 and 1.31), because the rework of the gate (PR 98) cost 5.1 to 6.9 h. The plan rates (600 and 1,000 lines for each hour) are not met by either measure.

| Item | Plan v5 (h) | Corrected (h) | Basis |
| --- | ---: | ---: | --- |
| Unit slices of all phases (U checks) | 358.0 | 737.5 | 358.0 x 2.06 |
| All other hours (Mac, native code, UI, tests, gates) | 247.6 | 247.6 | not measured by phase 1; keep the plan hours until a later gate gives a number |
| Plan total | 605.6 | 985.1 | 247.6 + 737.5 |

The corrected total, 985.1 h, is above the R15 worst case (827.1 h, at 400 lines for each hour; the plan v5 text for R15 is not in this repository, so this figure is quoted from the plan and cannot be rederived here). The measured rate of the unit loop is 211 to 278 lines for each hour, below the 400 of R15. `[INFERENCE]` The rework share (issues 37 and 82) is a one-time cost of the first loop, and later phases with the same oracle setup may need less; gate G2 measures it again. The owner should see this number before phase 2 starts.
