# Recorded cases of slice 1.3

Each `*-rs` folder holds a crate made by `rust-anything bridge` and its recorded cases (`cases.jsonl`).

Set up the [shared Ardia oracle](../../GATES.md#phase-1-check-u) before you record cases again.
The setup uses the pinned Ardia commit, not local edits.
Run `rust-anything record <crate> --inputs inputs-<unit>.jsonl`, then run `build`, `diff`, and `gate`.

Run the Rust tests with `cargo test`, not `blaze test`.
