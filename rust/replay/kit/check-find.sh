#!/usr/bin/env bash
set -euo pipefail

kit=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
scratch=$(realpath -- "${1:?Usage: check-find.sh ARDIA_SCRATCH}")
cases=$(mktemp -d "$scratch/find-cases.XXXXXX")
node "$kit/record-find.mjs" "$scratch" > "$cases/cases.jsonl"
count=$(wc -l < "$cases/cases.jsonl")
printf 'Recorded %s find cases in %s\n' "$count" "$cases/cases.jsonl"
if (( count < 200 )); then
  echo 'REC needs at least 200 cases.' >&2
  exit 1
fi

cp "$kit/sample/rust-anything.json" "$cases/"
sample="$cases/sample"
mkdir "$sample"
cp -R "$kit/sample/Cargo.toml" "$kit/sample/Cargo.lock" "$kit/sample/src" "$sample/"
(cd -- "$sample" && blaze build --locked)
RIA_REPLAY_BIN="$sample/target/debug/ria-replay-find" \
  rust-anything diff "$cases" --module "$kit/sample/index.cjs" --ignore-key-order
