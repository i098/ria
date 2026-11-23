# Recorded cases of slice 1.6

Check U for the units of `packages/core/src` that `ria_core::identity` ports. Each crate has the
recorded cases of the TypeScript original (`cases.jsonl`), its inputs (`<name>.inputs.jsonl`) and a thin `src/port.rs` that calls `ria_core`.

| Crate | Unit | Rust module |
| --- | --- | --- |
| `anonymous-identity-rs` | `anonymousIdentity.ts` | `identity::anonymous_identity` |
| `ingest-keys-rs` | `ingestKeys.ts` | `identity::ingest_keys` |
| `mobile-rs` | `mobile.ts` and `errors.ts` via `mobile.harness.ts` | `identity::mobile` and `errors` |
| `telemetry-wire-rs` | `telemetryWire.ts` | `identity::telemetry_wire` |
| `theme-rs` | `theme.ts` | `identity::theme` |

`tabGroups.ts` has only a constant and a type: `rust-anything bridge` refuses it ("exports no
functions"), so it has no crate. `identity::tab_groups` carries it.

The mobile harness calls all 13 mobile exports in order in one session.
The [input sequence](mobile.inputs.jsonl) replaces saved pages, search engines, and bang tables.
The sequence checks the state after each replacement.
The harness also records construction of `ArdiaError` and both branches of `ardiaError`.

## Run

Set up the [shared Ardia oracle](../../GATES.md#phase-1-check-u) before you run `record`, `gate`, or `fuzz`.
The setup exports the pinned commit, so local Ardia edits do not change the oracle.

```sh
rust-anything diff   rust/recorded/1.6/<crate>
rust-anything gate   rust/recorded/1.6/<crate>
rust-anything fuzz   rust/recorded/1.6/<crate> --seed <new seed>
```

To record the mobile sequences again, run these commands from the repository root:

```sh
TZ=UTC rust-anything record rust/recorded/1.6/mobile-rs --count 0 --inputs rust/recorded/1.6/mobile.inputs.jsonl
rust-anything build rust/recorded/1.6/mobile-rs
```

Use the [mobile gate command](../../GATES.md#phase-1-check-u) after the build.

`telemetryWire.ts` reads `Date.now()` when a timestamp does not parse.
Use the clock hook and the same `RIA_NOW_MS` for `record`, `diff`, `gate`, and `fuzz`.
The hook sets the time zone to UTC.
See [known-diffs.jsonl](known-diffs.jsonl) for the parity exception outside UTC.

```sh
TZ=UTC RIA_NOW_MS=1795417631295 NODE_OPTIONS=--import=$PWD/rust/recorded/1.6/telemetry-wire-rs/clock-freeze.mjs \
  rust-anything diff rust/recorded/1.6/telemetry-wire-rs
```
