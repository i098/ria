# blaze with the abc-host patch

`blaze` (https://github.com/indexable-inc/blaze) at commit `f5852696ea0a3c2c865ed2cdb4379e41900f279b`
(2026-10-30, the last commit of the v1 line that has `blaze build` and `blaze run`), plus the patches
`0001-blaze-abc-host.patch`, `0002-blaze-store-key-readonly-data.patch` and `0003-blaze-neon-narrowing-shifts.patch` (`install.sh` applies every `*.patch` in name order).
With them, `blaze build --release` builds the Rune host crate `rust/proofs/abc-host` on the Mac and the
result runs proofs A, B and C. The patches live only in this repository.

The newest commit of blaze (`2dd8afbda0c7`, 2026-11-22) has no `hir-ty-codegen` crate: it is the v2 line. Its canary
release `0.20261122.0` did not start on the Mac (`the pinned toolchain ... not there`) and on Linux (`io_uring_setup failed with ENOMEM`),
so the v1 commit above stays the base.

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

## Rust test harness limit

Blaze stops at the writable static `__RUST_STD_INTERNAL_VAL` before the `ria-core` bang-list parity test runs.
The fix in item 2 covers the `thread_local!` proof above, not this test harness.

## Patch 0002: the store key of LLVM's code (objc2 `declare.rs:275`)

Symptom, before (blaze `0.0.0-dev (f585269)` with patch 0001 only, Mac, proof C of `abc-host`; proof B had the same abort with 3 against 2):

    thread 'main' panicked at objc2-0.5.3/src/runtime/declare.rs:275:9:
    assertion `left == right` failed: selector setAccessibilitySelectedTextRange: accepts 1 arguments, but function accepts 2

Cause: it is not a miscompile of `F::Arguments::ENCODINGS`; each function body is compiled right. blaze keeps the code LLVM made for a
function in a store (`~/.cache/blaze/store`, or `BLAZE_CACHE`), under a key (`StoredCode::key`, `crates/hir-ty-codegen/cached.rs`) of the function's
Cranelift IR and of the constants it reads. The key had the *bytes* of the read-only data that holds no address. A constant such as
`&[Encoding]` is an address of the array and a length, so it was left out. LLVM folds `enc.len()` from it (`told_constants`). Two builds that have the
same IR and other slices then got the same key. A function stored by an earlier build (here: with a slice of 1 element) was reused by the next
one that needed 3, or 2. [INFERENCE] In the proofs the store held the code of an earlier build (proofs A and B, or other crates), which is how a function with another slice came back; the fixture shows the same with two small programs.

Fix: the key has the whole `ConstBlob` of each read-only datum that LLVM is told (`recorder.readonly()`): the bytes, the alignment, and each address
(offset, target id, addend) it holds. The key version moved from 9 to 10, so no stored entry of the old key is read. No crate is named.

Fixture (in the patch, `harness/wholeprogram`): `assoc_const_lib` (a trait with an associated const slice, implemented for tuples of 0 to 3 types; a trait of
`extern "C" fn` types of 2 to 5 parameters, with several arities; `add_method` reads the length through a generic function), and two programs that use it,
`assoc_const_slice_lib` (the table in the library) and `assoc_const_slice` (the same code in the crate of `main`). The harness builds them in this order
against one store (`BLAZE_CACHE` of the run); each prints 4 lines that end `ok`.

Before and after, same fixture, `blaze run --release -p ...` with a new `BLAZE_CACHE` and `BLAZE_HOME` for the pair (Linux x86_64; Mac arm64 after):

| Run | Before (patch 0001 only) | After (patch 0001 and 0002) |
|-----|--------------------------|-----------------------------|
| `assoc_const_slice` alone | 4 lines `ok` | 4 lines `ok` |
| `assoc_const_slice_lib`, then `assoc_const_slice` | second: `setMarked:selectedRange:replacementRange:: selector 3 function 1 MISMATCH` | both: 4 lines `ok` (cargo prints the same) |
| `assoc_const_slice`, then `assoc_const_slice_lib` | second: `selector 3 function 1 MISMATCH` | both: 4 lines `ok` |

The Mac, after, with `~/.cargo/bin/blaze` installed by `install.sh` (`0.0.0-dev (f585269)` + both patches): the pair prints 4 lines `ok` each, in the order that
failed. The branch `fm/ria-0-6-8-mac-proofs` (https://github.com/i098/ria/pull/17), `blaze build --release` in `rust/proofs/abc-host`: `Finished release profile [blaze] target(s) in 159.47s`.
`run.sh c`: `{"proof":"C","event":"result","pass":true,"checks":11,"fails":0}`, with no objc2 abort. `run.sh b`: `pass: true, checks: 22`. `run.sh a`: `pass: true, checks: 3`.
With patch 0002 the `class_addMethod` workaround of proof B is not needed.

## Patch 0003: NEON narrowing shifts (`zune_jpeg`)

Symptom, before (Mac, `blaze test -p ria-engine`, exit 101): `zune_jpeg::color_convert::neon64::ycbcr_to_rgb_baseline_no_clamp: unsupported: callee vqshrun_n_s32: unsupported: LLVM intrinsic aarch64.neon.sqshrun.v4i16`.
`zune_jpeg` comes in through `image` (`jpeg` feature) for `ria-engine` favicons, so the crate needs it.

Cause: `crates/hir-ty-codegen/neon.rs` lowered only `rshrn` among the shift-right-and-narrow intrinsics.

Fix: the same arm now lowers `sqshrn`, `uqshrn`, `sqshrun`, `sqrshrn`, `uqrshrn`, `sqrshrun` (same operand shape, immediate `1..=lane bits`); the test table has `aarch64.neon.sqshrun.v4i16`.

After, on the Mac with the rebuilt blaze: `blaze test -p ria-engine` builds and runs (`95 passed; 0 failed`).

Live run, on the Mac (measured before the live.rs check changes below): `blaze test -p ria-engine` exits 0 (`95 passed; 0 failed`). `blaze test -p ria-engine --test live` exited 101 before patch 0003 because blaze could not compile `zune_jpeg`, and it still exits 101 after patch 0003 (25 checks). Only the compile failure is fixed. The `all calls settled` check still failed because the engine did not reply to `ac_bookmarks_export_html` within the old 60 s loop limit (see the measured facts below).

Measured on the Mac (log `data/ria-fix-40-mac-check-l/diag1-export-timeout.log`): the engine call `ac_bookmarks_export_html` gets no reply for about 120 s, then replies `{"error":"Timeout"}`, both for an empty bookmark model and after `bookmarks.apply`. The path is absolute, its parent directory exists, the bookmark model is loaded (`bookmarks.tree` works), and the file never appears on disk. Ardia's own caller waits `EXPORT_ANSWER_MS = 120_000` (`packages/expo-windows/src/engine/domains/exports.ts` lines 4 and 33), then throws ERR_ENGINE "The export didn't finish". The live loop limit is 150 s. The `bookmarks.exportHtml` check passes only on reply ok with the file on disk holding the bookmark; a rejection or the 120 s Timeout fails it. So `blaze test -p ria-engine --test live` exits 101 on the Mac with exactly this one failed check, until the engine replies (issue 125). Direct probe (`data/ria-fix-125-export/findings.md`): the engine answers `an absolute path required` for a relative path, but gives no reply for a missing directory or a good path, so the call never reaches the writer's result.

The 500 ms quiet-window check (no `bookmarks.changed` after `bookmarks.apply`) passes on the Mac. A second check installs an unpacked extension whose service worker calls `chrome.bookmarks.create`; that change does not go through `bookmarks.apply`, so the watcher is not muted, and the check requires `bookmarks.changed` (issue 128).
