#!/usr/bin/env bash
# Usage: ci-check.sh <ardia-checkout>
#   Builds and tests the nested broker crates (host, jail, integration) on Linux.
#   They are not members of the rust/ workspace and need files from Ardia at the frozen commit:
#   packages/shell/plugin-host (HostPrelude.swift for host/build.rs), packages/plugin-sdk (bundled
#   into the sdk-bench fixture, whose dist/ Ardia does not commit) and tests/fixtures/plugins/v2-showcase.
#   The script copies the crates into a temp layout like Ardia's, so the result does not depend on
#   where this repository lives.
set -euo pipefail
case "${1:-}" in -h|--help|"") sed -n '2,8p' "$0"; exit 0 ;; esac
ardia="$(cd "$1" && pwd)"
here="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

broker="$work/packages/plugin-broker"
mkdir -p "$broker" "$work/packages/plugin-sdk" "$work/packages/shell" "$work/tests/fixtures/plugins"
cp -R "$here/crates" "$here/scripts" "$broker/"
cp -R "$ardia/packages/plugin-sdk/src" "$work/packages/plugin-sdk/src"
cp -R "$ardia/packages/shell/plugin-host" "$work/packages/shell/plugin-host"
cp -R "$ardia/tests/fixtures/plugins/v2-showcase" "$work/tests/fixtures/plugins/v2-showcase"
printf '[workspace]\nresolver = "2"\nmembers = ["crates/*"]\n' > "$broker/Cargo.toml"

# esbuild is the only bundler build.mjs needs; it is resolved from node_modules above the fixture.
(cd "$work" && npm init -y >/dev/null && npm install --no-audit --no-fund esbuild@0.28.2)
(cd "$broker/crates/host/tests/fixtures/sdk-bench" && node build.mjs)

cd "$broker"
cargo build --workspace --all-targets
cargo test -p ardia-plugin-host
cargo test -p ardia-plugin-integration
# The jail escape tests need a bwrap built by scripts/build-bwrap.sh; ci.yml runs the jail unit tests.
