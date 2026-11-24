#!/usr/bin/env bash
set -euo pipefail

kit=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
source=${1:-"$kit/../../../ardia"}
parent=${2:-${TMPDIR:-/tmp}}
commit=551d3c13c1c9ede130724ea9f1056d4b586fbe55

# The source must be the Ardia checkout, not its enclosing Ria repository.
if [[ $(git -C "$source" rev-parse --show-toplevel) != "$(realpath -- "$source")" ]]; then
  echo 'Pass an initialized Ardia checkout to setup.sh.' >&2
  exit 1
fi
scratch=$(mktemp -d "$parent/ria-replay.XXXXXX")
printf 'Installing Ardia in %s\n' "$scratch" >&2
git -C "$source" archive "$commit" | tar -x -C "$scratch"
(cd -- "$scratch" && pnpm install --frozen-lockfile --ignore-scripts --store-dir "$scratch/.pnpm-store") >&2
printf '%s\n' "$scratch"
