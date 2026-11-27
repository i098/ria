# Setup

Use this guide to build Ria from source.
The macOS steps ran on macOS 26.2 with Apple silicon and Xcode.
The Linux commands ran on Linux x86_64 with Node.js 26.12.1.

## Tools

| Tool | Use |
| --- | --- |
| git | clone with the `ardia` submodule |
| rustup (stable) | `cargo fmt`, `cargo clippy` |
| curl, python3 | `rust/tools/blaze/install.sh` |
| node 26 | parity rig scripts |
| sha256sum | check the reference captures |
| Xcode (macOS only) | macOS SDK and `codesign` |

Use a Mac with Apple silicon. Blaze does not support Intel Macs.

## Install

Start with a clean build shell. If you already use Blaze, isolate its files in a temporary home for this setup:

```sh
export HOME="$(mktemp -d)"
unset CARGO_HOME BLAZE_HOME RUSTUP_HOME
```

Run the remaining build and launch commands in that shell. Close the shell when you finish.

```sh
git clone --recurse-submodules https://github.com/i098/ria
cd ria
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
```

Install the pinned Blaze v1 release to get its Rust nightly:

```sh
curl -fsSL https://ix.dev/blaze | sh -s -- --version 0.20261027.0
```

Keep `--version 0.20261027.0`. The newer Blaze v2 installer does not download the toolchain that Ria's install script needs.

```sh
rust/tools/blaze/install.sh "${TMPDIR:-/tmp}/ria-blaze"
```

Both installers write to `${CARGO_HOME:-$HOME/.cargo}/bin`.
The bootstrap also changes Blaze state in `~/.blaze`; setting `CARGO_HOME` alone does not isolate it.
The Ria script selects the last nightly by directory name in `~/.blaze/toolchains`. Use the clean shell above if you have other Blaze toolchains.
The Ria script builds Blaze at a pinned commit, applies the [Ria patches](rust/tools/blaze/README.md), and installs the patched binaries.

## macOS: open file limit

Run `ulimit -n 10240` in each shell before the first build or check.
A macOS shell can start with a limit of 256 open files. Blaze can then stop with `Too many open files (os error 24)`.

The Blaze daemon keeps the limit of the shell that started it.
If Blaze reports this error in Ria's workspace, run these commands from the repository root, then try the failed command again:

```sh
ulimit -n 10240
(cd rust && blaze stop)
```

## Check the build

On Linux:

```sh
(cd rust && blaze check)
```

On macOS, check the app crate:

```sh
(cd rust && blaze check -p ria-app)
```

The workspace-wide `blaze check` fails on macOS because `ardia-plugin-jail` includes the Linux-only `jail-harness`.
Use `blaze check -p ria-app` on macOS.

## Test and lint

Install Rust stable with `rustfmt` and Clippy for these development checks.
Run them from the checkout in your normal shell.
Run the format check on either platform:

```sh
cargo fmt --manifest-path rust/Cargo.toml --check
```

On macOS, lint the app and host crates:

```sh
cargo clippy --locked --manifest-path rust/Cargo.toml -p ria-host-mac -p ria-app --all-targets -- -D warnings
```

On Linux, lint the workspace and check the parity captures:

```sh
cargo clippy --manifest-path rust/Cargo.toml
node rust/parity/compare.selftest.mjs
(cd rust/parity/captures/states && sha256sum -c SHA256SUMS)
```

## Run

Follow the [macOS launch guide](rust/crates/ria-app/src/launch/README.md) to fetch the engine, build the bundle and start Ria.
For the separate Phase 0 engine proof, see the [proof host guide](rust/proofs/abc-host/README.md).
