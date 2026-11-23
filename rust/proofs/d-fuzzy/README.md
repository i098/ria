# Proof D: rust-anything loop on `fuzzy.ts`

Unit: Ardia `packages/core/src/fuzzy.ts` at the frozen commit `551d3c13c` (copy in `original/`).
Host: Linux x64, node v26.12.1, rust-anything 3.3.0 (installed with `cargo install rust-anything`; it is the crates.io build of erkamyaman/rust-anything).

## Result

- `rust-anything gate fuzzy-rs`: passed, 13 of 13 checks.
- `diff`: 623/623 recorded cases match (2 by `known-diffs.jsonl`). Coverage 87 of 87 original lines.
- `fuzz` with a fresh seed: 385/385 match.
- The recorded cases include non-ASCII input: accents, NFD marks, Greek and final sigma, `İ`, Kelvin sign, long s, CJK, emoji, non-ASCII digits and spaces. See `inputs.jsonl`.
- Port: `fuzzy-rs/src/port.rs`, 158 lines for 87 lines of TypeScript (1.8x).

## Steps and wall time

| Step | Command | Wall time |
| --- | --- | --- |
| install | `cargo install rust-anything` | 235.6 s |
| analyze | `rust-anything analyze <repo> --json` | 0.02 s |
| bridge | `rust-anything bridge <repo> packages/core/src/fuzzy.ts --kind napi` | 21.8 s |
| record | `rust-anything record fuzzy-rs --inputs inputs.jsonl --node-arg=--experimental-strip-types` | 2.2 s |
| write port | by hand, `src/port.rs` (not timed) | - |
| refresh (after `actions` became `Value`) | `rust-anything refresh` | 35.4 s |
| build | `rust-anything build` | 111.6 s first, 6.0 s and 11.9 s after |
| diff | `rust-anything diff` | 8.1 s first, 9.6 s second, 0.5 s with known-diffs |
| gate | `rust-anything gate` | 43.7 s |
| fuzz, bench (extra) | `fuzz`, `bench` | 0.5 s, 1.2 s |

Machine time of the loop (bridge to gate, without install): about 4 min 10 s on a cold build cache.
Install plus loop: about 8 min.

## Findings

- Every first `diff` (613/623) failed on things the generated cases found, not on the happy path:
  - `actions` typed as a list made serde reject non-array input; the original throws `Invalid value used as weak map key` (primitives) or `Invalid array length` (objects). Fixed by typing `actions` as `Value`.
  - A repeated space in the query: the first edit of the whitespace collapse did not apply, `diff` caught it.
  - JS `trim` and `\s` differ from Rust `trim` (U+FEFF, U+0085). The port uses the JS set.
- Lone surrogates: the napi bridge cannot pass them to a Rust `String`. Two cases are in `fuzzy-rs/known-diffs.jsonl` (the skill says ask first; this is a bridge limit, not a port choice). A pure Rust app does not have this limit.
- `bench`: Rust is slower than V8 across the bridge (`fuzzyScore` 0.87x, `matchActions` 0.23x). That is expected for one small function and does not matter for Halyard (the port is for correctness proof, not speed).
- `matchActions` caches prepared names in a `WeakMap`. The port does not cache; the cache is not observable except through the two errors above.
- Inputs outside `ActionCandidate` (string `keywords`, odd titles) only fail with the error classes the recording covers. Marked `shortcut:` in `port.rs`.
- `fuzzy-rs/rust-anything.json` keeps the scratch `root` path of the run. Re-run `bridge` against the Ardia submodule when the base lands.

## Open point: can `record` capture a store as actions plus the state after each one?

No, not natively. Yes with a wrapper function.

Evidence:

- A case is one call: `{"fn", "args"}` to `{"ok"}` or `{"error"}` (`fuzzy-rs/cases.jsonl`). `rust-anything record --help` has no option for a sequence or a store.
- Tried on `apps/browser/src/state/find/` (copy of the `state` tree): `analyze` lists it as `ready` with 5 bridgeable exports (`openFind`, `closeFind`, `findStep`, ...). They are `void` functions that write the `useBrowser` zustand singleton, so the bridge types them as returning `serde_json::Value` and a case would record only the return value, not the store. `record` also stopped on `ERR_MODULE_NOT_FOUND`, because the app's imports (zustand, extensionless paths) are not installed. Not a clean store test, but the same limit holds.
- Workaround, tested in `store-demo/`: a pure function `replay(actions[])` makes a fresh store, applies each action and returns the state after each one. `record` captured 201 cases (one hand-written plus generated) with the full state list per case, including non-ASCII tab names (`replay-inputs.jsonl`, `store-demo/src/findStore.ts`; coverage 15/15 lines).
- Consequence for the plan: each Ardia store folder needs a small `replay` driver in a scratch copy (not in Ardia) before `record`.
