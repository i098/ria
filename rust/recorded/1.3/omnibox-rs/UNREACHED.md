# Lines that `omnibox-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 670 of 698 lines (95 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `appUrls.ts` | 28-30, 53-62 | Covered by unit `appurls-rs` (slice 1.2); its coverage reaches these lines. |
| `cleanUrl.ts` | 77-78 | Covered by unit `cleanurl-rs` (slice 1.2); its coverage reaches these lines. |
| `idn.ts` | 99-100 | No input of any crate reaches these lines. The `catch` of `scx` (`idn.ts:96-100`) runs only when the JavaScript engine rejects `\p{Script_Extensions=<name>}`. Node v26.12.1 accepts all 45 names in `SCRIPTS` (`idn.ts:88-92`), so no input reaches lines 99-100. |
| `idn.ts` | 340-350 | Covered by unit `idn-rs` (slice 1.2); its coverage reaches these lines. |
