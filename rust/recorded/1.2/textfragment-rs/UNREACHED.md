# Lines that `textfragment-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 72 of 73 lines (98 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `textFragment.ts` | 59 | No input of any crate reaches these lines. Line 59 is the `}` that closes the `if` after `for (let n = 1; ; n++)` (`textFragment.ts:52-58`). The loop has no condition and always `return`s, so control never leaves it. |
