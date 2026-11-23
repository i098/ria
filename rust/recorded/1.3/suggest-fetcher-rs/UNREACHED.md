# Lines that `suggest-fetcher-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 550 of 992 lines (55 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `appUrls.ts` | 11-17, 20-25, 28-30, 34-39, 42-44, 53-62 | Covered by unit `appurls-rs` (slice 1.2); its coverage reaches these lines. |
| `cleanUrl.ts` | 31-34, 36-42, 44-48, 53-70, 73-74, 77-78 | Covered by unit `cleanurl-rs` (slice 1.2); its coverage reaches these lines. |
| `engines.ts` | 92-96, 102-117, 122-123, 126-127, 166-176, 179-180 | Covered by unit `engines-rs` (slice 1.4); its coverage reaches these lines. |
| `idn.ts` | 99-100 | See the [IDN coverage explanation](../../1.2/idn-rs/UNREACHED.md). |
| `idn.ts` | 11-17, 22-52, 55-86, 123-128, 130-134, 136-143, 147-158, 160-170, 229-256, 258-265, 300-302, 306-315, 320-337, 340-350 | Covered by unit `idn-rs` (slice 1.2); its coverage reaches these lines. |
| `omnibox.ts` | 10-14, 17-18, 30-38, 44-48, 52-60, 68-71, 74-76, 82-100, 105-114, 119-121, 123-128, 130-135, 137-155, 167-171, 176-181, 184-189, 194-209, 212-217 | Covered by unit `omnibox-rs` (slice 1.3); its coverage reaches these lines. |
