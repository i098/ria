# Lines that `match-pattern-rs` does not reach

`rust-anything gate` reports all lines of `matchPattern.ts` as run. The lines below are in a file of another unit.

| File (`apps/browser/src/lib/`) | Lines | Why |
| --- | --- | --- |
| `urlParts.ts` | 118-120, 123-125 | Covered by unit `url-parts-rs`; its coverage reaches all lines of `urlParts.ts`. `matchPattern.ts` does not call `webUrl` or `minimizedUrl`. |
