# ria-state

One module for each of the 41 state folders of Ardia, plus the store kernel. `rust/tools/ts-sig` made the skeleton (slice 2.4).

## Actions that have no variant

A store method and an exported function with one name give one `Action` variant: the generator keeps the first and skips the other.
Two names are affected, and the slice of each folder must handle the skipped one:

- `plugins`: `disablePlugin` (the store method is skipped, the exported function stays).
- `splits`: `openSplitPane` (the store method is skipped, the exported function stays).
