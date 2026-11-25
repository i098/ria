# Recorded cases of slice 2.46

Check U for the units of `packages/sync/src` that `ria_sync::logic` ports. Each crate has the
recorded cases of the TypeScript original (`cases.jsonl`), its inputs (`*.inputs.jsonl`) and a thin
`src/port.rs` that calls `ria_sync`. A unit whose exports mutate their argument, take closures,
or are classes or async runs through a TypeScript harness (`*.harness.ts`): a JSON script that both
the original and `port.rs` interpret step by step.

| Crate | Unit | Rust module |
| --- | --- | --- |
| `applied-rs` | `applied.ts` via `applied.harness.ts` | `logic::applied` |
| `canon-rs` | `canon.ts` | `logic::canon` |
| `file-health-rs` | `fileHealth.ts` via `file-health.harness.ts` | `logic::file_health` |
| `folders-rs` | `folders.ts` via `folders.harness.ts` | `logic::folders` |
| `hlc-rs` | `hlc.ts` via `hlc.harness.ts` | `logic::hlc` |
| `order-rs` | `order.ts` | `logic::order` |
| `scope-state-rs` | `scopeState.ts` (`emptyScope`) | `logic::scope_state` |
| `scope-keys-rs` | `scopeKeys.ts` via `scope-keys.harness.ts` | `logic::scope_keys` |
| `scope-rs` | `scope.ts` via `scope.harness.ts` | `logic::scope` |
| `extensions-rs` | `extensions.ts` via `extensions.harness.ts` | `logic::extensions` |
| `companion-legacy-rs` | `companion/legacy.ts` (`fromLegacyUrl`) | `logic::companion::legacy` |
| `companion-theme-rs` | `companion/theme.ts` | `logic::companion::theme` |
| `companion-view-rs` | `companion/view.ts`, `legacy.ts`, `theme.ts` via `companion-view.harness.ts` | `logic::companion::{view,legacy,theme}` |
| `companion-cycle-rs` | `companion/adapters.ts`, `state.ts`, `cycle.ts` via `companion-cycle.harness.ts` | `logic::companion::{adapters,state,cycle}` |

`rust-anything bridge` refuses `index.ts` (re-exports only), `records.ts` (types only),
`native.ts` and `sharedFolders.ts` (every export must stay in TypeScript: they take native
objects). They have no crate. `logic::native` and `logic::shared_folders` carry their types and
traits, and a Rust unit test covers `folder_transport`. `companion/index.ts` and `fromLegacy`
(generic) stay in TypeScript for the bridge; `companion-view-rs` records `fromLegacy`.

The fuzz of a harness crate feeds random strings, so most fresh cases are "bad script"; the
recorded scripts are the proof. `canon-rs` and `scope-state-rs` make no fresh cases.

## Run

Set up the oracle first, from the repository root (`git submodule update --init --checkout ardia`):

```sh
rust/recorded/2.46/review-root.sh
```

Run it again after a change to a `*.harness.ts` file. Then, for each crate:

```sh
rust-anything diff   rust/recorded/2.46/<crate>
rust-anything gate   rust/recorded/2.46/<crate>
rust-anything fuzz   rust/recorded/2.46/<crate> --seed <new seed>
```

To record the harness crates again: `TZ=UTC rust-anything record <crate> --count 0 --inputs <inputs file>`.
`companion-cycle-rs` reads `Date.now()` for ghost devices, so it needs the clock hook:

```sh
TZ=UTC RIA_NOW_MS=1790812800000 NODE_OPTIONS=--import=$PWD/rust/recorded/2.46/companion-cycle-rs/clock-freeze.mjs \
  rust-anything diff rust/recorded/2.46/companion-cycle-rs
```

## Known differences

- `serde_json` refuses JSON nested past 127 levels; `JSON.parse` accepts it. This is a library difference: `parse_sync_file` gives `None` (see `scope.rs`).

Fuzz runs can also report these two harness-only differences. The library and the recorded cases are not affected.

- `companion-view-rs`: `themeOf` with a null scope returns a result where TS throws.
- `scope-rs`: a missing third `hide` argument clears the list where TS pushes `undefined`.
