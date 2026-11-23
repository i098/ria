# Setup

Steps to set up Ria from a clean machine. Commands here ran on Linux x86_64 (node v26.12.1). The Mac proofs need macOS with Xcode command line tools.

## Tools

| Tool | Use |
| --- | --- |
| git | clone with the `ardia` submodule |
| rustup (stable) | `cargo fmt`, `cargo clippy` |
| curl, python3 | `rust/tools/blaze/install.sh` |
| node 26 | parity rig scripts |
| sha256sum | check the reference captures |

## Install

```sh
git clone --recurse-submodules https://github.com/i098/ria
cd ria
rust/tools/blaze/install.sh
```

`install.sh` clones blaze at one commit, applies `rust/tools/blaze/*.patch`, builds it with the LLVM that blaze pins, and installs `blaze`, `blaze-engine` and `blaze-rustc` into `${CARGO_HOME:-~/.cargo}/bin`. It needs a blaze toolchain in `~/.blaze/toolchains`: run any `blaze` command once to fetch it. Details: `rust/tools/blaze/README.md`.

## Build

```sh
(cd rust && blaze check)
```

## Test and lint

```sh
node rust/parity/compare.selftest.mjs
(cd rust/parity/captures/states && sha256sum -c SHA256SUMS)
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml
```

## Run

There is no app yet. The Phase 0 host `rust/proofs/abc-host` runs on a Mac with the prebuilt Ardia engine; see `rust/proofs/abc-host/README.md`.
