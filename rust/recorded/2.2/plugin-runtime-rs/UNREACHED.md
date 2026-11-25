# Lines that `plugin-runtime-rs` does not reach

`rust-anything gate` reports all lines of `pluginRuntime.ts` and of its harness as run. The lines below are in a file of another unit.

| File (`apps/browser/src/lib/`) | Lines | Why |
| --- | --- | --- |
| `pluginChecks.ts` | 87-91, 114-120 | Covered by unit `plugin-checks-rs`, whose coverage reaches all lines of `pluginChecks.ts`. `pluginRuntime.ts` does not call `list` with these values, or the checks at those lines. |
