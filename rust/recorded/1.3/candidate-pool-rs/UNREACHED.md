# Lines that `candidate-pool-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 747 of 957 lines (78 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `appUrls.ts` | 28-30, 53-62 | Covered by unit `appurls-rs` (slice 1.2); its coverage reaches these lines. |
| `cleanUrl.ts` | 31-34, 36-42, 44-48, 53-70, 73-74, 77-78 | Covered by unit `cleanurl-rs` (slice 1.2); its coverage reaches these lines. |
| `idn.ts` | 99-100 | See the [IDN coverage explanation](../../1.2/idn-rs/UNREACHED.md). |
| `idn.ts` | 153, 155, 157, 164-165, 167-168, 248, 254-255, 340-350 | Covered by unit `idn-rs` (slice 1.2); its coverage reaches these lines. |
| `omnibox.ts` | 10-14, 17-18, 30-38, 44-48, 52-60, 68-71, 74-76, 82-100, 105-114, 119-121, 123-128, 130-135, 137-155, 162-164, 167-171, 176-181, 184-189, 194-209 | Covered by unit `omnibox-rs` (slice 1.3); its coverage reaches these lines. |
