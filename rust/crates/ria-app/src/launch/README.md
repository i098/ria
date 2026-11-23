# macOS launch

Set `ENGINE_DIR` to the directory that contains the prebuilt `Chromium Framework.framework`.
Run these commands from `rust/` on macOS:

```sh
blaze test -p ria-host-mac --lib launch::
ENGINE_DIR="$ENGINE_DIR" blaze test -p ria-app --test live
```

If Blaze reports `Too many open files`, use `cargo` in place of `blaze`.

The live test makes an app bundle in a temporary directory and starts Chromium with a new profile.
It checks the engine start, one window and tab, the fixture page title, and engine shutdown.
It also checks the exit code and searches the temporary directory for crash reports.
The test runs the shared launch entry point from the test executable, not the shipped `ria-app` executable.

Startup checks the profile directory identity before native messaging registration and engine startup.
See [issue 67](https://github.com/i098/ria/issues/67) for the remaining race between the check and filesystem access.
Ria preserves both raw switch inputs for [Ardia parity](https://github.com/ardia-browser/ardia/blob/551d3c13c1c9ede130724ea9f1056d4b586fbe55/packages/ardiacore/ios/ArdiaHost%2BLaunch.mm#L506-L526).
Neither input can set the engine process type or override remote debugging.

To start the app binary from a bundle:

```sh
blaze build -p ria-app
scratch=$(mktemp -d)
app="$scratch/Ria.app/Contents"
mkdir -p "$app/MacOS" "$app/Frameworks"
cp crates/ria-app/src/launch/Info.plist "$app/Info.plist"
cp target/debug/ria-app "$app/MacOS/ria-app"
cp -cR "$ENGINE_DIR/Chromium Framework.framework" "$app/Frameworks/"
codesign --force --sign - "$app/MacOS/ria-app"
RIA_DATA_DIR="$scratch/profile" "$app/MacOS/ria-app"
```
