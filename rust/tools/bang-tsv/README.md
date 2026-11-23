# Bundled bang lists

`bang_list.tsv` contains both runtime strings from Ardia's `packages/core/src/bangList.ts` at `551d3c13c` (Copyright 2026 Mantra Patel, Apache-2.0).
The primary list comes first, and the Unicode list comes second.
Keep the lists separate at lookup time so the primary list takes precedence.

Run these commands from the Ria root with Node.js and the frozen `ardia/` submodule initialized:

```sh
node rust/tools/bang-tsv/extract.mjs data > rust/crates/ria-core/src/search/bang_list.tsv
node rust/tools/bang-tsv/extract.mjs samples > rust/tools/bang-tsv/samples.tsv
(cd rust && blaze test -p ria-core --lib)
```

Pass an Ardia checkout path after `data` or `samples` to use that checkout instead of the submodule.
The extractor reads the pinned Git object and takes the JavaScript string values, not the raw template text.
The Rust test checks each list's row count and 100 evenly spaced rows, including both ends.
