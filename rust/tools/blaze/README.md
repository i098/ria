# blaze with the abc-host patch

`blaze` (https://github.com/indexable-inc/blaze) at commit `f5852696ea0a3c2c865ed2cdb4379e41900f279b`
(2026-10-30, the last commit of the v1 line that has `blaze build` and `blaze run`), plus
`0001-blaze-abc-host.patch`. With it, `blaze build --release` builds the Rune host crate
`rust/proofs/abc-host` on the Mac. The patch lives only in this repository.

Install (Mac arm64 or Linux x86_64; clones the commit, applies the patch, builds with blaze's LLVM, installs
`blaze`, `blaze-engine` and `blaze-rustc` into `~/.cargo/bin`):

    rust/tools/blaze/install.sh [WORK_DIR]    # default /tmp/ria-blaze, 10 to 15 minutes cold

## Before and after

Before, `blaze 0.20261027.0 (49ca152)` on the Mac refused three groups of functions in `abc-host`:

| # | Refusal | Cause |
|---|---------|-------|
| 1 | `unsupported: writable static MIN without a rustc symbol` (`std::thread::spawn`) | fixed upstream before `f5852696e` (the cause was not analysed here) |
| 2 | `unsupported: writable static __RUST_STD_INTERNAL_VAL without a rustc symbol` (`thread_local!` `.with`) | same |
| 3 | `unsupported: Alias(AliasTy { args: [CGRect, ()], kind: Projection ...}) in the C ABI` (objc2 `define_class!`, `accesskit_macos::PlatformNode`) | the C ABI code read a signature or field type as written, so `<CGRect as ConvertArgument>::__Inner` was not the struct it names |

After:

1. and 2. are fixed upstream: the base `f5852696e` builds `std::thread::spawn` and `thread_local!` programs
   (`harness/wholeprogram/std_threads`: output `41 2 2 1`, the same as cargo's). No patch is needed for them.
3. is fixed in `crates/hir-ty-codegen/mod.rs`: `fn_abi` and the three field walks of the C ABI classification
   (`scalar_leaves`, `classify`, transparent-wrapper `decays_in_c`) normalise a type before they read its layout.
   No crate name is special-cased. The fixture `harness/wholeprogram/c_abi_projection` (arguments and result
   as projections, through a generic caller) prints `13 2 1`, the same as cargo's.
4. `crates/hir/src/named.rs` uses `&text()` where the base used the unstable `str::as_str`, so the base builds
   with blaze's nightly-2026-10-27 toolchain (it did not: `E0658`).

The two fixtures are in `harness/wholeprogram` (listed in its `CASES`), the place where blaze keeps its programs that
are run and compared with cargo's.

After, on the Mac (arm64, macOS 26.2), blaze `0.0.0-dev (f585269)` installed by `install.sh`:

- `blaze build --release` in `rust/proofs/abc-host`: `Finished release profile [blaze] target(s) in 130.54s`
  (`Compiled 180476 fns in 260 crates`, no refusal).
- Proof A run of that binary (`run.sh` with the `cargo build` line replaced by this binary): `{"event":"result","pass":true,"checks":3,"fails":0}`, exit 0.
- `blaze check` in `rust/`: `Finished dev profile [blaze] target(s) in 2.64s`.

The previous Mac binary is kept as `~/.cargo/bin/blaze.v1-0.20261027.bak`.
