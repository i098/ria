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

## CI and quality gates

Each PR runs [`.github/workflows/ci.yml`](.github/workflows/ci.yml). The checks are `fmt`, `clippy`, `test`, `parity`, `headers` and `sentrux`. They use cargo, not blaze, because blaze is not on GitHub runners. Run the same checks locally:

```sh
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
rustup target add aarch64-apple-darwin
cargo clippy --locked --manifest-path rust/Cargo.toml --target aarch64-apple-darwin -p ria-host-mac --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --exclude ardia-plugin-jail
cargo test --manifest-path rust/Cargo.toml -p ardia-plugin-jail --lib
node rust/parity/compare.selftest.mjs
(cd rust/parity/captures/states && sha256sum -c SHA256SUMS)
rust/tools/check-headers.sh
```

The `test` job skips the jail escape tests (`rust/crates/ria-plugin-broker/crates/jail/tests/escape.rs`).
Those tests need a `bwrap` that `scripts/build-bwrap.sh` builds (`ARDIA_BWRAP`).
The live engine checks report a skip on Linux.
See the check sources for [`ria-engine-sys`](rust/crates/ria-engine-sys/tests/live.rs) and [`ria-engine`](rust/crates/ria-engine/src/domains/data/live.rs) for the macOS requirements.
See the [app launch check](rust/crates/ria-app/src/launch/README.md#launch-checks) for the macOS app test.
See the [surface host checks](rust/crates/ria-surface/README.md#checks) for the macOS-only `ria-surface` tests.

Follow the [macOS instructions for window-host Check L](rust/crates/ria-host-mac/tests/live_windows.rs#L3-L10).

### Run Linux checks on Cloudflare

After the [remote workflow](.github/workflows/remote-check.yml) is on `main`, push your branch with that workflow file.
Run these commands from your checkout. Replace `<branch>` with your branch name.

```sh
gh workflow run remote-check.yml --ref <branch>
gh run list --workflow remote-check.yml --branch <branch> --limit 5
gh run watch <run-id> --exit-status
```

Use the run ID from the list. If a job fails, run `gh run view <run-id> --log-failed`.
The workflow runs the Linux CI checks and builds all workspace test binaries.
It does not run the Mac checks or the jail escape tests.

### Local quality gates

The `sentrux` check runs in CI (version 0.5.7, pinned). Run it and one more gate locally:

```sh
sentrux gate .      # compares with the baseline in .sentrux/; a DEGRADED verdict fails the CI job
fallow audit        # advisory; fallow reads TypeScript/JavaScript only, so it covers little of this repository
```

Save a new baseline with `sentrux gate --save .` only when the owner agrees, and record the reason and the before and after numbers in `rust/DECISIONS.md` (see ADR-Q1).

Build and test Rust code with Blaze.
Use Cargo for `fmt`, `clippy`, and [tests that Blaze cannot run](rust/tools/bang-tsv/README.md).
Follow Google's Rust Style Guide.
Add tests only for algorithmic logic, boundary IO and integration flows.

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
