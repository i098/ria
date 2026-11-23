#!/bin/bash
# Build abc-host, wrap it in an app bundle next to the prebuilt Ardia engine, run one proof.
# Runs on a Mac with Xcode command line tools and rustup.
#   ENGINE_DIR=<dir holding "Chromium Framework.framework"> ./run.sh a|b|c [extra abc-host args]
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
proof=${1:?usage: run.sh a|b|c}
shift || true
engine=${ENGINE_DIR:?set ENGINE_DIR to the fetched prebuilt engine}
out=${OUT_DIR:-$here/target/proof}
(cd "$here" && blaze build --release)
app="$out/Halyard Proof.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Frameworks"
cp "$here/target/release/abc-host" "$app/Contents/MacOS/abc-host"
cp "$here/Info.plist" "$app/Contents/Info.plist"
cp -cR "$engine/Chromium Framework.framework" "$app/Contents/Frameworks/"
codesign --force --sign - "$app/Contents/MacOS/abc-host" >/dev/null 2>&1
data=$(mktemp -d "${TMPDIR:-/tmp}/halyard-proof-data.XXXXXX")
exec "$app/Contents/MacOS/abc-host" --proof "$proof" --page "file://$here/src/page.html" --data-dir "$data" "$@"
