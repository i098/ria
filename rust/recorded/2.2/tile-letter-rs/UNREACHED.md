# Lines that `tile-letter-rs` does not reach

`rust-anything gate` reports all lines of `tileLetter.ts` as run. The lines below are in a file of another unit.

| File (`apps/browser/src/lib/`) | Lines | Why |
| --- | --- | --- |
| `url.ts` | 16-23, 25-31, 36-43 | Covered by unit `url-rs`; its coverage reaches all lines of `url.ts`. `tileLetter.ts` calls only `displayHostOf`. |
