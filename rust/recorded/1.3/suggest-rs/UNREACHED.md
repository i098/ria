# Lines that `suggest-rs` does not reach

`rust-anything gate` prints: the recorded cases ran 1963 of 2383 lines (82 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `appUrls.ts` | 14-15, 28-30, 36-37, 53-62 | Covered by unit `appurls-rs` (slice 1.2); its coverage reaches these lines. |
| `bangs.ts` | 14-24, 26-38, 41-66, 84-113 | Covered by unit `bangs-rs` (slice 1.4); its coverage reaches these lines. |
| `bangs.ts` | 162-173, 178-185, 188-198 | Covered by unit `bangs-harness-rs` (slice 1.4); its coverage reaches these lines. |
| `calculator.ts` | 118-119, 121, 130-132, 134-136, 159-160, 181-183 | Covered by unit `calculator-rs` (slice 1.3); its coverage reaches these lines. |
| `candidatePool.ts` | 109-121 | Covered by unit `candidate-pool-rs` (slice 1.3); its coverage reaches these lines. |
| `cleanUrl.ts` | 31-34, 36-42, 44-48, 53-70, 73-74, 77-78 | Covered by unit `cleanurl-rs` (slice 1.2); its coverage reaches these lines. |
| `engines.ts` | 92-96, 102-117, 122-123, 126-127, 145-159, 166-176, 179-180 | Covered by unit `engines-rs` (slice 1.4); its coverage reaches these lines. |
| `fuzzy.ts` | 22-36 | Covered by unit `fuzzy-rs` (slice 1.1); its coverage reaches these lines. |
| `idn.ts` | 99-100 | See the [IDN coverage explanation](../../1.2/idn-rs/UNREACHED.md). |
| `idn.ts` | 26-29, 153, 155, 157, 164-165, 167-168, 248, 254-255, 340-350 | Covered by unit `idn-rs` (slice 1.2); its coverage reaches these lines. |
| `omnibox.ts` | 68-71, 167-171, 176-181, 184-189, 194-209 | Covered by unit `omnibox-rs` (slice 1.3); its coverage reaches these lines. |
| `providerRows.ts` | 33, 80-83, 86-102, 105-117 | Covered by unit `provider-rows-rs` (slice 1.4); its coverage reaches these lines. |
| `siteSearch.ts` | 51-56, 58-60, 62-65, 68-72, 74-89, 91-93, 102-105, 125-127, 130-131, 134-143 | Covered by unit `site-search-rs` (slice 1.4); its coverage reaches these lines. |
| `suggestionRows.ts` | 66-69 | Covered by unit `suggestion-rows-rs` (slice 1.3); its coverage reaches these lines. |
