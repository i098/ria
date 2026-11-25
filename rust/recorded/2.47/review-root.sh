#!/bin/sh
# Builds rust/recorded/.review-ardia-import, the TypeScript oracle of slice 2.47: `packages/core`
# and `packages/import/src` of the pinned Ardia commit, the replay harness, and a stand-in for
# `expo-modules-core` whose native module is the fake that the harness sets.
# Set ARDIA to a clone of Ardia when the `ardia` submodule is not checked out.
set -eu
recorded=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
ardia=${ARDIA:-"$recorded/../../ardia"}
root="$recorded/.review-ardia-import"
pin=$(git -C "$recorded" rev-parse HEAD:ardia)
if [ ! -e "$ardia/.git" ]; then
  printf '%s\n' 'Run git submodule update --init --checkout ardia from the repository root, or set ARDIA.' >&2
  exit 1
fi
rm -rf "$root"
mkdir -p "$root/node_modules/@ardia" "$root/node_modules/expo-modules-core"
git -C "$ardia" archive --format=tar --output="$root/ardia.tar" "$pin" packages/core packages/import/src packages/import/package.json
tar -xf "$root/ardia.tar" -C "$root"
rm "$root/ardia.tar"
ln -s ../../packages/core "$root/node_modules/@ardia/core"
cp "$recorded/2.47/expo-modules-core/"* "$root/node_modules/expo-modules-core/"
cp "$recorded/2.47/import.harness.ts" "$root/packages/import/src/"
