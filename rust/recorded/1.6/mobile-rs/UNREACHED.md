# Lines that `mobile-rs` does not reach

The recorded cases ran 12822 of 14100 lines (91 %). The lines below are the rest.
Lines of a file that belongs to another unit count as covered by that unit, named in the table. Other lines are lines that no input can reach, with the reason.

| File (`packages/core/src/`) | Lines | Why |
| --- | --- | --- |
| `anonymousIdentity.ts` | 42-49, 51-60, 65-71, 76-77, 81-95 | Covered by unit `anonymous-identity-rs` (slice 1.6); its coverage reaches these lines. |
| `appUrls.ts` | 28-30, 53-62 | Covered by unit `appurls-rs` (slice 1.2); its coverage reaches these lines. |
| `bangs.ts` | 22-23, 31-32, 34-35, 50-55, 59-61, 88-89 | Covered by unit `bangs-rs` (slice 1.4); its coverage reaches these lines. |
| `bangs.ts` | 195, 213 | Covered by unit `bangs-harness-rs` (slice 1.4); its coverage reaches these lines. |
| `calculator.ts` | 41-43, 56-63, 112-116, 118-119, 121, 130-132, 134-136, 143-145, 153-154, 156-157, 159-160, 172-175, 178-187, 191-193, 196-201, 211-212, 291-296, 298-299, 301, 313-315 | Covered by unit `calculator-rs` (slice 1.3); its coverage reaches these lines. |
| `candidatePool.ts` | 55-64, 82-85, 125-136, 155-167, 171-183 | Covered by unit `candidate-pool-rs` (slice 1.3); its coverage reaches these lines. |
| `cleanUrl.ts` | 40-41, 62-63, 77-78 | Covered by unit `cleanurl-rs` (slice 1.2); its coverage reaches these lines. |
| `engines.ts` | 105-115, 122-123, 166-176, 179-180 | Covered by unit `engines-rs` (slice 1.4); its coverage reaches these lines. |
| `extensionRows.ts` | 20-27 | Covered by unit `extension-rows-rs` (slice 1.4); its coverage reaches these lines. |
| `fuzzy.ts` | 11-19, 22-36, 40-43, 52-57, 62-76, 85-95 | Covered by unit `fuzzy-rs` (slice 1.1); its coverage reaches these lines. |
| `idn.ts` | 99-100 | See the [IDN coverage explanation](../../1.2/idn-rs/UNREACHED.md). |
| `idn.ts` | 11-17, 22-52, 55-86, 123-128, 130-134, 136-143, 147-158, 160-170, 229-256, 258-265, 300-302, 306-315, 321-323, 327-330, 334-336, 340-350 | Covered by unit `idn-rs` (slice 1.2); its coverage reaches these lines. |
| `ingestKeys.ts` | 16-18 | Covered by unit `ingest-keys-rs` (slice 1.6); its coverage reaches these lines. |
| `mobile.ts` | 13, 15-21 | No input of any crate reaches these lines. `HostURL` (`mobile.ts:11-22`) is a polyfill that line 25 installs only when `typeof scope.URL === "undefined"` (`mobile.ts:24-25`). Node defines `URL`, so the class is never built or called. |
| `omnibox.ts` | 68-71, 86-88, 90, 96, 105-114, 132-133, 150, 167-171, 176-181, 184-189 | Covered by unit `omnibox-rs` (slice 1.3); its coverage reaches these lines. |
| `openTabs.ts` | 6-12 | Covered by unit `open-tabs-rs` (slice 1.3); its coverage reaches these lines. |
| `pageMatch.ts` | 27-49, 60-64, 80-81, 93-95 | Covered by unit `suggest-rs` (slice 1.3); its coverage reaches these lines. |
| `pageText.ts` | 51-61 | Covered by unit `candidate-pool-rs` (slice 1.3); its coverage reaches these lines. |
| `providerConfig.ts` | 35-38, 44-64, 66-89, 92-108, 113-117, 120-137 | Covered by unit `provider-config-rs` (slice 1.4); its coverage reaches these lines. |
| `providerNetwork.ts` | 9-13, 15-29, 32-37, 39-48, 59-70, 73-97 | Covered by unit `provider-network-rs` (slice 1.4); its coverage reaches these lines. |
| `providerRows.ts` | 30-34, 37-42, 45-48, 51-77, 80-83, 86-102, 105-117, 120-135 | Covered by unit `provider-rows-rs` (slice 1.4); its coverage reaches these lines. |
| `quickCreate.ts` | 34-39 | Covered by unit `quick-create-rs` (slice 1.3); its coverage reaches these lines. |
| `siteSearch.ts` | 52, 54, 84-85, 96-99, 102-105, 125-127, 130-131, 134-143 | Covered by unit `site-search-rs` (slice 1.4); its coverage reaches these lines. |
| `suggest.ts` | 51-53, 63, 108-111, 120-122, 136-141, 200-207, 209-211 | Covered by unit `suggest-rs` (slice 1.3); its coverage reaches these lines. |
| `suggestFetcher.ts` | 12-24, 26-32, 34-38, 40-79 | Covered by unit `suggest-fetcher-rs` (slice 1.3); its coverage reaches these lines. |
| `suggestionRows.ts` | 25-32, 56-60 | Covered by unit `suggestion-rows-rs` (slice 1.3); its coverage reaches these lines. |
| `textFragment.ts` | 59 | See the [text fragment coverage explanation](../../1.2/textfragment-rs/UNREACHED.md). |
| `textFragment.ts` | 18-23, 28-37, 40-47, 49-58, 60-69, 73-76, 79-85 | Covered by unit `textfragment-rs` (slice 1.2); its coverage reaches these lines. |
| `theme.ts` | 58-67, 69-75, 81-87, 90-93, 99-108, 111-119, 122-123, 126-130, 133-135, 138-139, 142-143, 151-154, 157-164, 167-168, 171-175, 178-179, 182-183, 186-192, 195-196, 199-200, 203-205, 208-209, 212-213, 216-217, 220-221, 223-227, 229-231, 234-236, 239-240, 242-246, 249-250, 253-254, 258-261, 264-268, 271-272, 275-277, 280-282, 285-286, 289-295, 298-300, 303-304, 307-315, 317-323, 325-345, 348-353 | Covered by unit `theme-rs` (slice 1.6); its coverage reaches these lines. |
| `webStore.ts` | 6-19, 22-47, 49-53 | Covered by unit `webstore-rs` (slice 1.2); its coverage reaches these lines. |
