Source: Ardia at frozen commit `551d3c13c`. Paths are relative to the Ardia repo root.

# Ardia on-disk formats (macOS app)

Scope: the Mac app (`apps/browser/src`, `apps/browser/macos`, `packages/*/ios` except `packages/phone`). All paths cite the Ardia repo at commit 551d3c13c. A note marked [INFERENCE] is not verified in source.

## 1. Where the data lives

Ardia uses four roots. Ria must keep all four.
The port keeps the 4 formats in its own data folder and never opens an Ardia profile folder (ADR-D1).

| Root | Default macOS path | With `ARDIA_DATA_DIR=<D>` | Resolved at |
|---|---|---|---|
| App data dir ("documents") | `~/Library/Application Support/<bundle id>/` (bundle id `com.ardia.browser`) | `<D>/` | `packages/shell/ios/DocumentStore.swift:24-33`; bundle id `apps/browser/macos/Ardia.xcodeproj/project.pbxproj:408` |
| Chromium user data dir | `<app data dir>/Chromium/` | `<D>/Chromium/` | `packages/ardiacore/ios/ArdiaHost+Launch.mm:411`, `packages/ardiacore/ios/ArdiaProfiles.mm:67-70` |
| Config folder (user-edited) | `~/.config/ardia/` | `<D>/config/` | `packages/ardiacore/ios/ArdiaLaunchConfig.mm:204-206` |
| Plugin programs | `~/Library/Application Support/Ardia/Programs` + links in `~/.local/bin` | `<app data dir>/Programs` (needs `cliBinDir`) | `packages/shell/ios/PluginFolders.swift:35-47` |

Notes:

- `ARDIA_DATA_DIR` is read in `packages/ardiacore/ios/ArdiaLaunchConfig.mm:8` and `:204`. It makes a "scratch" test instance.
- The native host computes the app data dir again as `AppDataDirectory()`: the parent of the Chromium dir for a scratch instance, otherwise `Application Support/<bundle id or "Ardia">` (`packages/ardiacore/ios/ArdiaHost+Launch.mm:124-128`). This matches `DocumentStore`.
- The Chromium dir goes to the engine as `--user-data-dir=` (`packages/ardiacore/ios/ArdiaHost+Launch.mm:470`). `ArdiaHost.dataDirectory` returns this dir (`packages/ardiacore/ios/ArdiaHost.mm:462-464`).
- The plugin `Programs` folder uses the literal name `Ardia`, not the bundle id (`packages/shell/ios/PluginFolders.swift:43`). This is a separate folder from the app data dir.
- `DocumentStore.url` strips any folder part from a name (`lastPathComponent`, `packages/shell/ios/DocumentStore.swift:33`). So every JS "document" is a flat file in the app data dir.

### Profile layout

| Profile | Chromium sub-folder | Cite |
|---|---|---|
| Default (empty name) | `Chromium/Default/` | `packages/ardiacore/ios/ArdiaProfiles.mm:102-103` |
| Named profile `<name>` | `Chromium/Profile <name>/` | `packages/ardiacore/ios/ArdiaProfiles.mm:102-103` |
| Private (incognito) profile | Nothing on disk for Ardia files: favicons, tab history and the navigation-download list skip private profiles | `packages/ardiacore/ios/ArdiaFavicons.mm:33`, `packages/ardiacore/ios/WebKitTab+Lifecycle.swift:42-44`, `packages/ardiacore/ios/ArdiaNavigationDownloads.mm:32` |

The engine loads a profile from `user_data_dir().Append(directoryName)` (`engine/ardiacore/src/ardia/core/ardiacore_engine.mm:188`). Profile state for Ardia's own UI (windows, tabs, pins per profile) lives in `session.json`, keyed by profile id, not in the profile folder.

### Atomic-write rules

| Method | What it does | Cite |
|---|---|---|
| `DocumentStore.write` (JS `writeDocument` / `saveDocument`) | Stages the text in memory, then a utility queue calls `String.write(to:atomically:true)` (temp file + rename). A failure retries with back-off up to 60 s. `readDocument` returns the staged text first, so JS reads its own unsaved writes. Staged writes flush on `willTerminate`. | `packages/shell/ios/DocumentStore.swift:36-59`, `:85-107`, `:20-22`, `:115-131` |
| `saveDocument` / `documentSaved` | Same path, but the JS promise resolves when the file lands. | `packages/shell/ios/DocumentsModule.swift:9-19` |
| `removeDocument` | Drops the staged write and deletes the file on the queue. | `packages/shell/ios/DocumentStore.swift:72-83` |
| Versioned JSON | `versionedJson` writes `{...data, version: N}`. `parseSaved` runs migrations from the saved version up to N. It returns `null` for a newer or unknown version. | `apps/browser/src/lib/savedDocument.ts:20-40` |
| Native `writeToFile:atomically:YES` / Swift `.atomic` | Foundation temp + rename. | many, see each row |
| `MigrationFiles.writeDurably` | `O_EXCL` temp `.<name>.<pid>.tmp`, `fchmod`, write, optional `F_FULLFSYNC`, `rename`. | `packages/sync/ios/Core/MigrationFiles.swift:4-13` |
| Plugin storage / sandbox | `openat` temp with `O_EXCL|O_NOFOLLOW`, then `renameat`. | `packages/shell/ios/PluginCore/PluginStorage.swift:48-61`, `packages/shell/ios/PluginCore/PluginSandbox.swift:260-273` |
| Config schemas | `open(O_EXCL)` temp `.<name>.<pid>.<uuid>.tmp`, write, `rename`. | `packages/shell/ios/ConfigFolder.swift:79-91` |

## 2. Overview table

"App" = app data dir. "Chromium" = Chromium user data dir. "Config" = config folder. "Prof" = `Chromium/<profile dir>/`.

| Path | Format | Version | Atomic | Writer | Reader |
|---|---|---|---|---|---|
| App/`session.json` | JSON, saved browser state | `version` 11 | DocumentStore | `apps/browser/src/state/session/persist.ts:196` | `apps/browser/src/state/session/persist.ts:187`; native `packages/ardiacore/ios/ArdiaSavedSession.mm:27` |
| App/`downloads.json` | JSON `{downloads:[...]}` | `version` 3 | DocumentStore | `apps/browser/src/state/session/persist.ts:196` | `apps/browser/src/state/session/persist.ts:187` |
| App/`pins-backup.json`, `pins-backup-<ms>-<n>.json` | JSON `{version, backups:[{at,dropped,pins}]}` | 1 | DocumentStore (`saveDocument`) | `apps/browser/src/state/session/pinGuard.ts:138` | `apps/browser/src/state/session/pinGuard.ts:108` |
| App/`launch-look.json` | JSON `{themes, favicons}` | 2 | DocumentStore | `apps/browser/src/state/windows/launchLook.ts:47` | `apps/browser/src/state/windows/launchLook.ts:55` |
| App/`launch-layout.json` | JSON `{version, windows:{<id>:{...}}}` | 5 (native only) | `writeToFile:atomically:YES` | `packages/ardiacore/ios/ArdiaEarlyWindowLayouts.mm:240`, `:191` | `packages/ardiacore/ios/ArdiaEarlyWindowLayouts.mm:173-178` |
| App/`launch-picture-<windowId>.png` | PNG | none | `writeToFile:atomically:YES` | `packages/ardiacore/ios/ArdiaLaunchPicture.mm:271` | `packages/ardiacore/ios/ArdiaEarlyWindow.mm:175` |
| App/`live.json` | JSON | 2 | DocumentStore | `apps/browser/src/platform/livePersistence.ts:40` | `apps/browser/src/platform/livePersistence.ts:45` |
| App/`onboarding.json` | JSON `Saved` | 1 | DocumentStore | `apps/browser/src/state/onboarding/onboardingDocument.ts:32`, `:53` | `apps/browser/src/state/onboarding/onboardingDocument.ts:24` |
| App/`default-browser-offer.json` | JSON | 1 | DocumentStore / native | `apps/browser/src/state/onboarding/defaultBrowserOffer.ts:29`; `packages/sync/ios/Core/LegacyMigrator.swift:438` | `apps/browser/src/state/onboarding/defaultBrowserOffer.ts:24` |
| App/`release-notes.json` | JSON `{lastVersion, pending}` | 1 | DocumentStore | `apps/browser/src/state/releaseNotes/releaseNotes.ts:50` | `apps/browser/src/state/releaseNotes/releaseNotes.ts:41` |
| App/`air-traffic.json` | JSON link-routing rules | 2 | DocumentStore | `apps/browser/src/state/airTraffic/store.ts:37` | `apps/browser/src/state/airTraffic/store.ts:29`; stamp `packages/ardiacore/ios/ArdiaHost+Launch.mm:66` |
| App/`link-sources.json` | JSON recent source apps | 1 | DocumentStore | `apps/browser/src/state/airTraffic/store.ts:37` | `apps/browser/src/state/airTraffic/store.ts:29` |
| App/`automation-approvals.json` | JSON `{approved, dismissed}` | 1 | DocumentStore | `apps/browser/src/state/automation/approvals.ts:43` | `apps/browser/src/state/automation/approvals.ts:30`; stamp `packages/ardiacore/ios/ArdiaHost+Launch.mm:65` |
| App/`chrome-ctrl-tab.json` | JSON `{profiles}` | 1 | DocumentStore | `apps/browser/src/state/commands/chromeCtrlTab.ts:43` | `apps/browser/src/state/commands/chromeCtrlTab.ts:32` |
| App/`site-data-carry.json` | JSON `{profiles}` | 1 | DocumentStore | `apps/browser/src/state/engines/siteDataState.ts:37` | `apps/browser/src/state/engines/siteDataState.ts:25` |
| App/`bookmarks-cache.json` | JSON `{trees}` | 1 | DocumentStore | `apps/browser/src/state/bookmarks/engineSync.ts:230` | `apps/browser/src/state/bookmarks/engineSync.ts:180` |
| App/`bangs.txt` | text bang list (with etag) | none | DocumentStore (`saveDocument`) | `apps/browser/src/state/omnibox/bangs.ts:162` | `apps/browser/src/state/omnibox/bangs.ts:47` |
| App/`bangs-state.json` | JSON `{checkedAt}` | 1 | DocumentStore | `apps/browser/src/state/omnibox/bangs.ts:41` | `apps/browser/src/state/omnibox/bangs.ts:32` |
| App/`autofill-toggles-asked.json` | JSON `{profiles}` | 1 | DocumentStore | `apps/browser/src/state/settings/autofillAsked.ts:39` | `apps/browser/src/state/settings/autofillAsked.ts:14` |
| App/`password-manager.json` | JSON held-off profiles | 1 | DocumentStore (`saveDocument`) | `apps/browser/src/state/site/offerToSave.ts:93` | `apps/browser/src/state/site/offerToSave.ts:23`, `:98` |
| App/`password-panel-hints.json` | JSON `{version:1, dismissals}` | 1 (hand-written) | DocumentStore (`saveDocument`) | `apps/browser/src/state/site/panelHints.ts:49` | `apps/browser/src/state/site/panelHints.ts:18` |
| App/`report-draft.json` | JSON `SavedDraft` | 1 | DocumentStore | `apps/browser/src/state/report/actions.ts:51` (remove `:52`) | `apps/browser/src/state/report/actions.ts:61` |
| App/`Report Drafts/<uuid>/` | report attachments (deflated) | none | `FileHandle` stream | `packages/shell/ios/ReportFiles.swift:9-14`, `:49` | `packages/shell/ios/ReportFiles.swift:22` (removes) |
| App/`shared-folders.json` | JSON `{folders}` | 1 | DocumentStore | `apps/browser/src/state/sharedFolders/store.ts:66` | `apps/browser/src/state/sharedFolders/store.ts:44` |
| App/`switches-cache.json` | JSON `{version:1, switches}` | 1 | DocumentStore | `apps/browser/src/platform/killSwitches.ts:163` | `apps/browser/src/platform/killSwitches.ts:127` |
| App/`plugins.json` | JSON plugin records | 1 | DocumentStore | `apps/browser/src/state/plugins/savedState.ts:66` | `apps/browser/src/state/plugins/savedState.ts:58`; native uses it as a marker `packages/shell/ios/PluginFolders.swift:18-19` |
| App/`PluginData/<plugin>/storage.json` | JSON, max 2 MB | none | `openat` temp + `renameat` | `packages/shell/ios/PluginCore/PluginStorage.swift:41-61` | `packages/shell/ios/PluginCore/PluginStorage.swift:22-30` |
| App/`PluginData/<plugin>/plugin.log` (+`.1`) | text log, rotated | none | `O_APPEND` | `packages/shell/ios/PluginCore/PluginFiles.swift:200-203` | `packages/shell/ios/PluginsModule.swift:171` (reveal) |
| App/`PluginData/<plugin>/{home,tmp,certificates.pem,extra-certificates.pem}` | plugin sandbox | none | `renameat` | `packages/shell/ios/PluginCore/PluginSandbox.swift:222-237`, `:260-273` | `packages/shell/ios/PluginCore/PluginSandbox.swift:222-224` |
| App/`OfficialPlugins/` | official plugin archives + `.<name>.lock` | none | `O_EXCL` + move | `packages/shell/ios/PluginCore/OfficialStore.swift:29`, `:84`; `packages/shell/ios/OfficialPluginFetch.swift:148` | `packages/shell/ios/PluginFolders.swift:33` |
| App/`after-update-quit-stops.json` | JSON | none | `.atomic` | `packages/shell/ios/PluginProgramLinks.swift:285-290` | `packages/shell/ios/PluginProgramLinks.swift:302` |
| App/`Scripts/` (`generation`, `extension/`, `.lock`, `status.json`) | user scripts as an MV3 extension | `STATUS_VERSION` 1 | `status.json` `.atomic` | `packages/shell/ios/UserScripts.swift:148` | `packages/shell/ios/UserScripts.swift:74-76`, `:142` |
| App/`config-schema/{config,theme}.schema.json` | JSON Schema, regenerated each launch | none | temp + rename | `packages/shell/ios/ConfigFolder.swift:63-91` (from `apps/browser/src/state/config/configFiles.ts:254`) | editors only [INFERENCE] |
| App/`sync.json` | JSON `SyncDoc` | `v` 1 | DocumentStore | `apps/browser/src/state/sync/syncFiles.ts:265` | `apps/browser/src/state/sync/syncFiles.ts:115` |
| App/`sync-state.nns` | sealed JSON checkpoint | inner `version` 1 | DocumentStore (`saveDocument`) | `apps/browser/src/state/sync/syncFiles.ts:294` (empty `:278`) | `apps/browser/src/state/sync/syncFiles.ts:146` |
| App/`sync-journal-<n>.nns` | sealed JSON journal part | inner `version` 1 | DocumentStore (`saveDocument`) | `apps/browser/src/state/sync/syncFiles.ts:193` (remove `:242`) | `apps/browser/src/state/sync/syncFiles.ts:135` |
| App/`sync-journal.nns` (legacy) | sealed journal | — | DocumentStore | emptied `apps/browser/src/state/sync/syncFiles.ts:243` | `apps/browser/src/state/sync/syncFiles.ts:129` |
| App/`sync-extensions.json` | JSON `{generation, profiles}` | 1 | DocumentStore | `apps/browser/src/state/sync/extensions.ts:30` | `apps/browser/src/state/sync/extensions.ts:22` |
| App/`telemetry.json` | JSON sharing choice | 1 | DocumentStore | `apps/browser/src/state/telemetry/choice.ts:155`, `:167` | `apps/browser/src/state/telemetry/choice.ts:181` |
| App/`telemetry-queue.json` | JSON `{events, logs}` | 1 | DocumentStore | `apps/browser/src/state/telemetry/client.ts:75`, `:91` | `apps/browser/src/state/telemetry/client.ts:47` |
| App/`telemetry-daily.json` | JSON `{lastDay, pending}` | 1 | DocumentStore (`saveDocument`) | `apps/browser/src/state/telemetry/daily.ts:78` | `apps/browser/src/state/telemetry/daily.ts:62` |
| App/`telemetry-requests.log` | text, last 400 kB | none | DocumentStore (rewrite whole file) | `apps/browser/src/state/telemetry/requestLog.ts:25` | `apps/browser/src/state/telemetry/requestLog.ts:25` |
| App/`freeze-sends.json` | JSON `{day, sent, signatures}` | 1 | DocumentStore | `apps/browser/src/state/telemetry/freezeSends.ts:28` | `apps/browser/src/state/telemetry/freezeSends.ts:15` |
| App/`freezes.txt` | ring buffer: 20 slots × 4096 bytes | none | `pwrite` in place (not atomic) | `packages/ardiacore/ios/ArdiaFreezeCapture.mm:251`, `:254-265` | `apps/browser/src/state/telemetry/freezeRecords.ts:53` |
| App/`Telemetry/local/YYYY-MM-DD.jsonl` | JSON lines | none | `O_APPEND` | `packages/shell/ios/TelemetryLocalStore.swift:57-71` | `packages/shell/ios/TelemetryLocalStore.swift:228-253` |
| App/`hermes-fatal.txt`, `hermes-fatal-last.txt` | one text line | none | raw `write(2)` in a fatal handler | `packages/shell/ios/ArdiaHermesFatal.mm:30` | `packages/shell/ios/HermesFatal.swift:14-19` |
| App/`launch-guard.json` | JSON facts | — | — | no Mac writer found (only `scripts/windows/engine/src/ardia/win/main.cc`) | `apps/browser/src/state/telemetry/diagnostics.ts:381` |
| App/`ArdiaControl.sock`, `ArdiaControl.sock.lock` | Unix socket + `flock` file | — | `unlink` + `bind` | `packages/shell/ios/ControlSocket.swift:101-124` | CLI [INFERENCE] |
| App/`.legacy-migration.json` (+ `.lock`, `-staging`, `-replaced`) | JSON `Journal` | `version` 1 | `writeDurably` | `packages/sync/ios/Core/LegacyMigrator.swift:450-455` | `packages/sync/ios/Core/LegacyMigrator.swift:529` |
| App/`WebKit Content Rules/` (`settings.json`, `user-filters.json`, `user-filters-rejected.json`, `Store/`) | JSON + WebKit rule store | none | `.atomic` | `packages/ardiacore/ios/WebKitContentBlocker.swift:252`, `:288`, `:329` | `packages/ardiacore/ios/WebKitContentBlocker.swift:180-194` |
| Chromium/`WebKit Tab History/p<hex profile>/<hex key>` | JSON `{version, url, state(base64), epoch}` | 1 | `.atomic` | `packages/ardiacore/ios/WebKitTab+Lifecycle.swift:155-159` | `packages/ardiacore/ios/WebKitTab+Lifecycle.swift:105` |
| Chromium/`WebKit Tab History/p<hex>.<kind>` | marker file (UUID text) | — | `.atomic` | `packages/ardiacore/ios/WebKitTab+Lifecycle.swift:91` | `packages/ardiacore/ios/WebKitTab+Lifecycle.swift:71-72` |
| Chromium/`WebKit Transport Security/p<hex>.json` | JSON `[host: WebKitTransportSecurity]` | none | `.atomic` | `packages/ardiacore/ios/WebKitTab+Certificate.swift:165` | `packages/ardiacore/ios/WebKitTab+Certificate.swift:150-151` |
| Chromium/`NavigationDownloads.json` | JSON `{url: epoch seconds}`, max 200 | none | `writeToFile:atomically:YES` | `packages/ardiacore/ios/ArdiaNavigationDownloads.mm:44` | `packages/ardiacore/ios/ArdiaNavigationDownloads.mm:17` |
| Chromium/`NativeMessagingHosts/*.json` + `.ardia-managed.json` | host manifests + JSON array of names | none | `NSDataWritingAtomic` | `packages/ardiacore/ios/ArdiaNativeMessaging.mm:114-124` | `packages/ardiacore/ios/ArdiaNativeMessaging.mm:104-106` |
| Chromium/`.ardia-website-languages-kept` | empty marker | — | atomic | `packages/ardiacore/ios/ArdiaStartup.mm:42` | `packages/ardiacore/ios/ArdiaStartup.mm:25-26` |
| Prof/`Preferences` (Chromium file) | Chromium JSON; Ardia edits `intl.selected_languages` once | Chromium | atomic | `packages/ardiacore/ios/ArdiaStartup.mm:40` | `packages/ardiacore/ios/ArdiaStartup.mm:28-35` |
| Prof/`Ardia Favicons/<name>.png` | PNG | none | `writeToFile:atomically:YES` | `packages/ardiacore/ios/ArdiaFavicons.mm:34-37` | JS loads the `file://` URI (`packages/ardiacore/ios/ArdiaFavicons.mm:38`) |
| Prof/`uBlock Origin Lite settings moved` | text summary marker | — | atomic | `packages/ardiacore/ios/ArdiaContentBlockerLite.mm:74-76` | `packages/ardiacore/ios/ArdiaContentBlockerLite.mm:64-65` |
| Prof/`Extensions/<id>/<version>_*` | unpacked extension (Chromium layout) | Chromium | — | `packages/ardiacore/ios/ArdiaServices.mm:114-117` | engine |
| Config/`config.json`, `hooks.json`, `themes/*` | user-written JSON | — | user | not written by the app (see §3.9) | `apps/browser/src/state/config/configFiles.ts:234`, `apps/browser/src/state/automation/hookRules.ts:80` |
| `Ardia Sync/<chain>/<scope>/<id>.nns` (iCloud Drive, or App/ in a test instance) | AES-GCM sealed payload | `v:1` in payload | staging + coordinated move | `packages/sync/ios/Core/SyncVault.swift:36-47` | `packages/sync/ios/Core/SyncVault.swift:98-136` |
| `Ardia Shared/**/.ardia-folder.json` (iCloud Drive or App/) | JSON manifest | 1 | staging + `renameatx_np(RENAME_EXCL)` | `packages/sync/ios/Core/SharedFolderDirectory.swift:534-536` | `packages/sync/ios/Core/SharedFolderDirectory.swift:525` |
| `~/Library/Caches/<bundle id>/ImportIcons/<id>-<mtime>.png` | PNG | none | `.atomic` | `packages/import/ios/Core/AppIcons.swift:33` | `packages/import/ios/ImportModule.swift:15` |

## 3. One section per file

### 3.1 `session.json`

- Path: App/`session.json` (`apps/browser/src/state/session/sessionDocument.ts:3`).
- Format: JSON object. Top-level fields come from `SESSION_FIELDS` (`apps/browser/src/state/session/persist.ts:64-102`): `profiles`, `profileOrder`, `orphanedProfileData`, `settings`, `settingsMeta`, `windows`, `windowOrder`, `focusedWindowId`, `lastProfileId`, `launchTab`, `launchBackdrop`, `linkProfile`, `tabs`, `groups`, `splits`, `closedTabs`, `closedWindows`, `parkedPins`, `pinChoices`, `pinSync`, `topAppsMerged`, `closedGroups`, `deletedGroups`, `cleanedTabs`.
- Incognito data is filtered out before save (`apps/browser/src/state/session/persist.ts:51-52`).
- Version: `version` field, `SESSION_VERSION = 11` (`apps/browser/src/state/session/persist.ts:35`). Native also pins 11 (`packages/ardiacore/ios/ArdiaSavedSession.mm:22-24`) and rejects anything above (`packages/ardiacore/ios/ArdiaSavedDocument.mm:15-20`).
- Atomic: DocumentStore. Save loop with debounce (`apps/browser/src/state/session/persist.ts:370-399`). Quit waits on `documentSaved` (`apps/browser/src/state/session/persist.ts:283`).
- Writer: `apps/browser/src/state/session/persist.ts:196`. Field list registered at `:111`.
- Readers: JS `apps/browser/src/state/session/persist.ts:187`, `apps/browser/src/state/session/sessionDocument.ts:5`. Native, before JS starts: `packages/ardiacore/ios/ArdiaHost+Launch.mm:161`, `:225`, `:247`, `:308`, `:485`, and `packages/ardiacore/ios/ArdiaEarlyWindowLayouts.mm:253` (early window and link profile).

### 3.2 `downloads.json`

- Shape: `{ downloads: [...] }`. Migration 2→3 drops `stagingId` and `destination` from each row (`apps/browser/src/state/session/persist.ts:36-46`).
- Version: `version` 3, migrations `{0,1,2}` (`apps/browser/src/state/session/persist.ts:47-49`).
- Writer / reader: same `write` / `read` as session (`apps/browser/src/state/session/persist.ts:196`, `:187`), via `DOCS` (`:111`).

### 3.3 Pins backup

- `pins-backup.json`: `{version, backups:[{at, dropped:{id:n}, pins:{profile: ParkedPins}}]}` (`apps/browser/src/state/session/pinGuard.ts:17-18`, `:118`). Version 1 (`:12`).
- If the main file does not parse, Ardia writes a new side file `pins-backup-<Date.now()>-<n>.json` and leaves the bad file alone (`apps/browser/src/state/session/pinGuard.ts:105-113`).
- Writer: `apps/browser/src/state/session/pinGuard.ts:138` (`saveDocument`). Reader: `:108`.

### 3.4 Launch look, layout and pictures (fast launch window)

- `launch-look.json` (JS): `{themes:[{tab,page,color}], favicons, version:2}` (`apps/browser/src/state/windows/launchLook.ts:21-22`, `:13`). If too large, favicons are emptied (`:47`). Writer `:47`, reader `:55`.
- `launch-layout.json` (native only): `{version:5, windows:{<windowId>: frame/display/key/profile/backdrop}}` (`packages/ardiacore/ios/ArdiaEarlyWindowLayouts.mm:23`, `:240`). Path set at `packages/ardiacore/ios/ArdiaEarlyWindow.mm:128`. Written on a serial queue with `writeToFile:atomically:YES` (`packages/ardiacore/ios/ArdiaEarlyWindowLayouts.mm:186-194`). Read with `ArdiaSavedDocument parse:oldest:newest:` (`:173-176`); any other version reads as nothing (`docs/conventions.md:219-220`).
- `launch-picture-<windowId>.png`: name from `packages/ardiacore/ios/ArdiaLaunchPicture.mm:21-27`, written `:271`, deleted `:277-278`, read `packages/ardiacore/ios/ArdiaEarlyWindow.mm:175`.
- Folder for all three: `AppDataDirectory()` (`packages/ardiacore/ios/ArdiaHost+Launch.mm:484`).

### 3.5 Small versioned JS state files

All use DocumentStore and `versionedJson` / `parseSaved` with field `version` unless noted. Version constants are in the table in §2; cites for the constants:

| File | Version const | Shape cite |
|---|---|---|
| `live.json` | `apps/browser/src/platform/livePersistence.ts:5` (2, migrations `:6`) | `apps/browser/src/platform/livePersistence.ts:15-17` |
| `onboarding.json` | `apps/browser/src/state/onboarding/onboardingDocument.ts:7` | `apps/browser/src/state/onboarding/onboardingDocument.ts:9` |
| `default-browser-offer.json` | `apps/browser/src/state/onboarding/defaultBrowserOffer.ts:5` | `apps/browser/src/state/onboarding/defaultBrowserOffer.ts:29` (`askedAt`) |
| `release-notes.json` | `apps/browser/src/state/releaseNotes/releaseNotes.ts:36` | `apps/browser/src/state/releaseNotes/releaseNotes.ts:38` |
| `air-traffic.json` | `apps/browser/src/state/airTraffic/rules.ts:9` (2) | migrations `apps/browser/src/state/airTraffic/rules.ts:11` |
| `link-sources.json` | `apps/browser/src/state/airTraffic/sourceApps.ts:6` | — |
| `automation-approvals.json` | `apps/browser/src/state/automation/approvals.ts:12` | `apps/browser/src/state/automation/approvals.ts:43` |
| `chrome-ctrl-tab.json` | `apps/browser/src/state/commands/chromeCtrlTab.ts:13` | `:43` |
| `site-data-carry.json` | `apps/browser/src/state/engines/siteDataState.ts:13` | `:37` |
| `bookmarks-cache.json` | `apps/browser/src/state/bookmarks/engineSync.ts:164` | `:230` |
| `bangs-state.json` | `apps/browser/src/state/omnibox/bangs.ts:9` | `apps/browser/src/state/omnibox/bangs.ts:13` |
| `autofill-toggles-asked.json` | `apps/browser/src/state/settings/autofillAsked.ts:7` | `:39` |
| `password-manager.json` | `apps/browser/src/state/site/offerToSave.ts:14` | `apps/browser/src/state/site/offerToSave.ts:17` |
| `report-draft.json` | `apps/browser/src/state/report/savedDraft.ts:9` | `apps/browser/src/state/report/savedDraft.ts:16` |
| `shared-folders.json` | `apps/browser/src/state/sharedFolders/store.ts:23` | `:66` |
| `sync-extensions.json` | `apps/browser/src/state/sync/extensions.ts:12` | `apps/browser/src/state/sync/extensions.ts:15` |
| `plugins.json` | `apps/browser/src/state/plugins/savedState.ts:11` | written by `pluginRecordsJson` (`:66`) |
| `telemetry.json` | `apps/browser/src/state/telemetry/choice.ts:11` | `apps/browser/src/state/telemetry/choice.ts:117` [INFERENCE: `Shown` is the saved shape] |
| `telemetry-queue.json` | `apps/browser/src/state/telemetry/client.ts:22` | `apps/browser/src/state/telemetry/client.ts:33` |
| `telemetry-daily.json` | `apps/browser/src/state/telemetry/daily.ts:10` | `apps/browser/src/state/telemetry/daily.ts:53` |
| `freeze-sends.json` | `apps/browser/src/state/telemetry/freezeSends.ts:6` | `apps/browser/src/state/telemetry/freezeSends.ts:28` |

Hand-versioned (no `versionedJson`): `switches-cache.json` writes `{version:1, switches}` (`apps/browser/src/platform/killSwitches.ts:163`); `password-panel-hints.json` writes `{version:1, dismissals}` (`apps/browser/src/state/site/panelHints.ts:49`). `bangs.txt` is text from `bangDocument(etag, list)` (`apps/browser/src/state/omnibox/bangs.ts:162`); its layout is not checked here [INFERENCE: etag header + list].

### 3.6 Sync files

- `sync.json`: `SyncDoc` with `v: 1`, `enabled`, `folder`, `deviceId`, `clock`, `types`, `profiles`, `scopes`, `lastSyncedAt`, `setupAt`, `joining`, `seqs`, `sends`, `tabCloses`, `removing`, `removingDevices` (`apps/browser/src/state/sync/syncDoc.ts:47-64`, consts `:68-70`). Written with field `v` (`apps/browser/src/state/sync/syncFiles.ts:259`).
- `sync-state.nns` and `sync-journal-<n>.nns`: JSON with inner `version` 1 (`apps/browser/src/state/sync/syncFiles.ts:15`, `:17`), sealed by native `sealLocal` with scope tag `local` (`packages/sync/ios/SyncModule.swift:132-140`). Part names: `apps/browser/src/state/sync/syncFiles.ts:50`.
- Sync folder: `Ardia Sync/` in iCloud Drive, or in the app data dir for a test instance (`packages/sync/ios/SyncModule.swift:346-351`). Layout `<chainTag>/<scopeTag>/<id>.nns` (`packages/sync/ios/Core/SyncVault.swift:5`, `:17-18`, `:40`). Write: seal to a staging dir, then `NSFileCoordinator` move (`:41-47`).
- Shared folders: `Ardia Shared/` (`packages/sync/ios/SharedFoldersModule.swift:62-63`); manifest `.ardia-folder.json` (`packages/sync/ios/Core/SharedFolderDirectory.swift:4`), version 1 in JS (`apps/browser/src/state/sharedFolders/tree.ts:24`).

### 3.7 Telemetry and crash files

- `Telemetry/local/` sits beside `telemetry.json` (`packages/shell/ios/TelemetryLocalStore.swift:34-39`). App files match `YYYY-MM-DD.jsonl`; CLI files match `cli-YYYY-MM-DD.jsonl` (`:22-23`). Append with `O_APPEND` (`:57-71`); pruned over a byte limit (`:75-77`). Export writes `~/Downloads/Ardia data <day>.jsonl` (`:285-293`).
- `freezes.txt`: opened `O_RDWR|O_CREAT` in the app data dir or Application Support (`packages/ardiacore/ios/ArdiaFreezeCapture.mm:254-265`). Slots: `kSlots = 20`, `kSlotBytes = 4096` (`:26-27`); written with `pwrite` (`:251`).
- `hermes-fatal.txt` / `hermes-fatal-last.txt`: names at `packages/shell/ios/HermesFatal.swift:4-5`; path installed at `:9-11`; raw write at `packages/shell/ios/ArdiaHermesFatal.mm:30`.
- `telemetry-requests.log`: rewritten whole each time, last 400 000 chars (`apps/browser/src/state/telemetry/requestLog.ts:25`). Out of version scope (`docs/conventions.md:225`).

### 3.8 Plugins and user scripts

- Plugin data root: App/`PluginData/` (`packages/shell/ios/PluginFolders.swift:98`). `PluginFolders.current()` finds App via `DocumentStore.url("plugins.json")` (`:17-19`).
- Plugin source folders are in Config/`plugins/` (`packages/shell/ios/PluginFolders.swift:20-23`).
- `storage.json` per plugin: 2 MB cap, must be JSON, refuses symlinks (`packages/shell/ios/PluginCore/PluginStorage.swift:19-20`, `:41-61`).
- Programs: copies in `Programs/`, links in `~/.local/bin` (`packages/shell/ios/PluginFolders.swift:41-46`). A `.path-line` marker in the programs store (`packages/shell/ios/PluginProgramLinks.swift:10`, `:327`) records which shell file got a PATH line.
- Outside the data dir: Ardia appends a PATH line to the user's shell start file (`.zprofile`, `.bash_profile`, `.profile` or `fish/conf.d/ardia.fish`) (`packages/shell/ios/PluginCore/PluginLoginPath.swift:30-42`, `:58-61`).
- User scripts: App/`Scripts/` (`packages/shell/ios/UserScripts.swift:30`); the app reads `generation`, `extension/`, takes a shared `flock` on `.lock` (`:74-96`), and owns `status.json` (`:140-148`). [INFERENCE: the CLI writes `generation` and `extension/`.]

### 3.9 Config folder

- The app reads `config.json`, `hooks.json` and the active theme file, and watches them (`apps/browser/src/state/config/configFiles.ts:229-234`; native reader `packages/shell/ios/ConfigFolder.swift:21-26`).
- The app writes only the schemas: always to App/`config-schema/`, and also into the config folder when it exists (`packages/shell/ios/ConfigFolder.swift:63-74`). File names: `config.schema.json`, `theme.schema.json` (`apps/browser/src/state/config/configSchema.ts:12-13`).
- The CLI writes `config.json` and themes (`docs/config.md:3`, `docs/cli.md`) [INFERENCE: not traced in CLI source].
- Native stamps `hooks.json`, `config.json`, `automation-approvals.json` and `air-traffic.json` (dev/ino/size/mtime) to decide if a link policy is stale (`packages/ardiacore/ios/ArdiaHost+Launch.mm:45-66`).

### 3.10 Control socket

- `ArdiaControl.sock` and lock `ArdiaControl.sock.lock` in App (`packages/shell/ios/ControlSocket.swift:7-8`, `:101-105`).
- Takes `flock(LOCK_EX|LOCK_NB)` on the lock; a second instance refuses (`:110-114`). Removes a stale socket only if it is a socket (`:116-122`). Path max 103 bytes (`:10`, `:107`).
- `ArdiaControl.json` is Linux only [INFERENCE: no Mac writer found by `grep ArdiaControl.json packages/shell/ios`].

### 3.11 Legacy migration (Netnyahoo → Ardia)

- Runs first in `main.swift` (`apps/browser/macos/Ardia-macOS/main.swift:10`).
- Journal `.legacy-migration.json`: `{version:1, phase, outcome, files, secrets, secretAttempts, groups, groupAttempts, defaultBrowser, finishedAt}` (`packages/sync/ios/Core/LegacyMigrator.swift:29-39`). Names `:50-54`. Lock `:82`. Writer `:450-455`. Reader `:529`.
- Renamed paths: `Chromium/NetnyahooPictureInPicture.plist` → `Chromium/ArdiaPictureInPicture.plist`, `.netnyahoo-managed.json` → `.ardia-managed.json`, `Netnyahoo Favicons` → `Ardia Favicons` (`packages/sync/ios/Core/LegacyMigration.swift:39-43`).
- `Chromium/ArdiaPictureInPicture.plist` is read by a self-test only (`packages/ardiacore/ios/ArdiaPiPController+SelfTest.mm:137`). No writer found in `packages`, `engine/ardiacore/src`, `patches` or `engine/patches` [INFERENCE: the engine writes it from code outside those trees].
- Legacy JS inputs, read once then deleted: `bookmarks.json` (`apps/browser/src/state/bookmarks/engineSync.ts:17`, `:194`), `history.json` (`apps/browser/src/state/history/engineSync.ts:17`, `:305`), via `migrateLegacyFile` (`apps/browser/src/state/app/legacyFiles.ts:5-24`). Old `favicons-<id>.json` files are removed (`apps/browser/src/state/favicons/favicons.ts:332-337`).

### 3.12 WebKit engine files

- `WebKit Content Rules/` is beside the Chromium dir, in App (`packages/ardiacore/ios/WebKitContentBlocker.swift:82-86`). `settings.json` = `{enabled, lists?, allowedHosts}` (`:28-32`).
- `WebKit Tab History/`: one file per tab key, max 32 MB, kept 90 days, version 1 (`packages/ardiacore/ios/WebKitTab+Lifecycle.swift:27-30`, `:42-50`).
- `WebKit Transport Security/p<hex>.json` (`packages/ardiacore/ios/WebKitTab+Certificate.swift:143-147`).
- WebKit website data stores use a UUID from SHA-256 of `ardia.webkit|<data dir>|<profile>` (`packages/ardiacore/ios/WebKitStores.swift:69-75`). WebKit owns where those stores live [INFERENCE: under `~/Library/WebKit/<bundle id>/`].

### 3.13 Chromium-owned files

The engine owns everything else under `Chromium/`: `Local State`, `SingletonLock` (checked by `packages/ardiacore/ios/ArdiaStartup.mm:17-18`), each profile's `Preferences`, `History`, `Bookmarks`, `Cookies`, `Login Data`, `Extensions/`, `Local Extension Settings/` (read at `packages/ardiacore/ios/ArdiaContentBlockerLite.mm:66`). Bookmarks and history live in Chromium; JS only keeps `bookmarks-cache.json`. Ria must treat these as engine files and not parse them [INFERENCE for the file list beyond the cited names].

## 4. Non-file stores

### Keychain

| Service | Account | Owner | Cite |
|---|---|---|---|
| `Ardia Safe Storage` | `Ardia` | Chromium cookie/password encryption key | `packages/sync/ios/Core/LegacyMigration.swift:28-30` |
| `Ardia Connected Accounts` | per account | JS `ArdiaKeychain` module | `packages/shell/ios/KeychainModule.swift:7`, add/update `:31-45` |
| `Ardia Sync Key` | per account | sync key, `AfterFirstUnlockThisDeviceOnly` | `packages/sync/ios/Core/LegacyMigration.swift:34-36` |

Test instance substitutes (file, mode 0600): `Connected Accounts.json` in App (`packages/shell/ios/KeychainModule.swift:72-75`, `:106-116`), and `sync-key-<account>` files (`packages/sync/ios/Core/SyncKeyStore.swift:57-73`, chosen at `packages/sync/ios/SyncModule.swift:354`).

### NSUserDefaults

With `ARDIA_DATA_DIR`, `standardUserDefaults` is swizzled to the suite `<D>/Preferences/<bundle id>` (`packages/shell/ios/ArdiaIsolation.m:44-49`). Otherwise the normal `~/Library/Preferences/<bundle id>.plist` [INFERENCE: macOS default].

| Key | Cite |
|---|---|
| `ArdiaHoldIncomingLinks`, `ArdiaLinkPolicyStamp` | `packages/ardiacore/ios/ArdiaHost+Launch.mm:33`, `:35` |
| `interfaceScale` | `packages/ardiacore/ios/ArdiaInterfaceScale.mm:12` |
| `ACPictureInPictureKeepOnTop`, `ACPictureInPictureZoom`, `ACPictureInPictureThrow` | `packages/ardiacore/ios/ArdiaPiPController.mm:12`, `:18`, `:24` |
| `ArdiaMovedDownloads`, `ArdiaDownloadMoveJournal` | `packages/ardiacore/ios/ArdiaDownloadMover.mm:17`, `:19` |
| `ardia.webkit.storeMade.<uuid>` | `packages/ardiacore/ios/WebKitStores.swift:94` |
| `ACAppIcon` | `packages/shell/ios/AppIcons.swift:13` |
| `ACUpdateChannel`, `ACUpdateFeedURL` (read) | `packages/shell/ios/AppUpdater.swift:14`, `:10` |
| `ACUpdatePrewarm` | `packages/shell/ios/UpdatePrewarm.swift:8` |
| `AppleIconAppearanceTheme` (read) | `packages/shell/ios/UpdaterCore/IconStyle.swift:4` |
| `ArdiaLastWindowFrame` | `packages/shell/ios/WindowManager.swift:19` |
| Sparkle `SUSkippedVersion`, `SUSkippedMajorVersion`, `SUSkippedMajorSubreleaseVersion` | `packages/shell/ios/SkippedUpdateDefaults.swift:5` |

### Other

- Debug builds give a scratch instance its own pasteboard `com.ardia.scratch:<D>` (`packages/shell/ios/ArdiaIsolation.m:24-29`, `:41-43`).
- Temp files: notification icons `$TMPDIR/ac-notification-<id>.<ext>` (`packages/shell/ios/NotificationHub.swift:155-157`); recovery kit PDF in `$TMPDIR/Ardia Recovery Kit/` (`packages/sync/ios/SyncModule.swift:307`, `:316`).

## 5. Test and dev only files (App dir, scratch instance)

Not part of user data. Listed so Ria does not mistake them for state.

| File | Writer | Reader |
|---|---|---|
| `dev-console.log` | `apps/browser/src/components/dev/devHarness.ts:56` | same line |
| `dev-eval-result.json` | `apps/browser/src/components/dev/devHarness.ts:259`, `apps/browser/src/platform/nativeApi.ts:36` | test harness |
| `layout-watch.json` | `apps/browser/src/state/sidebar/layoutWatch.ts:71` | test harness |
| `calendar-fixture.json` | test harness | `apps/browser/src/state/live/calendar.ts:83` |
| `perf-probe` | test harness | `apps/browser/src/state/app/perfRun.ts:14` |
| `activation.log` | `packages/shell/ios/SharePicker.swift:115-120`; `packages/ardiacore/ios/ArdiaActivation.mm:23` | test harness |
| `haptics.log` (background instance only) | `packages/shell/ios/HapticLog.swift:8-12` | test harness |
| `file-chooser.txt` | test harness | `packages/ardiacore/ios/ArdiaActivation.mm:74-80` (deletes after read) |
| `pip-button-selftest.json`, `pip-hover.json` | `packages/ardiacore/ios/ArdiaPiPController+SelfTest.mm:196`, `:229` | test harness |
| Menu choice test files | `packages/shell/ios/MenuChoice.swift:10-12` | test harness |
| `ardia-cli.json` (Linux only) | `apps/browser/src/state/app/linuxCommandInstall.ts:50` | `:42` |

## How I searched

All commands ran from the Ardia repo root. Hit counts are lines of output.

| Command | Hits | Purpose |
|---|---|---|
| `grep -rn "readDocument\|writeDocument" apps/browser/src packages --include=*.ts --include=*.tsx \| grep -v test \| wc -l` | 148 | first size check of the JS document API |
| `grep -rnE "\b(readDocument\|writeDocument\|saveDocument\|documentSaved\|removeDocument)\(" apps/browser/src --include=*.ts --include=*.tsx \| grep -v "\.test\.\|test-native\|platform/documents.ts"` | 88 | every JS document read/write call site |
| `grep -rnE "\"[A-Za-z0-9._-]+\.(json\|nns\|txt\|log\|jsonl)\"" apps/browser/src --include=*.ts --include=*.tsx \| grep -v "\.test\.\|test-native\|/dev/\|import " \| grep -E "(const\|=) "` | 48 | JS document name constants |
| `grep -rnE "^(export )?const [A-Z_]*VERSION\s*=\s*[0-9]+" apps/browser/src/state apps/browser/src/platform --include=*.ts \| grep -v test` | 35 | version constants |
| `grep -rnE "\.write\(to\|writeToFile\|createFileAtPath\|FileHandle\(forWriting\|fileHandleForWriting\|O_CREAT\|renameat?\(\|moveItem\(" packages/*/ios apps/browser/macos/Ardia-macOS --include=*.swift --include=*.mm --include=*.m --include=*.c \| grep -v "packages/phone\|/Tests/"` | 150 (66 files) | native file writers; each file checked |
| `grep -rn "DocumentStore.url(" packages/*/ios --include=*.swift \| grep -v packages/phone` | 11 | native files in the app data dir |
| `grep -rnE "LaunchConfig\.shared\.dataDirectory\|ArdiaLaunchConfig\.shared\.dataDirectory\|ArdiaHost\.dataDirectory" packages/*/ios apps/browser/macos/Ardia-macOS ...` | 27 | native users of the data dirs |
| `grep -rnE "UserDefaults\.standard\|standardUserDefaults" packages/*/ios apps/browser/macos/Ardia-macOS ... \| grep -v packages/phone` | 31 | NSUserDefaults keys |
| `grep -rnE "SecItemAdd\|SecItemUpdate\|kSecAttrService" packages/*/ios --include=*.swift --include=*.mm` | 12 | Keychain writers |
| `grep -rln "launch-guard" engine/ardiacore/src engine/ardiacore-linux apps/browser/macos packages scripts` | 1 (`scripts/windows/engine/src/ardia/win/main.cc`) | `launch-guard.json` writer |
| `grep -rln "PictureInPicture.plist" engine/ardiacore/src patches engine/patches` | 0 | PiP plist writer |
| `grep -rn "ardia-data-dir\|kArdiaDataDir" engine/ardiacore/src` and `grep -rn "user-data-dir\|userDataDir" packages/ardiacore/ios ...` | ~20 each | engine data dir handling |

Skipped / not checked: the CLI source (who writes `config.json`, themes, `Scripts/generation`), and the exact contents of `bangs.txt` and `plugins.json`. Risk: `ArdiaPictureInPicture.plist` and `launch-guard.json` have no Mac writer in the searched trees.
