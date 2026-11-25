# Recorded cases of slice 2.2

Check U for the units of `apps/browser/src/lib` that `ria_state::shared` ports. Each crate has the recorded cases of the
TypeScript original (`cases.jsonl`), its extra inputs (`inputs/<name>.inputs.jsonl`) and a thin `src/port.rs` that calls `ria_state::shared`.
A `*.harness.ts` file wraps functions that the wire cannot carry (a `Uint8Array` result, random ids, a stateful promise helper, a data table).

Slice 2.2 has 3 parts (issues 173, 174, 175). This file lists the crates of the parts that are merged.

## Part 1 (issue 173)

| Crate | Unit (`apps/browser/src/lib/`) |
| --- | --- |
| `absolute-folder-rs` | `absoluteFolder.ts` |
| `badge-fit-rs` | `badgeFit.ts` |
| `color-rs` | `color.ts` |
| `custom-icon-rs` | `customIcon.ts` |
| `deadline-rs` | `deadline.harness.ts` |
| `decode-rs` | `decode.ts` |
| `emoji-data-rs` | `emojiData.harness.ts` |
| `emoji-rs` | `emoji.ts` |
| `errors-rs` | `errors.harness.ts` |
| `fallback-rs` | `fallback.harness.ts` |
| `interface-scale-rs` | `interfaceScale.ts` |
| `json-schema-rs` | `jsonSchema.ts` |
| `legacy-rs` | `legacy.harness.ts` |
| `link-source-rs` | `linkSource.ts` |
| `link-wrappers-rs` | `linkWrappers.ts` |
| `match-pattern-rs` | `matchPattern.ts` |
| `press-motion-rs` | `pressMotion.harness.ts` |
| `saved-document-rs` | `savedDocument.harness.ts` |
| `sha256-rs` | `sha256.harness.ts` |
| `single-flight-rs` | `singleFlight.harness.ts` |
| `single-line-rs` | `singleLine.ts` |
| `tile-letter-rs` | `tileLetter.ts` |
| `time-rs` | `time.ts` |
| `toolbar-badge-rs` | `toolbarBadge.ts` |
| `toolbar-row-rs` | `toolbarRow.ts` |
| `unread-rs` | `unread.ts` |
| `url-parts-rs` | `urlParts.ts` |
| `url-rs` | `url.ts` |
| `versions-rs` | `versions.ts` |
| `webloc-rs` | `webloc.harness.ts` |

Files with no crate (no function that the bridge can carry; each module doc says what is not ported):
`uiLocale.ts`, `hoverMarks.ts`, `hermesHeap.ts`, `afterFirstWindow.ts`, `devEval.ts`, `keys.ts`.
The Windows, Linux and web variants of a file (`*.windows.ts`, `*.linux.ts`, `*.web.ts`) are outside the mac scope and are not ported.

The part split follows import clusters, not the alphabet, because the alphabetical split would give part 2 a need for files of part 3 (issue 173).

## Run

Build the TypeScript oracle once (needs a checkout of Ardia at the pinned commit, or `git submodule update --init --checkout ardia`):

```sh
rust/recorded/2.2/review-root.sh [path-to-ardia-checkout]
```

Then, for each crate:

```sh
rust-anything diff   rust/recorded/2.2/<crate>
rust-anything gate   rust/recorded/2.2/<crate>
rust-anything fuzz   rust/recorded/2.2/<crate> --seed <new seed>
```

`rust-anything build rust/recorded/2.2/<crate>` makes the `.node` module first. Use `CARGO_TARGET_DIR` on a big disk.
`time-rs` reads a clock: use `TZ=UTC RIA_NOW_MS=1795417631295 NODE_OPTIONS=--import=$PWD/rust/recorded/2.2/time-rs/clock-freeze.mjs` (see its README).
Crates whose coverage is below 100 % have an `UNREACHED.md`; the lines belong to another unit that covers them.
Crates with differences that the Rust `String` forces (lone surrogates and similar) have a `known-diffs.jsonl`.

## Part 2 (issue 174)

| Crate | Unit (`apps/browser/src/lib/`) |
| --- | --- |
| `plugin-activation-rs` | `pluginActivation.ts` |
| `plugin-checks-rs` | `pluginChecks.ts` |
| `plugin-commands-rs` | `pluginCommands.ts` |
| `plugin-contributions-rs` | `pluginContributions.ts` |
| `plugin-manifest-rs` | `pluginManifest.ts` |
| `plugin-markdown-rs` | `pluginMarkdown.ts` |
| `plugin-migrations-rs` | `pluginMigrations.ts` |
| `plugin-names-rs` | `pluginNames.ts` |
| `plugin-network-rs` | `pluginNetwork.ts` |
| `plugin-nodes-rs` | `pluginNodes.ts` |
| `plugin-page-bar-rs` | `pluginPageBar.ts` |
| `plugin-password-manager-rs` | `pluginPasswordManager.ts` |
| `plugin-permissions-rs` | `pluginPermissions.ts` |
| `plugin-rows-rs` | `pluginRows.ts` |
| `plugin-runtime-rs` | `pluginRuntime.ts` |
| `plugin-schema-parts-rs` | `pluginSchemaParts.ts` |
| `plugin-schema-rs` | `pluginSchema.ts` |
| `plugin-schema-v1-rs` | `pluginSchemaV1.ts` |
| `plugin-shortcuts-rs` | `pluginShortcuts.ts` |
| `plugin-theme-css-rs` | `pluginThemeCss.ts` |
| `plugin-web-url-rs` | `pluginWebUrl.ts` |
| `plugin-when-rs` | `pluginWhen.ts` |

`pluginProtocol.ts` holds only types: it has a Rust port (`shared/plugin_protocol.rs`) and no crate.
Crates with known differences (lone surrogates and similar) have a `known-diffs.jsonl`; lines the cases do not reach are in `UNREACHED.md`.

## Part 3 (issue 175)

| Crate | Unit (`apps/browser/src/lib/`) |
| --- | --- |
| `active-look-rs` | `activeLook.ts` |
| `minimalism-look-rs` | `minimalismLook.ts` |
| `platform-chrome-rs` | `platformChrome.ts` |
| `preset-look-harness-rs` | `presetLookHarness.ts` |
| `preset-look-rs` | `presetLook.ts` |
| `profile-look-rs` | `profileLook.ts` |
| `selection-contrast-rs` | `selectionContrast.ts` |
| `tab-corners-rs` | `tabCorners.ts` |
| `theme-file-rs` | `themeFile.ts` |
| `theme-gesture-rs` | `themeGesture.ts` |
| `theme-look-rs` | `themeLook.ts` |
| `window-button-layout-rs` | `windowButtonLayout.ts` |
| `window-chrome-rs` | `windowChrome.ts` |
| `window-layout-rs` | `windowLayout.ts` |
| `window-tint-rs` | `windowTint.ts` |

`sidebarMetrics.ts`, `platformFeatures.ts` and `platformTerms.ts` export only constants and types: they have Rust ports and no crate. The crates of `presetLook`, `activeLook`, `minimalismLook`, `tabCorners` and `themeGesture` use a `*.harness.ts` file because the wire cannot carry their values.
Arguments outside the TS types are not recorded (`record --count 0`); each README has a fuzz scope note.
