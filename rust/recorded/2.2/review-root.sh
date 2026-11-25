#!/bin/sh
# Builds the TypeScript oracle of slice 2.2: `apps/browser/src/lib` and `packages/core` of Ardia at the pinned commit.
# Usage: review-root.sh [ARDIA_CHECKOUT]   (default: the `ardia` submodule at the repository root)
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ardia=${1:-$here/../../../ardia}
root="$here/.oracle"
pin=$(git -C "$here" rev-parse HEAD:ardia)
if [ ! -e "$ardia/.git" ]; then
  printf '%s\n' 'Run git submodule update --init --checkout ardia from the repository root, or pass an Ardia checkout.' >&2
  exit 1
fi
rm -rf "$root"
mkdir -p "$root"
git -C "$ardia" archive --format=tar --output="$root/src.tar" "$pin" apps/browser/src/lib packages/core
tar -xf "$root/src.tar" -C "$root"
rm "$root/src.tar"
mkdir -p "$root/node_modules/@ardia"
ln -s ../../packages/core "$root/node_modules/@ardia/core"
