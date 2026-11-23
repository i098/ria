# Recorded cases of slice 1.6

Check U for the units of `packages/core/src` that `ria_core::identity` ports. Each crate has the
recorded cases of the TypeScript original (`cases.jsonl`), its inputs (`<name>.inputs.jsonl`) and a thin `src/port.rs` that calls `ria_core`.

| Crate | Unit | Rust module |
| --- | --- | --- |
| `anonymous-identity-rs` | `anonymousIdentity.ts` | `identity::anonymous_identity` |
| `ingest-keys-rs` | `ingestKeys.ts` | `identity::ingest_keys` |
| `mobile-rs` | `mobile.ts` via `mobile.harness.ts` | `identity::mobile` |
| `telemetry-wire-rs` | `telemetryWire.ts` | `identity::telemetry_wire` |
| `theme-rs` | `theme.ts` | `identity::theme` |

`tabGroups.ts` has only a constant and a type: `rust-anything bridge` refuses it ("exports no
functions"), so it has no crate. `identity::tab_groups` carries it.

The mobile harness calls all 13 mobile exports in order on one session. The input replaces saved
pages, search engines, and bang tables, and checks the state after each replacement.

## Run

The TypeScript source is in the `ardia` submodule (`packages/core/src`). `record` needs a repository
root that holds it; the `root` field of each `rust-anything.json` is the scratch checkout of the run.

```sh
rust-anything diff   rust/recorded/1.6/<crate>
rust-anything gate   rust/recorded/1.6/<crate>
rust-anything fuzz   rust/recorded/1.6/<crate> --seed <new seed>
```

To record the mobile sequence from a fresh checkout, initialize the pinned Ardia submodule.
Run these commands from the repository root:

```sh
mkdir -p .review-ardia
cp -R ardia/packages .review-ardia/
cp rust/recorded/1.6/mobile.harness.ts .review-ardia/packages/core/src/
TZ=UTC rust-anything record rust/recorded/1.6/mobile-rs --count 0 --inputs rust/recorded/1.6/mobile.inputs.jsonl
rust-anything build rust/recorded/1.6/mobile-rs
rust-anything gate rust/recorded/1.6/mobile-rs
```

`telemetry-wire-rs` reads `Date.now()` when a timestamp does not parse. Use the hook for recording
and replay. It fixes the clock and sets the time zone to UTC.
See [known-diffs.jsonl](known-diffs.jsonl) for the exception outside the UTC recording.

```sh
TZ=UTC RIA_NOW_MS=1795417631295 NODE_OPTIONS=--import=$PWD/rust/recorded/1.6/telemetry-wire-rs/clock-freeze.mjs \
  rust-anything diff rust/recorded/1.6/telemetry-wire-rs
```
