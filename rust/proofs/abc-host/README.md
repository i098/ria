# abc-host: proofs A, B and C

A Rust host for the prebuilt Ardia engine with Rune layers. Plan v5, section 8.5, slices 0.6 to 0.8.

Run on the Mac: `ENGINE_DIR=<dir with "Chromium Framework.framework"> ./run.sh a|b|c`.
Each step prints one JSON line. A proof passes when `result` has `"pass":true` and the exit code is 0.

## Build

`blaze build --release` builds this crate (run.sh does it). It needs the patched blaze of `rust/tools/blaze/` (`install.sh`),
and `ulimit -n 10240` on the Mac (the git fetch of the Rune checkout opens many files). The older `blaze 0.20261027.0`
refused 3 groups of functions here; see `rust/tools/blaze/README.md` for the errors and the fix.

With the patched blaze (`0.0.0-dev f585269` plus `rust/tools/blaze/` patches 0001 and 0002) the crate builds and the blaze-built binary passes on the Mac:
A 3 checks, B 22 checks, C 11 checks, 0 fail each. `ulimit -n 10240` is needed or the Rune git fetch fails with `Too many open files`.
The load-case numbers in `rust/RESOURCES.md` were measured with `cargo build --release` before blaze built this crate; they have not been repeated with blaze.

## Proof B and C results (Mac arm64, macOS 26.2)

- Proof B: `./run.sh b` (blaze build) passes: 22 checks, 0 fail (tab delegate, pointer, click, wheel, focus, text keys, Backspace, Cmd+A, arrows, IME preedit and commit through `NSTextInputClient`, candidate rectangle, 100 moves + 20 wheels + 50 keys with no lost event, dark mode, live resize, frame times). Check "Hiragana input source" is NOT run: it needs the app in front (`ABC_IME_FOREGROUND=1 ./run.sh b`). The queue drain during menu tracking and during a user's live resize are NOT run (they need the window in front and a pointer drag).
- Proof C: `./run.sh c` (blaze build) passes: 11 checks, 0 fail, measured before the check `ax_chrome_text_field` was added and not re-run since (transparent layer, z order page/glass/Rune, click over the page reaches the layer only, 2 layers read one counter, accessibility children, AXButton label). VoiceOver itself and the omnibox case are NOT run (VoiceOver needs the app in front).
- Load case: `./run.sh b --load 1` and `./run.sh c --load 2`; numbers in `rust/RESOURCES.md`.

## Surface checks in the agent desktop (issue 102)

Changes from Ardia: none in product code. The fixture gains these test hooks:

- Proof C has a chrome layer (button and text field) and an overlay layer (suggestion list). New check `ax_chrome_text_field` fails on the old fixture, which had no text field.
- `ABC_IME_EXTERNAL_KEYS=1`: another process posts the Hiragana keys as real key events. Synthetic `sendEvent` keys do not reach the input method.
- `ABC_AX_HOLD_SECS=<n>`: how long proof C holds the accessibility tree for a screen reader (default 12).

Result in a Tart macOS 15.7.7 guest (arm64, no graphics): proof B Hiragana checks `ime_hiragana_marked` and `ime_hiragana` pass (marked text `日本`, field `日本`); 24 checks, 1 failure (`frame_times_600`, 608 frames at 58.9 Hz, 9 over 20.7 ms, a virtual GPU limit). VoiceOver: not tested. The guest has no audio output, and the VoiceOver first-run dialog and a screen-recording prompt covered the fixture, so no VoiceOver cursor reading was captured.
