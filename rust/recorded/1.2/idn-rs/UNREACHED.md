# Lines that `idn-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 321 of 323 lines (99 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `idn.ts` | 99-100 | The fallback in `scx` runs only if Node rejects a `Script_Extensions` name. Node v26.12.1 accepts all 29 names in `SCRIPTS`, so no input reaches the fallback. |
