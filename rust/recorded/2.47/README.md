# Recorded cases of slice 2.47

Check U for `packages/import/src/index.ts` of Ardia, which `ria_import::logic` ports. The crate
`import-rs` has the recorded cases of the TypeScript original (`cases.jsonl`), their inputs
(`import.inputs.jsonl`, made by `gen_inputs.py`) and a thin `src/port.rs` that calls `ria_import`.

`index.ts` loads the native module with `requireNativeModule` from `expo-modules-core`, so Node
cannot run it alone. `import.harness.ts` loads a fresh copy of `index.ts` for each replay over a
stand-in `expo-modules-core` (`expo-modules-core/`) whose native module is a fake that follows a
script: it can emit events, abort the signal, and answer with JSON text, `null` or an error.
Each step of a replay is `[export, ...arguments, script]`. The trace of a step holds the result or
error, each call that reached the native module, the progress and history callbacks, and how many
event and abort listeners are left. `port.rs` reads the same steps and runs `ria_import` over the
same fake.

The harness is the only source file the bridge sees, so `record` covers its lines (129 of 129),
not those of `index.ts`; the trace is what proves `index.ts`.

## Run

Prepare the oracle from the repository root. Set `ARDIA` to a clone of Ardia when the `ardia`
submodule is not checked out.

```sh
git submodule update --init --checkout ardia
rust/recorded/2.47/review-root.sh
```

```sh
rust-anything diff rust/recorded/2.47/import-rs
rust-anything gate rust/recorded/2.47/import-rs
rust-anything fuzz rust/recorded/2.47/import-rs --seed <new seed>
```

To record the cases again:

```sh
python3 rust/recorded/2.47/gen_inputs.py
TZ=UTC rust-anything record rust/recorded/2.47/import-rs --count 0 --inputs rust/recorded/2.47/import.inputs.jsonl
```

[known-diffs.jsonl](import-rs/known-diffs.jsonl) lists the one recorded case with a known difference: the text of a JSON
error is the message of V8 in Ardia and of serde in Ria.
