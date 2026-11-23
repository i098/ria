# abc-host: proofs A, B and C

A Rust host for the prebuilt Ardia engine with Rune layers. Plan v5, section 8.5, slices 0.6 to 0.8.

Run on the Mac: `ENGINE_DIR=<dir with "Chromium Framework.framework"> ./run.sh a|b|c`.
Each step prints one JSON line. A proof passes when `result` has `"pass":true` and the exit code is 0.

## Build tool exception (cargo, not blaze)

The standing rule is to build with `blaze`. `blaze` does not build this crate, so the owner allowed `cargo build --release`
and `cargo run --release` for this crate only (firstmate inbox 001, 2026-11-23). Other crates keep using blaze.

Blaze versions tried on the Mac (arm64, macOS 26.2):

- `blaze 0.20261027.0 (49ca152)`: `blaze build --release` refused 3 groups of functions:
  1. `std::thread::spawn` (rune `Renderer::start_lcd`, `rune_text::TextSystem::mask_cache_frame`, `rune_media` `SurfaceRenderer`, rune watchdog):
     `unsupported: writable static MIN without a rustc symbol`.
  2. `std` `thread_local` `.with` (`rune_staging::Inner::encode`, `rune::clipboard::MemoryClipboard::handle`):
     `unsupported: writable static __RUST_STD_INTERNAL_VAL without a rustc symbol`.
  3. `objc2` `define_class` with `CGRect` (`accesskit_macos::node::PlatformNode::class`):
     `unsupported: Alias(... __Inner) in the C ABI`.
- `blaze 0.20261122.0`: has `check` only. `blaze build` and `blaze run` answer `blaze has no build command` / a usage error for `check`.

These sit in Rune, accesskit and `std`, so the proof crate cannot remove them.

## Proof B and C results (Mac arm64, macOS 26.2)

- Proof B: `./run.sh b` passes: 22 checks, 0 fail (tab delegate, pointer, click, wheel, focus, text keys, Backspace, Cmd+A, arrows, IME preedit and commit through `NSTextInputClient`, candidate rectangle, 100 moves + 20 wheels + 50 keys with no lost event, dark mode, live resize, frame times). Check "Hiragana input source" is NOT run: it needs the app in front (`ABC_IME_FOREGROUND=1 ./run.sh b`). The queue drain during menu tracking and during a user's live resize are NOT run (they need the window in front and a pointer drag).
- Proof C: `./run.sh c` passes: 11 checks, 0 fail (transparent layer, z order page/glass/Rune, click over the page reaches the layer only, 2 layers read one counter, accessibility children, AXButton label). VoiceOver itself and the omnibox case are NOT run (VoiceOver needs the app in front).
- Load case: `./run.sh b --load 1` and `./run.sh c --load 2`; numbers in `rust/RESOURCES.md`.
