# State replay kit

Use the kit to record the behavior of an Ardia state module and compare each step with its Rust port.
The find sample is a standalone crate. It does not change `ria-state`.

## Check REC on Linux

Use Node.js 26, pnpm 12.21.0, Blaze, and `rust-anything` 3.3.0 or later.
Start in a Ria clone with the Ardia submodule initialized. Use a temporary parent directory with space for Ardia and its packages.

```sh
scratch=$(rust/replay/kit/setup.sh "$(pwd)/ardia" /sdb-disk/tmp)
node --test rust/replay/kit/replay.test.mjs
rust/replay/kit/check-find.sh "$scratch"
```

Replace `/sdb-disk/tmp` with an existing temporary directory on your machine.
Setup archives Ardia at `551d3c13c`, then installs its locked dependencies without lifecycle scripts. It does not change the source checkout.
The check writes the JSONL recording and builds the Rust sample under the scratch path. It must record at least 200 cases and match every case.
Keep the printed `cases.jsonl` path for investigation. Remove the scratch directory when you no longer need the recording.

## Record another store

1. Load the needed Ardia modules with `loadArdia(scratch)`. It registers Ardia's test loader and its React Native and Expo fakes.
2. Use `replayDriver(createCase)` to make a `replay(actions)` function. Supply `apply`, `state`, `platform`, and `cleanup` for each case.
3. Return the complete data fields for the module from `state`. Use stable names for generated IDs and supply deterministic inputs for clocks or other nondeterministic values.
4. Use `nativeFake().method(name, result)` to record calls in order and return fixture results. Give the Rust fake the same results as inputs.
5. Call `replay` once per case, in sequence if cases share an Ardia singleton. Write one `{ fn: "replay", args: [actions], ok: steps }` JSON object per line.

Each output step contains `state`, `calls`, and `result` when the action returns a value. Each call records `name`, `args`, and `result` when present.
State JSON omits function and undefined object fields and preserves null. Maps become objects and sets become sorted arrays.
String sets sort by UTF-16 order. Other sets sort by canonical JSON text. Object keys are sorted, except integer keys use JavaScript's JSON ordering.
The diff compares object keys without regard to order but checks array order, including the order of calls after each action.

`find.mjs` uses the real Ardia find actions and store. It records the full find state for two tabs under stable names and tests an absent tab.
`record-find.mjs` varies queries, replace mode, direction, and webview availability. It includes Unicode queries and checks the Enter key timing boundary.
The sample records the effects requested by the Rust state update and runs them against a fake webview. Its adapter sends each replay case to the Rust binary.
