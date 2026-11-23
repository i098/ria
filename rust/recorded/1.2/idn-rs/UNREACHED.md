# Lines that `idn-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 321 of 323 lines (99 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `idn.ts` | 99-100 | No input of any crate reaches these lines. The `catch` of `scx` (`idn.ts:96-100`) runs only when the JavaScript engine rejects `\p{Script_Extensions=<name>}`. Node v26.12.1 accepts all 45 names in `SCRIPTS` (`idn.ts:88-92`), so no input reaches lines 99-100. |
