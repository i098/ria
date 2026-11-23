#!/bin/sh
# Builds `.review-ardia/` in each crate folder of slice 1.4: the TypeScript original that
# `rust-anything fuzz` runs (the `root` of each `rust-anything.json`). Usage: review-root.sh <ardia checkout at 551d3c13c>
set -eu
ardia=${1:?usage: review-root.sh <ardia checkout>}
for dir in "$(dirname "$0")"/*-rs; do
  rm -rf "$dir/.review-ardia"
  mkdir -p "$dir/.review-ardia/packages"
  cp -R "$ardia/packages/core" "$dir/.review-ardia/packages/"
  # The harness of `bangs-harness-rs` is not part of Ardia.
  if [ -f "$dir/bangs.harness.ts" ]; then cp "$dir/bangs.harness.ts" "$dir/.review-ardia/packages/core/src/"; fi
done
