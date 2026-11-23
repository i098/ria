# Ria

A native Rust and [Rune](https://github.com/anomalyco/rune) port of the [Ardia](https://github.com/ardia-browser/ardia) browser for macOS.

**State: early.** Phase 0 (proofs) passed. Phase 1 (core logic) is in work. There is no app to download yet.

## What works today

```sh
(cd rust && blaze check)                                     # workspace compiles
cargo fmt --manifest-path rust/Cargo.toml --check            # format
cargo clippy --manifest-path rust/Cargo.toml                 # lint: no issues
node rust/parity/compare.selftest.mjs                        # parity comparer: 15 checks passed
(cd rust/parity/captures/states && sha256sum -c SHA256SUMS)  # reference captures: all OK
```

Run these from the repository root.

The Mac proofs of Phase 0 (Rust host on the Ardia engine, Rune layers, accessibility) and the Linux proof of the port loop passed. Results: [`rust/GATES.md`](rust/GATES.md).

## Build and test

```sh
git clone --recurse-submodules https://github.com/i098/ria
cd ria
rust/tools/blaze/install.sh     # installs the patched blaze into ~/.cargo/bin; 10 to 15 minutes cold
(cd rust && blaze check)
```

Ria builds with [blaze](https://github.com/indexable-inc/blaze) at one pinned commit plus two patches kept in [`rust/tools/blaze/`](rust/tools/blaze/README.md). Without blaze, run `cargo fmt` and `cargo clippy` (first block above); they check the workspace. Blaze is the only supported build tool, so say in your PR that you could not run `blaze check`. Full steps: [`setup.md`](setup.md).

## Phases

| Phase | Work | State |
| --- | --- | --- |
| 0 | Decide and prove | done |
| 1 | Core logic | in work |
| 2 | State and data | todo |
| 3 | Rust host on the engine | in work |
| 4 | UI in Rune | todo |
| 5 | Native services | todo |
| 6 | Prove and cut over | todo |

The goal is equality with Ardia at commit `551d3c13c` in behaviour, data formats and animation, within the tolerances of the parity rig. Out of scope: Linux, Windows and phone apps; changes to the Chromium engine; new features.

## Repository map

| Path | Content |
| --- | --- |
| `ardia/` | Ardia, a git submodule frozen at `551d3c13c` |
| `rust/` | Cargo workspace, proofs, parity rig, blaze patches |
| `rust/MAP.md` | Unit map: each Ardia unit with its target |
| `rust/DECISIONS.md` | Decision records |
| `rust/GATES.md` | Gate results |
| `rust/ANIMATIONS.md` | Animation inventory |

## Contribute

Read [`CONTRIBUTING.md`](CONTRIBUTING.md). Report a vulnerability as [`SECURITY.md`](SECURITY.md) says.

## License

Apache-2.0, see [`LICENSE`](LICENSE). Ria is a port of Ardia, copyright 2026 Mantra Patel; see [`NOTICE`](NOTICE).
