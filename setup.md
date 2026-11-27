# Setup

Steps to set up Ria from a clean machine. The Linux commands ran on Linux x86_64 (node v26.12.1). The macOS steps ran on macOS 26.2, Apple silicon, with Xcode.

## Tools

| Tool | Use |
| --- | --- |
| git | clone with the `ardia` submodule |
| rustup (stable) | `cargo fmt`, `cargo clippy` |
| curl, python3 | `rust/tools/blaze/install.sh` |
| node 26 | parity rig scripts |
| sha256sum | check the reference captures |
| Xcode (macOS only) | `codesign` and the SDK for the app. A Mac without Xcode is untested. |

blaze has no Intel Mac build. Use a Mac with Apple silicon.

## Install

```sh
git clone --recurse-submodules https://github.com/i098/ria
cd ria
rust/tools/blaze/install.sh
```

`install.sh` clones blaze at one commit, applies `rust/tools/blaze/*.patch`, builds it with the LLVM that blaze pins, and installs `blaze`, `blaze-engine` and `blaze-rustc` into `${CARGO_HOME:-~/.cargo}/bin`. This replaces a `blaze` that is already there; set `CARGO_HOME` to keep it. Details: `rust/tools/blaze/README.md`.

`install.sh` needs the blaze toolchain in `~/.blaze/toolchains`. A machine that has no blaze has no toolchain. Install the older blaze first. It downloads the toolchain (`nightly-2026-10-27`, 123 MB) once:

```sh
curl -fsSL https://ix.dev/blaze | sh -s -- --version 0.20261027.0
```

Do not run this installer without `--version`. It then installs blaze 0.20261122.0, which does not download the toolchain, so `install.sh` finds none.

## macOS: open file limit

Run `ulimit -n 10240` in the shell before every `blaze` command, in the same shell that starts it. A new macOS shell can start with a limit of 256 open files, and blaze can then stop with `Too many open files (os error 24)`.

The blaze daemon keeps the limit of the shell that started it. If a `blaze` command already failed with this error, stop the daemon and run the command again:

```sh
ulimit -n 10240
blaze stop
```

## Build

On Linux:

```sh
(cd rust && blaze check)
```

On macOS, check the app crate:

```sh
(cd rust && blaze check -p ria-app)
```

On macOS, `(cd rust && blaze check)` for the whole workspace fails. It cannot compile `ardia-plugin-jail` (`jail-harness` uses `libc::ptrace`, which exists only on Linux). Use `-p` until this is fixed.

## Test and lint

```sh
node rust/parity/compare.selftest.mjs
(cd rust/parity/captures/states && sha256sum -c SHA256SUMS)
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml
```

## Run

Follow the [macOS launch guide](rust/crates/ria-app/src/launch/README.md) to start Ria. In order, it has these steps: fetch the engine, build `ria-app`, make the app bundle, start it.
For the separate Phase 0 engine proof, see the [proof host guide](rust/proofs/abc-host/README.md).
