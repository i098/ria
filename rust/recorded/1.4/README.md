# Recorded cases of slice 1.4

The `root` of each crate here is `.review-ardia` in the crate folder. `rust-anything fuzz` runs the
TypeScript original from there. Build the folders from a checkout of the `ardia` submodule
(`packages/core`), before `fuzz`:

```sh
rust/recorded/1.4/review-root.sh ardia
```

The script also copies `bangs.harness.ts` for `bangs-harness-rs`. `.review-ardia/` is ignored by git.
