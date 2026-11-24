# Lines that `bangs-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 123 of 210 lines (58 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `bangs.ts` | 115-118, 120-138, 141-159, 162-173, 178-185, 188-198, 201-214 | Covered by unit `bangs-harness-rs` (slice 1.4); its coverage reaches these lines. The bridge did not export `bangTable, checkedBangTable, findBang, joinBangTables, readBangDocument`, so this crate has no input for them; `bangs-harness-rs` records them. |
