#!/bin/sh
# Builds the TypeScript oracle of slice 2.46: `packages/sync` at the commit of the Ardia gitlink.
# Usage: review-root.sh   (set ARDIA to another clone of Ardia; its HEAD is then the pin)
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ardia=${ARDIA:-"$here/../../../ardia"}
root="$here/.oracle"
if [ -n "${ARDIA:-}" ]; then
  pin=$(git -C "$ardia" rev-parse HEAD)
else
  pin=$(git -C "$here" rev-parse HEAD:ardia)
fi
if [ ! -e "$ardia/.git" ]; then
  printf '%s\n' 'Run git submodule update --init --checkout ardia from the repository root.' >&2
  exit 1
fi
rm -rf "$root"
mkdir -p "$root"
git -C "$ardia" archive --format=tar --output="$root/sync.tar" "$pin" packages/sync/package.json packages/sync/src
tar -xf "$root/sync.tar" -C "$root"
rm "$root/sync.tar"
# `native.ts` and `sharedFolders.ts` import the Expo runtime; the oracle gets an empty module.
mkdir -p "$root/packages/sync/node_modules/expo-modules-core"
printf '%s\n' '{"name":"expo-modules-core","version":"0.0.0","type":"module","main":"index.js"}' >"$root/packages/sync/node_modules/expo-modules-core/package.json"
printf '%s\n' 'export const requireOptionalNativeModule = () => null;' >"$root/packages/sync/node_modules/expo-modules-core/index.js"
for h in "$here"/*-rs/*.harness.ts "$here"/*.harness.ts; do
  [ -e "$h" ] && cp "$h" "$root/packages/sync/src/"
done
