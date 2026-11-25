# Lines that `platform-chrome-rs` does not reach

`rust-anything gate` reports all lines of `platformChrome.ts` as run. The lines below are in a file of another unit.

| File (`apps/browser/src/lib/`) | Lines | Why |
| --- | --- | --- |
| `windowButtonLayout.ts` | 43-46 | Covered by unit `window-button-layout-rs`; its coverage reaches all lines of `windowButtonLayout.ts`. This unit does not call `buttonLayoutSide`. |
