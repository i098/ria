#!/bin/bash
set -euo pipefail

die() { printf '%s\n' "$*" >&2; exit 1; }

[[ $(uname -s) == Darwin && $(uname -m) == arm64 ]] || die 'Use an Apple silicon Mac.'
command -v blaze >/dev/null || die 'Install the patched Blaze from setup.md first.'
ulimit -Sn 10240

rust=$(CDPATH='' cd "$(dirname "$0")/../.." && pwd)
fetch="$rust/../ardia/packages/ardiacore/scripts/fetch-engine.sh"
[[ -x "$fetch" ]] || die 'Run git submodule update --init --recursive from the repository root.'
framework=$("$fetch")
(cd "$rust" && CARGO_TARGET_DIR="$rust/target" blaze build -p ria-app)

scratch=$(mktemp -d "${TMPDIR:-/tmp}/ria-bundle.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
app="$scratch/Ria.app"
bin="$app/Contents/MacOS/ria-app"
mkdir -p "${bin%/*}" "$app/Contents/Frameworks"
cp "$rust/crates/ria-app/src/launch/Info.plist" "$app/Contents/Info.plist"
cp "$rust/target/debug/ria-app" "$bin"
cp -cR "$framework" "$app/Contents/Frameworks/"
codesign --force --sign - "$bin"
trap - EXIT

printf 'Bundle: %s\n' "$app"
printf 'Launch: RIA_DATA_DIR=%q %q\n' "$scratch/profile" "$bin"
