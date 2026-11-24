# ria-plugin-broker

## Running the tests

`cargo test` needs `crates/host/tests/fixtures/sdk-bench/dist/index.js`. The file is not in the repo: it is built from the Ardia plugin SDK.
Without it, `confine_order` stops at start with an error that names the file.

Build it and run the tests with:

    rust/crates/ria-plugin-broker/scripts/ci-check.sh <ardia-checkout>

`<ardia-checkout>` is a checkout of `ardia-browser/ardia` (CI pins commit `551d3c13c1c9ede130724ea9f1056d4b586fbe55`).
