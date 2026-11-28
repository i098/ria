#!/bin/bash
# Build Ria.app from the current checkout and publish it, zipped, as a GitHub prerelease.
# Run on an Apple silicon Mac from a checkout of `main` that has the commit pushed.
set -euo pipefail

die() { printf '%s\n' "$*" >&2; exit 1; }

command -v gh >/dev/null || die 'Install gh and run gh auth login.'
here=$(CDPATH='' cd "$(dirname "$0")" && pwd)
sha=$(git -C "$here" rev-parse HEAD)
git -C "$here" diff --quiet HEAD -- || die 'Commit or stash your changes first.'
git -C "$here" branch -r --contains "$sha" | grep -q . || die 'Push this commit first.'

out=$("$here/bundle-mac.sh")
printf '%s\n' "$out"
app=$(sed -n 's/^Bundle: //p' <<<"$out")
[[ -d "$app" ]] || die 'bundle-mac.sh printed no bundle path.'

tag="mac-${sha:0:9}"
zip="$(dirname "$app")/Ria-mac-arm64-${sha:0:9}.zip"
ditto -c -k --keepParent "$app" "$zip"
gh release create "$tag" "$zip" --prerelease --target "$sha" --title "Ria for Mac ${sha:0:9}" \
  --notes "Unsigned Ria.app (ad hoc signature) built from $sha. Apple silicon only. See the README install section."
rm -rf "$(dirname "$app")"
