# abc-host: proofs A, B and C

A Rust host for the prebuilt Ardia engine with Rune layers. Plan v5, section 8.5, slices 0.6 to 0.8.

Run on the Mac: `ENGINE_DIR=<dir with "Chromium Framework.framework"> ./run.sh a|b|c`.
Each step prints one JSON line. A proof passes when `result` has `"pass":true` and the exit code is 0.

## Build

`blaze build --release` builds this crate (run.sh does it). It needs the patched blaze of `rust/tools/blaze/` (`install.sh`),
and `ulimit -n 10240` on the Mac (the git fetch of the Rune checkout opens many files). The older `blaze 0.20261027.0`
refused 3 groups of functions here; see `rust/tools/blaze/README.md` for the errors and the fix.

With the patched blaze (`0.0.0-dev f585269`) the crate builds; results of the blaze-built binary on the Mac:

- A: pass, 3 checks.
- B: pass, 22 checks, after one change in this crate: `setMarkedText:selectedRange:replacementRange:` is added with `class_addMethod` after `define_class!`, because under blaze `define_class!` gives that 3-argument selector an argument list of 2 and objc2 panics at class registration (`selector ... accepts 3 arguments, but function accepts 2`).
- C: FAILS to start. `accesskit_macos` (objc2 0.5.3) panics when its class is declared: `selector setAccessibilitySelectedTextRange: accepts 1 arguments, but function accepts 2`. Same family as the B error (the argument list `F::Arguments::ENCODINGS` has the wrong length under blaze). The code is in a dependency, so this crate cannot work around it.
- `ulimit -n 10240` is needed or the Rune git fetch fails with `Too many open files`.
- The load-case numbers in `rust/RESOURCES.md` were measured with `cargo build --release` before the blaze patch; they have not been repeated with blaze.

## Proof B and C results (Mac arm64, macOS 26.2)

- Proof B: `./run.sh b` (blaze build) passes: 22 checks, 0 fail (tab delegate, pointer, click, wheel, focus, text keys, Backspace, Cmd+A, arrows, IME preedit and commit through `NSTextInputClient`, candidate rectangle, 100 moves + 20 wheels + 50 keys with no lost event, dark mode, live resize, frame times). Check "Hiragana input source" is NOT run: it needs the app in front (`ABC_IME_FOREGROUND=1 ./run.sh b`). The queue drain during menu tracking and during a user's live resize are NOT run (they need the window in front and a pointer drag).
- Proof C: `./run.sh c` (blaze build) passes: 11 checks, 0 fail (transparent layer, z order page/glass/Rune, click over the page reaches the layer only, 2 layers read one counter, accessibility children, AXButton label). VoiceOver itself and the omnibox case are NOT run (VoiceOver needs the app in front).
- Load case: `./run.sh b --load 1` and `./run.sh c --load 2`; numbers in `rust/RESOURCES.md`.
