#!/bin/bash
# Build blaze at a named commit with this directory's patches (all *.patch, in name order)
# and install it as ~/.cargo/bin/blaze.
# usage: install.sh [WORK_DIR]   (default /tmp/ria-blaze; the build output stays in it)
# Needs git, curl, python3 and a blaze toolchain in ~/.blaze/toolchains (any `blaze` run fetches it).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
work=${1:-/tmp/ria-blaze}
commit=f5852696ea0a3c2c865ed2cdb4379e41900f279b
bin=${CARGO_HOME:-$HOME/.cargo}/bin

tc=$(ls -d "$HOME"/.blaze/toolchains/nightly-* 2>/dev/null | tail -1)
[ -n "$tc" ] || { echo "no toolchain in ~/.blaze/toolchains; run any blaze command once" >&2; exit 1; }
triple=$("$tc/bin/rustc" -vV | sed -n 's/^host: //p')

mkdir -p "$work"
if [ ! -d "$work/blaze/.git" ]; then
  git clone -q https://github.com/indexable-inc/blaze "$work/blaze"
fi
git -C "$work/blaze" checkout -q -f "$commit"
git -C "$work/blaze" clean -qfd
for p in "$here"/*.patch; do git -C "$work/blaze" apply "$p"; done

# The LLVM that blaze pins (crates/hir-ty-codegen/llvm.json); `blaze build --release` needs a blaze that has it.
# llvm-sys's build script may name only a system directory, so a prefix below /tmp is used here and
# the build script of llvm-sys is the one that reads it; `.github/scripts/llvm.py fetch` does the same
# but needs a Python 3.12 tar, which the Mac's system python lacks.
llvm="$work/llvm"
if [ ! -f "$llvm/blaze-llvm.json" ]; then
  read -r url sha < <(python3 - "$work/blaze/crates/hir-ty-codegen/llvm.json" "$triple" <<'PY'
import json, sys
p = json.load(open(sys.argv[1])); t = p["targets"][sys.argv[2]]
print(p["release"] + "/" + t["name"], t["sha256"])
PY
)
  curl -fsSL -o "$work/llvm.tar.xz" "$url"
  echo "$sha  $work/llvm.tar.xz" | shasum -a 256 -c -
  mkdir -p "$work/llvm-x" && tar xf "$work/llvm.tar.xz" -C "$work/llvm-x"
  mv "$work/llvm-x"/* "$llvm" && rmdir "$work/llvm-x" && rm "$work/llvm.tar.xz"
fi

(cd "$work/blaze" && LLVM_SYS_231_PREFIX=$llvm CARGO_TARGET_DIR=$work/target RUSTC=$tc/bin/rustc \
  "$tc/bin/cargo" build --release -p blaze-engine --features llvm)

mkdir -p "$bin"
for b in blaze blaze-engine blaze-rustc; do
  install -m 755 "$work/target/release/$b" "$bin/$b.new" && mv -f "$bin/$b.new" "$bin/$b"
done
"$bin/blaze" --version
