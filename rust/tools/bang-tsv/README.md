# Bundled bang lists

`bang_list.tsv` and `samples.tsv` contain data from Ardia's [`packages/core/src/bangList.ts`](https://github.com/ardia-browser/ardia/blob/551d3c13c1c9ede130724ea9f1056d4b586fbe55/packages/core/src/bangList.ts).
Ardia took this snapshot from [Helium's bang list](https://services.helium.imput.net/bangs.json).
The data is copyright Kagi Search and imput under the [MIT License](../../crates/ria-core/src/search/bang_list.LICENSE).
The primary list comes first, and the Unicode list comes second.
Keep the lists separate at lookup time so the primary list takes precedence.

Run these commands from the Ria root with Node.js and the frozen `ardia/` submodule initialized:

```sh
regenerate() (
  temp=$(mktemp "$2.XXXXXX") || exit 1
  trap 'rm -f "$temp"' EXIT
  node rust/tools/bang-tsv/extract.mjs "$1" > "$temp" && mv "$temp" "$2"
)

regenerate data rust/crates/ria-core/src/search/bang_list.tsv &&
  regenerate samples rust/tools/bang-tsv/samples.tsv
```

The extractor reads the pinned Git object and takes the JavaScript string values, not the raw template text.
`samples.tsv` records each list's row count and 100 evenly spaced rows with zero-based indexes, including both ends.
The Rust test compares both lists with that fixture.

Run the parity test with Cargo, as in [CI](../../../CONTRIBUTING.md#ci-and-quality-gates).
See the [Blaze test-harness limit](../blaze/README.md#rust-test-harness-limit).

```sh
cargo test --manifest-path rust/Cargo.toml -p ria-core --lib bundled_lists_match_frozen_ardia_samples
```
