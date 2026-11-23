#!/bin/sh
set -eu
recorded=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ardia="$recorded/../../ardia"
root="$recorded/.review-ardia"
if [ ! -f "$ardia/packages/core/package.json" ]; then
  printf '%s\n' 'Run git submodule update --init --checkout ardia from the repository root.' >&2
  exit 1
fi
rm -rf "$root"
mkdir -p "$root/packages"
cp -R "$ardia/packages/core" "$root/packages/"
cp "$recorded"/*/*-rs/*.harness.ts "$recorded"/*/*.harness.ts "$root/packages/core/src/"
