# ts-sig

Makes the skeleton of `rust/crates/ria-state` from the TypeScript stores of Ardia at `551d3c13c` (plan section 6.5, slice 2.4).

For each of the 41 folders of `apps/browser/src/state` it writes `rust/crates/ria-state/src/<module>/mod.rs` with:

- the Rust types of the folder (one Rust type for each TypeScript type that the folder declares),
- `pub struct State`: the data fields of the TypeScript store,
- `pub enum Action` (`#[serde(tag = "type")]`): one variant for each exported store action,
- `pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects)` with the body `todo!("slice 2.N")`.

It also writes `src/lib.rs` and `src/kernel/modules.rs` (`AppState`, `AppAction` and one `Module` impl for each module).

```sh
cd rust/tools/ts-sig && bun install        # TypeScript 5.9.3, as in engine-api-gen
node gen.mjs ../../../ardia                # writes the files
node gen.mjs ../../../ardia --check        # writes nothing, exits 1 when a file would change
```

After slice 2.4 merges, the slice of each folder owns the `mod.rs` of the folder and edits it by hand. Run the script again only
to see what a new Ardia commit would change (use `--check` and read the diff); a sync is not in the plan.

## Rules

| Item | Rule |
| --- | --- |
| store | A top-level variable of a state file that has `getState` and `setState`. Its state type is the return type of `getState`. Ardia has no `node_modules` in the frozen checkout, so `stubs/zustand.d.ts` gives the types of `create`. |
| `State` | The data properties (no function) of the store state types. A property belongs to the folder of the file that declares it, so each slice of `useBrowser` goes to the folder that declares it. A property that is `?` or whose type admits `undefined` is an `Option` with `skip_serializing_if`. Names are `serde`-renamed to the TypeScript names. |
| `Action` | The function properties of the store state types, and each exported function of a state file that writes a store: it calls `setState`, a store method, a `const x = storeAction("…")` wrapper, or an exported function that does. React hooks (`use*`) and store creators (`(set, get)`) are not actions. |
| Duplicate names | A store method and an exported function with one name give one variant (the function); see the crate README. |
| Action parameters | One field for each parameter. A parameter of a function type is not in the JSON, so it has no field; the doc comment of the variant names it. |
| Types | `number` is `kernel::Number`, a `Set`, `ReadonlySet` or `Iterable` is a `Vec`, a `Map`, `ReadonlyMap` and a `Record` are a `HashMap` with `String` keys, a `Date` is a `String`, a string-literal union is an enum, a union of objects with one tag property is an enum with `#[serde(tag)]`. A function property is not in the JSON. A type that refers to itself goes through a `Box`. |
| Unknown | A type that the script cannot map becomes `serde_json::Value` and the script prints a warning. |
| Platform files | Files of the form `name.windows.ts`, `.linux.ts`, `.android.ts` and `.web.ts` are not read: the first scope is the Mac. |

State JSON rule (check S): `serde_json::to_value(&state)` with sorted keys equals the data fields of the TypeScript store after
`JSON.parse(JSON.stringify(...))` with sorted keys.
