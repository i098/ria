# Recorded cases of slice 1.6

Check U for the units of `packages/core/src` that `ria_core::identity` ports. Each crate has the
recorded cases of the TypeScript original (`cases.jsonl`), its inputs (`<name>.inputs.jsonl`) and a thin `src/port.rs` that calls `ria_core`.

| Crate | Unit | Rust module |
| --- | --- | --- |
| `anonymous-identity-rs` | `anonymousIdentity.ts` | `identity::anonymous_identity` |
| `ingest-keys-rs` | `ingestKeys.ts` | `identity::ingest_keys` |
| `telemetry-wire-rs` | `telemetryWire.ts` | `identity::telemetry_wire` |
| `theme-rs` | `theme.ts` | `identity::theme` |

`tabGroups.ts` has only a constant and a type: `rust-anything bridge` refuses it ("exports no
functions"), so it has no crate. `identity::tab_groups` carries it.

`mobile.ts` is not in this slice yet: it calls the modules of slices 1.2, 1.3 and 1.4.

## Run

The TypeScript source is in the `ardia` submodule (`packages/core/src`). `record` needs a repository
root that holds it; the `root` field of each `rust-anything.json` is the scratch checkout of the run.

```sh
rust-anything diff   rust/recorded/1.6/<crate>
rust-anything gate   rust/recorded/1.6/<crate>
rust-anything fuzz   rust/recorded/1.6/<crate> --seed <new seed>
```

`telemetry-wire-rs` reads `Date.now()` when a timestamp does not parse. Fix the clock for the
original and the port with the test hook:

```sh
RIA_NOW_MS=1795417631295 NODE_OPTIONS=--import=$PWD/rust/recorded/1.6/telemetry-wire-rs/clock-freeze.mjs \
  rust-anything diff rust/recorded/1.6/telemetry-wire-rs
```
