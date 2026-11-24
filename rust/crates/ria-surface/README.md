# Ria surface host

Create one `WindowHost` for each macOS window.
Pass the window's content view, the shared app state handle, and the app's main-thread `ActionQueue`.
Pass the engine tab's `NSView` to `set_page` when the active tab changes.
Give the chrome layout an element whose key matches `WindowSpec::page_slot`.
The host uses the element's rectangle to place the page.
Use `set_overlay_visible` when the app shows or closes an overlay.
After the app changes shared state, call `mark_dirty` for each layer that reads the new state.
Drop the window hosts before the engine shuts down.

The native views queue input and display ticks instead of calling the app from AppKit callbacks.
Hit tests, text input queries, accessibility queries, and size callbacks return during the native call.
The next display tick uses the size that AppKit reported.

## Checks

Run these commands from a fresh checkout with the documented Blaze tools installed:

```sh
(cd rust && blaze check -p ria-surface)
(cd rust && blaze test -p ria-surface)
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml -p ria-surface --all-targets -- -D warnings
```

Run the native tests on macOS.
The direct-entry test creates an inactive AppKit window and checks the native replies while the action queue drains.
It also checks that the display link posts a presentation time and the Rune rectangle sizes the native page view.
The text-range tests check UTF-16 AppKit offsets against Rust UTF-8 offsets.
