# macOS launch

Set `ENGINE_DIR` to the directory that contains the prebuilt `Chromium Framework.framework`.
Run the commands below from `rust/` on macOS.

## Launch checks

```sh
blaze test -p ria-host-mac --lib launch::
ENGINE_DIR="$ENGINE_DIR" blaze test -p ria-app --test live
```

If Blaze reports `Too many open files`, use `cargo` in place of `blaze`.

Check N runs the native launch unit tests.
Check L makes a temporary app bundle with a new profile and a saved launch tab.
It serves the fixture page over loopback HTTP.
It checks engine start, one native window, one engine tab, the fixture page title, and engine shutdown.
It clicks the Rune pin button and requires the launch tab to be pinned.
It then relaunches on the same profile with a saved window layout and checks the restored frame (`windowFrame 1000x700`) and that the strip layer is present.
It also downloads a `file:` payload and moves it into an approved folder through `system_signals::current()`, the app's host of `setApprovedDownloadFolders` and `moveCompletedDownload`.
It requires a successful exit within 60 seconds and checks the temporary directory for crash reports.
It calls the shared launch entry point from the test executable, not from the `ria-app` binary.

## Profile and startup

By default, Ria stores data in `~/Library/Application Support/com.i098.ria` and reads configuration from `~/.config/ria`.
For an isolated profile, set `RIA_DATA_DIR` to a new directory, an empty directory, or an existing Ria profile.
An isolated profile reads configuration from its own `config/` subdirectory.
Do not point `RIA_DATA_DIR` at an Ardia profile. Ria also rejects a case-alias of the Ardia data path.
Ria checks the profile directory identity before engine startup.
For the normal profile only, Ria also checks the directory identity before it registers native messaging hosts.
Ria reports `launch profile directory changed` and does not open the first window if the final profile check fails.
See the [launch configuration module](../../../ria-host-mac/src/launch/launch_config.rs) for the Chromium handoff and its limits.
See [ADR-D1](../../../../DECISIONS.md#adr-d1-own-identity-and-own-data-ad-hoc-signature) for the keychain policy.

Ria reopens a saved launch tab only for an HTTP or HTTPS URL, a nonempty tab ID, and a usable profile.
Ria does not restore launch tabs from private or deleted profiles.
If the saved launch tab is not eligible, Ria opens `about:blank` in the default profile.
On a normal foreground launch, Ria uses the saved layout if it passes the layout checks.
The cached picture must match the content signature.
Ria removes the picture when the window changes size.
This startup path does not restore all saved windows and tabs.

Ria opens an `http` or `https` URL that another macOS app sends in a new tab of the browser window, and drops every other scheme.
`Info.plist` declares `CFBundleURLTypes` for `http` and `https`, so LaunchServices sends those URLs to Ria; Ria does not set itself as the default browser.
It queues URLs that arrive before the window is ready, and opens them in order when it is.
A click on the Dock icon, with no visible window, shows the browser window again, or opens a new one if the user closed it.
A URL that arrives while every window is closed also opens a new window.
Ria registers Apple Event handlers for both events, because the engine gives the application no delegate.
The sender of a URL is not recorded.

Ria accepts raw switches from `chromiumSwitches` in `RIA_INSTANCE` and from the command line for [Ardia parity](https://github.com/ardia-browser/ardia/blob/551d3c13c1c9ede130724ea9f1056d4b586fbe55/packages/ardiacore/ios/ArdiaHost%2BLaunch.mm#L506-L526).
Ria filters reserved engine switches from both inputs, including the profile, process type, and remote debugging switches.
Ria accepts `RIA_REMOTE_DEBUGGING_PORT` only with an isolated profile and a build without a distribution signature.
The same restriction applies to `RIA_TEST` and `RIA_TRACE`.
Set `RIA_BACKGROUND=1` to start without bringing Ria to the front.

## Run the app binary

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

## Window host

`host_launch::run` builds one `WindowHost` (`ria-host-mac/src/launch/window_host.rs`): the `ChromeWindowHost`, its shared `TabStrips`, and the `Popups` built on them.
The engine hooks for popup permissions and for popup windows that Chromium makes go to it, and `WindowHost::strip_command` routes `tabs.stripCommand` into the shared `TabStrips`.
The browser window also hosts a Rune tab strip layer around the page (`strip_layer.rs`, through `ria-surface`): one tab button and one pin button. A press on the pin sends an Arrange command through `WindowHost::strip_command`. Check L covers the pin click and the restored-layout relaunch.
`blaze test -p ria-host-mac --test live_windows` checks, through this host, that an Arrange command pins a popup tab and that moving the tab out of the popup clears the pin.
