# Lines that `provider-network-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 449 of 965 lines (46 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `appUrls.ts` | 11-17, 20-25, 28-30, 34-39, 42-44, 53-62 | Covered by unit `appurls-rs` (slice 1.2); its coverage reaches these lines. |
| `cleanUrl.ts` | 31-34, 36-42, 44-48, 53-70, 73-74, 77-78 | Covered by unit `cleanurl-rs` (slice 1.2); its coverage reaches these lines. |
| `idn.ts` | 99-100 | No input of any crate reaches these lines. The `catch` of `scx` (`idn.ts:96-100`) runs only when the JavaScript engine rejects `\p{Script_Extensions=<name>}`. Node v26.12.1 accepts all 45 names in `SCRIPTS` (`idn.ts:88-92`), so no input reaches lines 99-100. |
| `idn.ts` | 11-17, 22-52, 55-86, 123-128, 130-134, 136-143, 147-158, 160-170, 229-256, 258-265, 300-302, 306-315, 320-337, 340-350 | Covered by unit `idn-rs` (slice 1.2); its coverage reaches these lines. |
| `omnibox.ts` | 10-14, 17-18, 30-38, 44-48, 52-60, 68-71, 74-76, 82-100, 105-114, 119-121, 123-128, 130-135, 137-155, 162-164, 167-171, 176-181, 184-189, 194-209, 212-217 | Covered by unit `omnibox-rs` (slice 1.3); its coverage reaches these lines. |
| `pageText.ts` | 5-6, 37-46, 51-61 | Covered by unit `candidate-pool-rs` (slice 1.3); its coverage reaches these lines. |
| `providerRows.ts` | 30-34, 45-48, 51-77, 80-83, 86-102, 105-117, 120-135 | Covered by unit `provider-rows-rs` (slice 1.4); its coverage reaches these lines. |
