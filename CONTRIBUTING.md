# Contributing to Ria

Thank you for helping. Read the [Code of Conduct](CODE_OF_CONDUCT.md) first.

## Set up, build, test

Follow [`setup.md`](setup.md). Before a PR, run:

```sh
(cd rust && blaze check)
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml
node rust/parity/compare.selftest.mjs
```

Build and test Rust code with blaze. Use cargo only for `fmt` and `clippy`. Follow Google's Rust Style Guide. Add tests only for algorithmic logic, boundary IO and integration flows.

## Flow

1. Open an issue first. Describe the bug or the change.
2. Fork the repository and make a branch from `main`.
3. Make a small change. Do not edit `ardia/`: it is a frozen submodule.
4. Open a PR. Write `Closes #N` in the body. Fill in the PR template.

## Commit rule

Each commit message and each PR title follows [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): imperative summary`.

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Mark a breaking change with `!` after the type or with a `BREAKING CHANGE:` footer.

Example: `feat(core): add fuzzy score for command bar`

## License

You submit your work under Apache-2.0, as in [`LICENSE`](LICENSE).
