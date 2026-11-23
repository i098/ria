# Recorded cases of slice 1.3

Each `*-rs` folder holds a crate made by `rust-anything bridge` and its recorded cases (`cases.jsonl`).
`rust-anything.json` in each folder names the TypeScript unit with a path in the scratch checkout where it was bridged.

To record again from a fresh checkout:

1. Check out the pinned Ardia source (commit `551d3c13c`) and copy its `packages/` folder into a scratch repository.
2. Copy each `*.harness.ts` from the crate folders (`suggest-rs`, `candidate-pool-rs`, `suggestion-rows-rs`, `suggest-fetcher-rs`) into `packages/core/src/` of that repository.
3. Set `root` in each `rust-anything.json` to that repository, then run `rust-anything record <crate> --inputs inputs-<unit>.jsonl`, `build`, `diff` and `gate`.

Run the Rust tests with `cargo test`, not `blaze test`.
