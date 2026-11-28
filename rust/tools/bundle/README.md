# Build a local Mac test bundle

Finish [Setup: Install](../../../setup.md#install) on an Apple silicon Mac with Xcode.
Run this command from the repository root:

```sh
rust/tools/bundle/bundle-mac.sh
```

The command fetches the pinned Chromium engine and builds `ria-app` with Blaze.
It puts `Ria.app` in a new directory under `${TMPDIR:-/tmp}`.
It does not install or replace an application.

Copy the command after `Launch:` into the same shell to start Ria with a fresh profile beside the bundle.
Use that command to keep the test profile separate from the normal Ria profile.
Add `RIA_BACKGROUND=1` before `RIA_DATA_DIR=...` to start Ria without bringing it to the front.
Remove the new directory after you quit Ria if you no longer need the bundle or profile.

## Publish a prerelease

From a pushed commit of `main` on the same Mac, with `gh` signed in:

```sh
rust/tools/bundle/release-mac.sh
```

It builds the bundle, zips it with `ditto` as `Ria-mac-arm64-<commit>.zip`, and creates the prerelease `mac-<commit>` with that asset.
GitHub Actions cannot do this: the build needs the patched Blaze ([setup](../../../setup.md)).

## Signing limits

The script ad hoc signs the Ria executable and keeps the engine framework as supplied.
The bundle is for local tests only. It has no Developer ID signature or notarization for Gatekeeper distribution.
Do not use this profile for real passwords or cookies. Ria uses a [mock keychain](../../DECISIONS.md#adr-d1-own-identity-and-own-data-ad-hoc-signature) with a fixed key.

If Blaze reports `Too many open files`, follow the [daemon recovery steps](../../../setup.md#macos-open-file-limit), then run the bundle command again.
