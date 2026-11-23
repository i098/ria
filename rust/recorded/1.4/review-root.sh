#!/bin/sh
set -eu
recorded=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ardia="$recorded/../../ardia"
root="$recorded/.review-ardia"
pin=$(git -C "$recorded" rev-parse HEAD:ardia)
if [ ! -e "$ardia/.git" ]; then
  printf '%s\n' 'Run git submodule update --init --checkout ardia from the repository root.' >&2
  exit 1
fi
rm -rf "$root"
mkdir -p "$root"
git -C "$ardia" archive --format=tar --output="$root/core.tar" "$pin" packages/core
tar -xf "$root/core.tar" -C "$root"
rm "$root/core.tar"
cp "$recorded"/*/*-rs/*.harness.ts "$recorded"/*/*.harness.ts "$root/packages/core/src/"
