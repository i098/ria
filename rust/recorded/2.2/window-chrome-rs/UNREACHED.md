# Lines that `window-chrome-rs` does not reach

`rust-anything gate` reports all lines of `windowChrome.ts` as run except the body of `onWindowChromeChange`.

| File (`apps/browser/src/lib/`) | Lines | Why |
| --- | --- | --- |
| `windowButtonLayout.ts` | 43-46 | Covered by unit `window-button-layout-rs`; its coverage reaches all lines of `windowButtonLayout.ts`. This unit does not call `buttonLayoutSide`. |
| `windowChrome.ts` | 32-34 | `onWindowChromeChange` takes a function, which the bridge cannot carry (`rust-anything bridge` keeps it in TS). The unit test `watchers_run_once_each_until_removed` in `window_chrome.rs` checks the Rust port. |
