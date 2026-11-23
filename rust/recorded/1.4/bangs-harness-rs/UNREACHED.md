# Lines that `bangs-harness-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 150 of 230 lines (65 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `bangs.ts` | 14-24, 26-38, 41-66, 84-113 | Covered by unit `bangs-rs` (slice 1.4); its coverage reaches these lines. `bangs-rs` records them. |
