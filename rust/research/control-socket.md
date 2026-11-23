Source: Ardia at frozen commit `551d3c13c`. Paths are relative to the Ardia repo root.

# Control socket contract (Ardia @ 551d3c13c, macOS app)

All paths are relative to the Ardia repo root. Abbreviations:
`AU` = `apps/browser/src/state/automation`, `SH` = `packages/shell/ios`.
The code is the authority. `docs/control-socket.md` and `docs/cli.md` are guides. The drift list is at the end.

## 1. Layers

| Layer | File | What it does |
| --- | --- | --- |
| Native socket server (Swift, Mac) | `SH/ControlSocket.swift:5-231` | Makes the Unix socket, accepts peers, checks peer UID, hands each line to JS. |
| Per-connection framing and I/O | `SH/ControlConnection.swift:4-246` | Reads bytes, splits on `\n`, enforces line, pending and stall limits, writes queued output. |
| Output queue | `SH/ControlOutput.swift:5-221` | Keeps replies and decide lines, evicts events, adds the `dropped` marker, native 1 MiB fallback. |
| Expo module bridge | `SH/AutomationModule.swift:6-28` | Module `ArdiaAutomation`; events `onRequest`, `onClose`, `onWritten`; functions `start`, `stop`, `reply`, `broadcast`. |
| JS binding | `apps/browser/src/platform/automation.ts:5-14` | Thin wrappers over the module. |
| Dispatcher | `AU/controlSocket.ts:132-269` | Parses, checks control and generation, validates params, dispatches. |
| Envelope parse, protocol, errors | `AU/automationMethods.ts:8-145` | `PROTOCOL`, error codes, `parseRequest`, `hello`, `planRun`, `parseSubscription`. |
| Method catalogue | `AU/socketCatalog.ts:13-195` | `MethodSpec`, core methods, registration, access check, param check, `stillServing`. |
| Schema checker | `AU/socketSchema.ts:23-86` | JSON Schema subset validator and builders. |
| Area registration | `AU/socketAreas.ts:26-30` | Registers the 21 area arrays once. |
| Line-size fitting | `AU/socketWire.ts:4-101` | 1 MiB fit for replies and events. |
| Decide hooks over the socket | `AU/decideSocket.ts:9-152` | `serveHooks`, decide requests, `answer`. |
| Shared-folder write guard | `AU/socketSharedChokePoint.ts:67-109` | Wraps every `mutates` request. |
| Plugin `plugins*` handlers | `apps/browser/src/state/plugins/socketMethods.ts:41-77` | Installed through `handlePluginSocketMethods` (`AU/controlSocket.ts:111`, `apps/browser/src/state/plugins/start.ts:16`). |

Startup: `apps/browser/src/main.tsx:72` calls `afterFirstWindow(startAutomation)`. `startAutomation` (`AU/controlSocket.ts:356-367`) registers areas, starts decide hooks, records launch, runs launch actions, then calls `sync()` and re-runs `sync()` on each settings change (`AU/controlSocket.ts:364-366`).
Plugin socket handlers register in `startPlugins` (`apps/browser/src/state/plugins/start.ts:18-26`), which returns early when `!PLATFORM_FEATURES.plugins` (`start.ts:19`). If that happens, `plugins*` still appear in `methods` (they are core specs) but answer `unknown_method` (`AU/controlSocket.ts:266-268`). [INFERENCE: plugins are on for macOS; not checked in `PLATFORM_FEATURES`.]

## 2. Transport

### 2.1 Socket file, folder and permissions

| Rule | Value | Source |
| --- | --- | --- |
| File name | `ArdiaControl.sock` | `SH/ControlSocket.swift:7` |
| Folder | the data folder: `LaunchConfig.shared.dataDirectory` if set, else `~/Library/Application Support/<bundle id or "Ardia">/` (created) | `SH/ControlSocket.swift:101`, `SH/DocumentStore.swift:24-33` |
| Folder safety | must be a real dir (lstat), owned by euid, not group/other writable; else start is refused | `SH/ControlSocket.swift:106`, `:219-226` |
| Path length | at most 103 UTF-8 bytes | `SH/ControlSocket.swift:10`, `:107-109` |
| Single instance | lock file `ArdiaControl.sock.lock`, opened `O_RDWR|O_CREAT|O_CLOEXEC|O_NOFOLLOW` mode 0600, `flock(LOCK_EX|LOCK_NB)`; failure = "Another Ardia is serving this data folder." | `SH/ControlSocket.swift:8`, `:110-115` |
| Stale file | an existing path is unlinked only if it is a socket; any other file type refuses start | `SH/ControlSocket.swift:116-123` |
| Socket | `socket(AF_UNIX, SOCK_STREAM, 0)`, non-blocking, `FD_CLOEXEC`, `SO_NOSIGPIPE` | `SH/ControlSocket.swift:124`, `:212-217` |
| Mode | `umask(0o177)` around `bind` (gives 0600) | `SH/ControlSocket.swift:138-142` [INFERENCE: resulting mode 0600 follows from umask; no explicit chmod] |
| Backlog / max live connections | 16 | `SH/ControlSocket.swift:9`, `:144`, `:185` |
| Peer check | `getpeereid` uid must equal `geteuid()`; else close at once. No token. | `SH/ControlSocket.swift:183-188` |
| Accepts per wakeup | 8 | `SH/ControlSocket.swift:11`, `:180` |
| Generation | incremented on each successful start; stamped on every request | `SH/ControlSocket.swift:150`, `:191-195` |
| Stop | unlinks the path only if it is still the same socket (dev + inode); closes lock | `SH/ControlSocket.swift:162-177` |
| Quit | `NSApplication.willTerminateNotification` stops the server | `SH/ControlSocket.swift:30-33` |
| Status to JS | `{state: "on"|"off", generation, path, reason?}` | `SH/ControlSocket.swift:86-95` |

Access model: every connection is `"owner"` (`AU/controlSocket.ts:214`). The plugin-session access path exists (`AU/socketCatalog.ts:25-27`, `:128-131`) but no connection is ever `"plugin"` yet.

### 2.2 Framing (newline-delimited JSON)

| Rule | Value | Source |
| --- | --- | --- |
| Request frame | bytes up to `\n` (byte 10); the line is decoded as UTF-8 | `SH/ControlConnection.swift:218-223` [INFERENCE: `String(decoding:as:UTF8.self)` repairs bad UTF-8 with U+FFFD] |
| Outbound frame | JSON text + `\n` | `SH/ControlSocket.swift:80-84` |
| Max inbound line | 1,048,576 bytes (excl. newline); larger = connection retired | `SH/ControlConnection.swift:5`, `:220`, `:230` |
| Max requests in flight per connection | 32 (unreplied); the 33rd retires the connection | `SH/ControlConnection.swift:6`, `:221-222`, `:58` |
| Partial-line stall | 30 s with an incomplete line retires the connection | `SH/ControlConnection.swift:7`, `:235`, `:238-245` |
| Read size | 64 KiB per read | `SH/ControlConnection.swift:11` |
| Peer half-close (EOF) | JS gets `onClose` at once; native keeps only replies, writes them for up to 5 s, then closes | `SH/ControlConnection.swift:187`, `:191-205`, `SH/ControlSocket.swift:203-206` |
| Retire | finish the line being written (max 5 s), drop the rest, max 16 retiring (17th closes the oldest) | `SH/ControlConnection.swift:8-9`, `:77-93` |
| Outbound max line | 1 MiB incl. newline (JS check) | `AU/socketWire.ts:4`, `:12-14` |
| Output queue | max 34 MiB total, events max 4 MiB and 1,000 waiting, 64 B kept for the marker | `SH/ControlOutput.swift:5-9` |
| Event eviction | oldest event first; next event gets `{"dropped":N,...}` prepended | `SH/ControlOutput.swift:51-67`, `:205-210` |
| Never dropped | replies and any line that starts `{"decide":` | `SH/ControlSocket.swift:68-78` |

Reply order: JS replies when each handler settles (`AU/controlSocket.ts:166-167`). [INFERENCE: async replies can come back out of request order; clients must match on `id`.]

### 2.3 Envelopes

| Kind | Shape | Source |
| --- | --- | --- |
| Request | `{"id": <safe integer or string ≤256 UTF-8 bytes, well-formed>, "method": <string>, "params"?: <object>}` | `AU/automationMethods.ts:28`, `:45-61` |
| Reply (ok) | `{"id", "result"}` | `AU/socketWire.ts:84`, `AU/controlSocket.ts:62` |
| Reply (error) | `{"id", "error": {"code", "message"}}` | `AU/automationMethods.ts:24` |
| Event | `{"event", "time", "data"}` (+ optional leading `"dropped"`) | `AU/socketWire.ts:96`, `SH/ControlOutput.swift:205-210` |
| Decide request | `{"decide", "event", "hook", "revision", "deadline", "time", "data"}` | `AU/decideSocket.ts:87-88` |
| Decide cancel | `{"decide": id, "cancel": true}` | `AU/decideSocket.ts:108` |
| Oversized read reply | error `failed` "The reply would be over 1 MiB…; ask for less (…)" | `AU/socketWire.ts:54`, `:92` |
| Oversized write reply | `{"id", "result": {…scalars, lists as {count}, ok, oversized: true, error?, hint}}` | `AU/socketWire.ts:56-81`, `:91` |
| Oversized error | `{code, message cut to 4096 chars}` | `AU/socketWire.ts:87-90` |
| Oversized event | scalars + `{count}` lists + `truncated: true`; else only `{truncated: true}` | `AU/socketWire.ts:95-101` |
| Native fallback | reply over 1 MiB at the queue becomes `failed` with echoed id (or `null`) | `SH/ControlOutput.swift:180-189`, `:212-220` |

Parse failures (`AU/automationMethods.ts:48-61`), all `bad_request`:
- not JSON → `id: null`, "The line isn't JSON."
- not an object → `id: null`
- bad `id` → `id: null`
- `method` not a string → original `id`
- `params` present but not an object → original `id`
- Missing `params` becomes `{}`.

### 2.4 Error codes

`ErrorCode` (`AU/automationMethods.ts:11`): `bad_request`, `unknown_method`, `unknown_command`, `refused`, `no_window`, `failed`, `password_manager_plugin`, `unsupported_protocol`.

| Code | Where it comes from |
| --- | --- |
| `bad_request` | parse errors; schema problems as `"<method>: <problem>"` (`AU/socketCatalog.ts:168-171`); handler checks; `OpError("bad_request")` |
| `unknown_method` | no spec (`AU/controlSocket.ts:222`), or spec with no handler (`:268`) |
| `unknown_command` | `run` with a non-catalogue command (`AU/automationMethods.ts:92`) |
| `refused` | policy refusals; `run` in a background instance (`AU/controlSocket.ts:181`); shared-folder veto (`AU/socketSharedChokePoint.ts:10`, `:77-81`); plugin session access (`AU/socketCatalog.ts:130`) |
| `no_window` | no usable window/tab/profile/etc. (`OpError`) |
| `failed` | control off ("Control is off.", `AU/controlSocket.ts:136`); thrown non-`OpError` errors (`AU/socketCatalog.ts:174`, `AU/controlSocket.ts:148-150`); `stillServing()` stop (`AU/socketCatalog.ts:7`, `:191-193`) |
| `unsupported_protocol` | `hello` with `protocol` < 4 (`AU/automationMethods.ts:65-70`) |
| `password_manager_plugin` | not traced in this pass; doc says only `passwords.list`/`passwords.remove` [INFERENCE from `docs/control-socket.md:117-120`] |

Handler errors: an `OpError(code, message)` (`AU/workspaceTargets.ts:15`) keeps its code; any other throw is `failed` with the error text (`AU/socketCatalog.ts:174`).
`plugins.call` passes through the tool outcome's code (`apps/browser/src/state/plugins/socketMethods.ts:38`). [INFERENCE: this can surface codes outside `ErrorCode` if `callTool` returns them; not checked.]

### 2.5 hello and protocol versioning

- Constants: `PROTOCOL = 4`, `MIN_CLIENT_PROTOCOL = 4` (`AU/automationMethods.ts:8-9`).
- Result: `{app: "Ardia", version: <appVersion>, protocol: 4, minClientProtocol: 4}` (`AU/automationMethods.ts:63`).
- `params.protocol` missing → treated as 4. Not a safe integer ≥1 → `bad_request`. Below 4 → `unsupported_protocol` (`AU/automationMethods.ts:65-70`). Schema types `protocol` as integer first (`AU/socketCatalog.ts:76-79`).
- `params.caller` `{name, app}` is cleaned and stored per connection (`AU/controlSocket.ts:234-235`, `apps/browser/src/state/app/controlCaller.ts:23-28`). The `caller` method then reports it as the one running (`AU/controlSocket.ts:238-242`).
- `hello` is not required before other methods. No code checks it. [Verified absence in `answerAs`, `AU/controlSocket.ts:220-269`.]
- The `hello` schema has no `additionalProperties: false`, so extra keys pass (`AU/socketCatalog.ts:76-79`, `AU/socketSchema.ts:61`).

### 2.6 Dispatch order (per request)

From `AU/controlSocket.ts:132-269`:
1. `parseRequest`; error → reply (`:133-134`).
2. If the native generation tag ≠ the JS generation, or `cliControl` is off → `failed` "Control is off." (`:136`).
3. Connection added to `openConnections` (`:137`). A `run` gets a write ticket (`:138-144`).
4. `guardedBySocketOrigin(spec.mutates ?? true, …)` (`:217`). Unknown methods count as mutating here.
5. No spec → `unknown_method` (`:222`).
6. `accessProblem` (always passes for `owner`) (`:223-224`).
7. `checkedParams`: schema check, then wrap params in a Proxy that throws on any read of an unlisted key (`AU/socketCatalog.ts:139-172`).
8. Core switch for `hello caller commands methods windows workspace offscreen apply subscribe answer run` (`:230-263`).
9. Else `runSpec` (area methods with `run`) (`:264-265`).
10. Else the plugin handler (`:266-267`). Else `unknown_method` (`:268`).
11. The reply is fitted to 1 MiB and counted in telemetry per family (`:153-165`). Pending `tab.idle` replays go out after the reply (`:157`, `:164`).

### 2.7 Consent gating (`cliControl`)

- Setting key `cliControl`, default `false` (`apps/browser/src/state/settings/settings.ts:139`). Read by `programControlOn` (`apps/browser/src/state/app/programControlSetting.ts:3-5`).
- On → `start()` calls native `start` and takes its generation (`AU/controlSocket.ts:303-311`). Off → clear connections, history handles, tab calls, asks, responders, then native `stop` (`:313-345`). The socket file is removed on stop (`SH/ControlSocket.swift:172-173`).
- Each request re-checks `controlOn()` (`AU/controlSocket.ts:136`).
- The socket cannot change `cliControl`: `settings.set` refuses it (`AU/socketSettings.ts:15`). `run` refuses commands that "touch control itself" (`AU/automationMethods.ts:78`).
- A plugin whose manifest asks for `control` turns `cliControl` on when enabled, unless the user turned control off before (`apps/browser/src/state/plugins/lifecycle.ts:262-263`).
- `consentToken()` in the dispatcher is telemetry consent, not control consent (`AU/controlSocket.ts:24`, `:152`, `:156`).

### 2.8 stillServing rule

- `stillOn = () => (origin.segment(), generation === serving && controlOn() && openConnections.has(connection))` (`AU/controlSocket.ts:228-229`).
- `serving` is the generation captured when the request started (`:228`). A connection leaves `openConnections` on close or peer EOF (`:277-287`) and on control off (`:314`, `:336`).
- Area handlers get `SocketCall {connection, serving, stillServing}` (`AU/socketCatalog.ts:11`, `:187-195`). `stillServing()` throws `OpError("failed", STOPPED)`: "Control was turned off before Ardia made the change, so nothing changed." (`AU/socketCatalog.ts:7`).
- Rule: call `stillServing()` after every `await` and before any write (example `AU/socketTabs.ts:64-69`, `AU/socketSites.ts:165-170`).
- `origin.segment()` also re-opens the shared-folder check for the next synchronous stretch (`AU/socketSharedChokePoint.ts:71-75`). A mutating request whose stretch changed a shared-folder projection is rolled back for those groups and answers `refused` (`AU/socketSharedChokePoint.ts:38-46`, `:77-81`).

### 2.9 Param schema validation

Validator `schemaProblem` (`AU/socketSchema.ts:23-71`). Supported keywords:

| Keyword | Rule | Line |
| --- | --- | --- |
| `anyOf` | at least one form must pass; message "doesn't match any of its forms" | `:24` |
| `type` (string or list) | `integer` = finite, whole, `Number.isSafeInteger`; `number` accepts integers; non-finite numbers always fail | `:5-19`, `:25-27` |
| `const`, `enum` | strict equality | `:28-29` |
| `minimum`, `maximum`, `exclusiveMinimum` | numbers only | `:30-34` |
| `pattern` | `new RegExp(pattern).test` on strings | `:35` |
| `minItems`, `maxItems`, `prefixItems`, `items` | arrays | `:36-51` |
| `required` | own property must exist and not be `undefined` | `:54` |
| `properties` | own properties only (`Object.hasOwn`) | `:12`, `:56` |
| `additionalProperties: false` | unknown key → "This method takes no X (it takes …)" | `:61-63` |
| `additionalProperties: <schema>` | applied to unknown keys | `:64-66` |

Builders: `obj()` always sets `additionalProperties: false` (`:73`). `ids()` = string array, `minItems 1`, `maxItems` default 500 (`:76`). `PAGE` = `limit` integer 1–1000, `offset` integer ≥0 (`:86`).
Second layer: handlers only see a Proxy. Reading, `in`, or `getOwnPropertyDescriptor` of an unlisted key throws `UnlistedParamError` → `failed` (`AU/socketCatalog.ts:133-166`). The Proxy recurses into nested closed objects (`:137`, `:144-148`). Prototype keys plus `toJSON` and `then` are allowed reads (`:133`).

### 2.10 subscribe and events

`subscribe` params (`AU/socketCatalog.ts:88-102`, parse at `AU/automationMethods.ts:131-145`):
- `events`* string list; `"*"` means all events except `NAMED_ONLY_EVENTS` (`AU/automationMethods.ts:137`, `AU/eventNames.ts:48-57`). Unknown names → `bad_request` listing all events (`:134-135`).
- `fullUrls` boolean. Without it, URLs in events are cut to origin + path (`AU/controlSocket.ts:84-89`, `AU/eventRedaction.ts`).
- `idle: {tab: [s], user: [s]}`: max 8 thresholds each, whole seconds from 60 to 604,800 (`AU/automationMethods.ts:123-129`, `AU/eventNames.ts:86`). `tab.idle`/`tab.resumed` and `user.idle`/`user.active` go only to matching thresholds (`AU/controlSocket.ts:72-82`).
- `hooks`: list (max 100) of hook objects from `hooks.json` to serve, or `null` to stop (`AU/decideSocket.ts:43-65`). A hook another connection already serves at the same revision is not taken.
- A new `subscribe` replaces the old one. Empty `events` removes it (`AU/controlSocket.ts:204-208`).
- Result `{ok: true, served: [names], revisions?: {name: revision}}`. `revisions` appears only when `hooks` was given (`AU/controlSocket.ts:211`).
- `app.launched` is replayed once per connection if the launch is less than 120 s old (`AU/controlSocket.ts:119-128`, `AU/lifecycleEvents.ts:38-41`, `AU/eventNames.ts:85`).

Events (39, `AU/eventNames.ts:1-41`):

| Event | In `"*"` | To plugins |
| --- | --- | --- |
| `window.opened`, `window.closed`, `window.focused` | yes | yes |
| `tab.opened`, `tab.closed`, `tab.activated`, `tab.navigated`, `tab.moved`, `tab.crashed` | yes | yes |
| `tab.idle`, `tab.resumed` (need `idle.tab`) | yes | yes |
| `settings.changed` | yes | yes |
| `app.launched`, `app.quitting` | yes | yes |
| `app.activated`, `app.deactivated` | no (named only) | yes |
| `media.started`, `media.stopped` | yes | yes |
| `notification.shown` | no (named only) | no |
| `peek.opened`, `peek.closed`, `littleArdia.opened` | yes | yes |
| `split.created`, `split.closed`, `pin.added`, `pin.removed` | yes | yes |
| `profile.switched`, `profile.changed` | yes | yes |
| `system.sleep`, `system.wake` | no (named only) | yes |
| `user.idle`, `user.active` (need `idle.user`) | no (named only) | yes |
| `page.error` | yes | yes |
| `bookmarks.changed`, `history.removed`, `download.changed` | yes | yes |
| `permission.prompted`, `menu.opened` | yes | no |
| `password.prompted` | no (named only) | no |

Sources: named-only set `AU/eventNames.ts:48-57`; plugin set `:44-46`. Event payload fields are not traced here (see `docs/automation.md` › Events).

Decide hooks (`AU/eventNames.ts:74`; deadlines `:77-83`): `tab.opening` 250 ms, `link.opening` 250 ms, `download.deciding` 2000 ms, `permission.requested` 2000 ms. `omnibox.query` (300 ms) has a deadline but is not in `DECIDE_EVENTS`.
- Limits: 64 pending total and 16 per connection (`AU/decideSocket.ts:9-10`).
- A decide line over 1 MiB is not sent (`:99-102`).
- On timeout or abort, Ardia sends `{decide, cancel: true}` and uses its default (`:104-112`).
- `answer {decide, answer}`: `answer` must be an object, a list or null; otherwise it is cancelled and returns `bad_request` (`AU/decideSocket.ts:119-137`, `AU/controlSocket.ts:257-260`).
- Callers: `AU/decisions.ts:40`, `AU/omniboxHooks.ts:40`.

## 3. Method table (every registered method)

Registration:
- `CORE_METHODS` (17) at `AU/socketCatalog.ts:75-110`.
- 21 area arrays registered in one call at `AU/socketAreas.ts:29`, through `registerSocketMethods` (`AU/socketCatalog.ts:115-121`). A duplicate name throws.
- `methods` returns all specs except `hidden` (`AU/socketCatalog.ts:123-124`).

Total registered: **157**. Listed by `methods`: **156**. `menus.choose` is `hidden` (`AU/socketSurfaces.ts:123`).

Legend:
- `*` = required.
- `area` = text before the first dot (`AU/socketCatalog.ts:66`). `family` = telemetry family (`AU/socketCatalog.ts:32-67`, default `core`).
- `P` = `profile: string` (optional unless marked).
- `L/O` = `limit` int 1–1000, `offset` int ≥0.
- `Mut` = `mutates`.
- All `obj()` params are closed (no extra keys).

### 3.1 Core (`AU/socketCatalog.ts`)

| Method | Family | Mut | Params | Result | Source |
| --- | --- | --- | --- | --- | --- |
| `hello` | core | no | `protocol`: integer; `caller`: object `{name, app}` (schema open) | `{app, version, protocol, minClientProtocol}` | `AU/socketCatalog.ts:76` |
| `caller` | core | no | none | `{ok}` | `AU/socketCatalog.ts:80` |
| `methods` | core | no | none | `[{name, area, mutates, description, params, result}]` | `AU/socketCatalog.ts:81` |
| `commands` | core | no | none | `[{id, title, …}]` | `AU/socketCatalog.ts:82` |
| `windows` | windows | no | none | `[{id, kind, focused, profile, profileName, tabs}]`; tabs get `targetId`, `pageGeneration` (`AU/controlSocket.ts:170-178`) | `AU/socketCatalog.ts:83` |
| `workspace` | workspace | no | `window`: string\|null; `profile`: string\|null; `usage`: boolean | `{windows, …}` | `AU/socketCatalog.ts:84` |
| `apply` | workspace | yes | `ops`*: array (open; ops are checked by `parseOps`: `open`, `tab.move`, `tab.pin`, `tab.unpin`, `tab.activate`, `tab.mute`, `tab.unmute`, `tab.sleep`, `tab.wake`, `tab.close`, `tab.rename`, `tab.icon`, `group.create`, `group.update`, `group.move`, `group.ungroup`, `group.close`, `group.delete`, `AU/workspaceApply.ts:330-370`) | `{ok, applied, results, tabs, groups, windows}` | `AU/socketCatalog.ts:85` |
| `run` | workspace | yes | `command`*: string; `arg`: string\|null; `window`: string\|null; `tab`: string\|null | `{ok}` | `AU/socketCatalog.ts:86` |
| `offscreen` | workspace | yes | `tab`*: string | `{ok}` | `AU/socketCatalog.ts:87` |
| `subscribe` | core | yes | `events`*: string[]; `fullUrls`: boolean; `idle`: `{tab: int[], user: int[]}` (closed); `hooks`: array\|null | `{ok, served, revisions?}` | `AU/socketCatalog.ts:89` |
| `answer` | core | yes | `decide`*: string; `answer`: any | `{ok}` | `AU/socketCatalog.ts:103` |
| `plugins` | plugins | no | none | `[{name, title, version, enabled, active, tools, …}]` (full row `apps/browser/src/state/plugins/socketMethods.ts:15-33`) | `AU/socketCatalog.ts:104` |
| `plugins.official` | plugins | no | none | `[…]` | `AU/socketCatalog.ts:105` |
| `plugins.enable` | plugins | yes | `name`*: string | `{ok}` \| `{waiting}` | `AU/socketCatalog.ts:106` |
| `plugins.disable` | plugins | yes | `name`*: string | `{ok}` | `AU/socketCatalog.ts:107` |
| `plugins.remove` | plugins | yes | `name`*: string | `{ok}` | `AU/socketCatalog.ts:108` |
| `plugins.call` | plugins | yes | `plugin`*: string; `tool`*: string; `input`: any | the tool's result | `AU/socketCatalog.ts:109` |

`run` rules (`AU/automationMethods.ts:89-120`):
- Unknown command → `unknown_command`.
- A policy decision other than `run` → `refused` (`endsApp`, `changesEngine`, `leftOutSetting`, `control`, `needsArgument`, `needsPerson`; `:74-81`).
- `arg` must be one of the command's variants.
- `tab` is allowed only for page-scope commands and `closeTab togglePin moveTabUp moveTabDown duplicateTab bookmarkPage` (`:85`).
- With no target, it uses the last focused eligible window.
- In a background instance, `run` is always `refused` (`AU/controlSocket.ts:181`).

Plugin handler behaviour (`apps/browser/src/state/plugins/socketMethods.ts:41-77`):
- `enable` of an official plugin starts its install and returns `{waiting: true}`. Another plugin opens Settings › Plugins for consent and returns `{waiting: true}`. An already active plugin returns `{ok: true}`. A consent sheet already open → `refused`.
- `disable` saves `plugins.json`.
- `remove` takes `serving` as its stop check.

### 3.2 Area methods

| Method | Family | Mut | Params | Result | Source |
| --- | --- | --- | --- | --- | --- |
| `airTraffic.rules` | air_traffic | no | none | `{rules, fallback, fallbackChosen, profiles: [{id, name, groups: [{key, name}]}]}` | `AU/socketAirTraffic.ts:162` |
| `airTraffic.addRule` | air_traffic | yes | `from`: `{name*, bundleId}`\|null; `when`: `{kind*: any\|site\|pattern\|contains\|equals, host, pattern, text}`; `profile`: string\|null; `open`: tab\|window\|littleArdia; `group`: string\|null; `enabled`: bool (`:145-158`) | `{id, enabled, from, when, profileId, open, groupKey?}` | `AU/socketAirTraffic.ts:163` |
| `airTraffic.updateRule` | air_traffic | yes | `id`*: string + all `addRule` fields | rule row | `AU/socketAirTraffic.ts:164` |
| `airTraffic.removeRule` | air_traffic | yes | `id`*: string | `{ok}` | `AU/socketAirTraffic.ts:165` |
| `airTraffic.moveRule` | air_traffic | yes | `id`*: string; `index`*: int ≥0 | `[rule id]` | `AU/socketAirTraffic.ts:166` |
| `airTraffic.setFallback` | air_traffic | yes | `open`*: tab\|window\|littleArdia\|null; `profile`: string\|null; `group`: string\|null | `{fallback}` | `AU/socketAirTraffic.ts:168` |
| `airTraffic.test` | air_traffic | no | `url`*: string; `from`: object `{name*, bundleId}` (null not allowed) | `{ok, url, dropped, profileName, groupName, usedLast, open}` | `AU/socketAirTraffic.ts:175` |
| `bookmarks.tree` | bookmarks | no | P; `folder`: string; `depth`: int 0–64 | `{bar, other, truncated}` \| `{…folder, truncated}` | `AU/socketLibrary.ts:173` |
| `bookmarks.search` | bookmarks | no | P; `query`: string; L/O | `{items, total, nextOffset}` | `AU/socketLibrary.ts:174` |
| `bookmarks.get` | bookmarks | no | `id`*: string | bookmark | `AU/socketLibrary.ts:175` |
| `bookmarks.add` | bookmarks | yes | `url`*: string; `title`: string; P; `parent`: string; `index`: int ≥0 | bookmark | `AU/socketLibrary.ts:176` |
| `bookmarks.addFolder` | bookmarks | yes | `title`*: string; P; `parent`; `index`: int ≥0 | folder | `AU/socketLibrary.ts:177` |
| `bookmarks.update` | bookmarks | yes | `id`*; `title`; `url` | bookmark | `AU/socketLibrary.ts:178` |
| `bookmarks.move` | bookmarks | yes | `ids`*: string[1–500]; `parent`*: string; `index`: int ≥0 | folder | `AU/socketLibrary.ts:179` |
| `bookmarks.remove` | bookmarks | yes | `ids`*: string[1–500] | `{removed}` | `AU/socketLibrary.ts:180` |
| `closed.list` | tabs | no | `window`: string | `[{id, kind, closedAt, …}]` | `AU/socketTabs.ts:172` |
| `closed.restore` | tabs | yes | `id`*: string; `window`: string | `{tabs}` | `AU/socketTabs.ts:179` |
| `closed.clear` | tabs | yes | `window`: string | `{cleared}` | `AU/socketTabs.ts:181` |
| `downloads.list` | downloads | no | P; `state`: downloading\|finished\|failed\|cancelled; L/O | `{items: [download], total, nextOffset}` | `AU/socketDownloads.ts:106` |
| `downloads.pause` | downloads | yes | `id`*: string | `{ok}` | `AU/socketDownloads.ts:107` |
| `downloads.resume` | downloads | yes | `id`* | `{ok}` | `AU/socketDownloads.ts:108` |
| `downloads.cancel` | downloads | yes | `id`* | `{ok}` | `AU/socketDownloads.ts:109` |
| `downloads.keep` | downloads | yes | `id`* | `{ok}` | `AU/socketDownloads.ts:110` |
| `downloads.open` | downloads | yes | `id`* | `{ok}` | `AU/socketDownloads.ts:111` |
| `downloads.reveal` | downloads | yes | `id`* | `{ok}` | `AU/socketDownloads.ts:112` |
| `downloads.remove` | downloads | yes | `id`* | `{ok}` | `AU/socketDownloads.ts:113` |
| `downloads.clear` | downloads | yes | none | `{removed}` | `AU/socketDownloads.ts:114` |
| `downloads.settings` | downloads | no | P | `{folder, askWhereToSave, managed}` | `AU/socketDownloads.ts:115` |
| `downloads.setSettings` | downloads | yes | P; `folder`: string; `askWhereToSave`: bool | `{folder, askWhereToSave, managed}` | `AU/socketDownloads.ts:116` |
| `extensions.install` | extensions | yes | P; `webStore`: string; `folder`: string (always refused) | `{waiting, id}` \| `{installed, extensions}` | `AU/socketExtensions.ts:112` |
| `extensions.configure` | extensions | yes | P; `id`*; exactly one of `siteAccess`: allSites\|specificSites\|onClick, `incognito`: bool, `userScripts`: bool (`anyOf`, `:101-108`) | `{id, name, incognito, userScripts, siteAccess, sites}` | `AU/socketExtensions.ts:121` |
| `extensions.list` | extensions | no | P | `[extension]` | `AU/socketSites.ts:156` |
| `extensions.setEnabled` | extensions | yes | P; `id`*; `enabled`*: bool | extension | `AU/socketSites.ts:157` |
| `extensions.setPinned` | extensions | yes | P; `id`*; `pinned`*: bool | extension | `AU/socketSites.ts:158` |
| `extensions.reload` | extensions | yes | P; `id`* | `{ok}` | `AU/socketSites.ts:160` |
| `extensions.uninstall` | extensions | yes | P; `id`* | `{removed}` | `AU/socketSites.ts:172` |
| `favicons.get` | favicons | no | `url`*: string; P | `{url, profile, found, mime, data}` | `AU/socketFavicons.ts:40` |
| `history.search` | history | no | P; `query`: string; `since`, `until`: int ≥0 (ms); L/O | `{items: [{id, url, title, visits, lastVisit, clipped?}], total, nextOffset, truncated, reason?}` | `AU/socketHistory.ts:119` |
| `history.remove` | history | yes | P; `ids`: string[1–1000]; `urls`: string[1–1000] | `{ok}` | `AU/socketHistory.ts:127` |
| `history.clear` | history | yes | P; `since`: int ≥0 | `{ok}` | `AU/socketHistory.ts:134` |
| `littleArdia.open` | windows | yes | `url`: string; P | window | `AU/socketWindows.ts:143` |
| `littleArdia.close` | windows | yes | `window`*: string | `{ok}` | `AU/socketWindows.ts:144` |
| `littleArdia.toMain` | windows | yes | `window`* | `{tab, window}` | `AU/socketWindows.ts:145` |
| `live.list` | live | no | P (all profiles when left out) | `{folders: [row], accounts}` | `AU/socketLive.ts:185` |
| `live.items` | live | no | `folder`*: string; L/O | `{items: [{item, shown, unread, completed}], total, nextOffset}` | `AU/socketLive.ts:187` |
| `live.create` | live | yes | `kind`*: pullRequests\|documents; P; `sources`: string[1–3] of github\|bitbucket\|notion\|confluence\|gdrive; `name`: string | row | `AU/socketLive.ts:195` |
| `live.update` | live | yes | `folder`*; `name`; `icon`: string\|null; `color`: none\|tab group colours\|null; `collapsed`: bool; `sources`: string[1–3]; `filters`: `{authored: bool, reviewRequests: bool}`; `groupStacks`: bool; `showCompleted`: bool | row | `AU/socketLive.ts:203` |
| `live.remove` | live | yes | `folder`* | `{ok}` | `AU/socketLive.ts:220` |
| `live.refresh` | live | yes | `folder`* | row | `AU/socketLive.ts:221` |
| `live.hide` | live | yes | `folder`*; `items`*: string[1–200] | row | `AU/socketLive.ts:222` |
| `live.unhide` | live | yes | `folder`* | row | `AU/socketLive.ts:223` |
| `live.markRead` | live | yes | `folder`*; `items`: string[1–200] | row | `AU/socketLive.ts:225` |
| `live.setRepoHidden` | live | yes | `folder`*; `repo`*: string; `hidden`*: bool | row | `AU/socketLive.ts:232` |
| `live.open` | live | yes | `folder`*; `item`*: string; `window`: string; `background`: bool | `{tab, window}` | `AU/socketLive.ts:234` |
| `media.list` | media | no | none | `[{tab, window, title, artist, state, position, duration, …}]` | `AU/socketMisc.ts:95` |
| `media.control` | media | yes | `tab`*; `action`*: play\|pause\|toggle\|next\|previous\|seekBy\|seekTo\|stop; `seconds`: number −86400…864000 | `{ok}` | `AU/socketMisc.ts:97` |
| `menus.open` | menus | no | none | `[{id, kind, window, tab, items}]` | `AU/socketSurfaces.ts:110` |
| `menus.choose` (hidden) | menus | yes | `menu`*: string; `path`*: int[1–8] each ≥0; `ref`: string | always `refused` (captured menu) or `no_window` | `AU/socketSurfaces.ts:121` |
| `menus.dismiss` | menus | yes | `menu`*: string | `{ok}` | `AU/socketSurfaces.ts:130` |
| `menus.bar` | menus | no | none | `[{path, ref, title, separator, key?, command?, arg?, action?, dynamic?, submenu?}]` | `AU/socketSurfaces.ts:138` |
| `menus.runBar` | menus | yes | `path`*: int[1–8]; `ref`*: string; `window`: string | `{ok, title, command? \| action?}` | `AU/socketSurfaces.ts:150` |
| `autofill.list` | secrets | no | P | `{addresses, cards}` | `AU/socketMisc.ts:107` |
| `autofill.prompts` | secrets | no | none | `[{id, tab, window, origin, kind, fields, description}]` | `AU/socketPrompts.ts:152` |
| `autofill.respond` | secrets | yes | `tab`*; `id`*: int ≥1; `decision`*: accept\|decline\|dismiss | `{ok}` | `AU/socketPrompts.ts:160` |
| `passwords.list` | secrets | no | P | `[{origin, username, created, modified}]` | `AU/socketMisc.ts:105` |
| `passwords.remove` | secrets | yes | P; `origin`*: string; `username`*: string | `{ok}` | `AU/socketMisc.ts:106` |
| `passwords.prompts` | secrets | no | none | `[{id, tab, window, origin, state, username, usernames, usernameCount, federation}]` | `AU/socketPrompts.ts:128` |
| `passwords.respond` | secrets | yes | `tab`*; `id`*: string; `decision`*: save\|update\|never\|notNow\|dismiss; `usernameId`: string | always `refused` (`AU/socketPrompts.ts:80-84`) | `AU/socketPrompts.ts:136` |
| `permissions.prompts` | permissions | no | none | `[{id, window, tab, origin, permissions, queued}]` | `AU/socketPrompts.ts:112` |
| `permissions.respond` | permissions | yes | `id`*: string; `decision`*: allow\|allowOnce\|block\|dismiss | always `refused` (`AU/socketPrompts.ts:51-53`) | `AU/socketPrompts.ts:120` |
| `peek.list` | peek | no | none | `[{source, tab, window, url, title}]` | `AU/socketSplits.ts:126` |
| `peek.open` | peek | yes | `tab`*; `url`*: string | `{source, tab}` | `AU/socketSplits.ts:127` |
| `peek.close` | peek | yes | `tab`* | `{ok}` | `AU/socketSplits.ts:129` |
| `peek.toTab` | peek | yes | `window`* | `{tab}` | `AU/socketSplits.ts:142` |
| `pins.folders` | pins | no | `window`: string | `[{id, window, profile, name, label, tabs}]` | `AU/socketPins.ts:113` |
| `pins.createFolder` | pins | yes | `tabs`*: string[1–100]; `name`: string | folder row | `AU/socketPins.ts:115` |
| `pins.renameFolder` | pins | yes | `folder`*; `window`; `name`*: string | folder row | `AU/socketPins.ts:122` |
| `pins.removeFolder` | pins | yes | `folder`*; `window` | `{ok, tabs}` | `AU/socketPins.ts:123` |
| `pins.addToFolder` | pins | yes | `folder`*; `window`; `tabs`*: string[1–100] | folder row | `AU/socketPins.ts:124` |
| `pins.removeFromFolder` | pins | yes | `tabs`*: string[1–100] | `{ok}` | `AU/socketPins.ts:125` |
| `presets.describe` | presets | no | `code`*: string | `{preset, settings, color, pins, lines}` | `AU/socketPresets.ts:40` |
| `presets.apply` | presets | yes | `code`*: string; `window`: string | `{ok, window, lines}` | `AU/socketPresets.ts:42` |
| `profiles.list` | profiles | no | none | `[profile]` | `AU/socketProfiles.ts:104` |
| `profiles.create` | profiles | yes | `name`*: string; `color`: plum\|blue\|purple\|pink\|red\|orange\|yellow\|green\|neutral; `icon`: string; `shareDataWith`: string; `shareTopApps`: bool | profile | `AU/socketProfiles.ts:106` |
| `profiles.update` | profiles | yes | `profile`*; `name`; `icon`: string\|null; `pinnedCollapsed`, `shareTopApps`, `syncTabs`: bool | profile | `AU/socketProfiles.ts:114` |
| `profiles.setLook` | profiles | yes | `profile`*; `color`: same enum; `theme`: string\|object\|null | profile | `AU/socketProfiles.ts:122` |
| `profiles.reorder` | profiles | yes | `profiles`*: string[1–200] | `[profile]` | `AU/socketProfiles.ts:129` |
| `profiles.setDefault` | profiles | yes | `profile`* | profile | `AU/socketProfiles.ts:131` |
| `profiles.delete` | profiles | yes | `profile`* | `{deleted}` | `AU/socketProfiles.ts:144` |
| `settings.get` | settings | no | `keys`: string[1–200] | `{key: value}` | `AU/socketSettings.ts:146` |
| `settings.schema` | settings | no | none | `[{key, schema, default, writable, why?, synced}]` | `AU/socketSettings.ts:147` |
| `settings.set` | settings | yes | `values`*: object (open) | `{changed, values}` | `AU/socketSettings.ts:149` |
| `settings.reset` | settings | yes | `keys`*: string[1–200] | `{changed, values}` | `AU/socketSettings.ts:156` |
| `search.engines` | settings | no | none | `[{id, name, keyword, url, custom, default, controlledBy?}]` | `AU/socketSettings.ts:157` |
| `search.setDefault` | settings | yes | `id`*: string | engine | `AU/socketSettings.ts:159` |
| `search.add` | settings | yes | `name`*, `keyword`*, `url`*: string | engine | `AU/socketSettings.ts:172` |
| `search.update` | settings | yes | `id`*; `name`, `keyword`, `url` | engine | `AU/socketSettings.ts:173` |
| `search.remove` | settings | yes | `id`* | `{ok}` | `AU/socketSettings.ts:174` |
| `sharedFolders.list` | shared_folders | no | none | `[{profile, key, groups, name, path, url, links, state, waiting, error, addedAt}]` | `AU/socketSharedFolders.ts:46` |
| `sites.origins` | sites | no | P | `[origin]` | `AU/socketSites.ts:125` |
| `sites.get` | sites | no | P; `origin`*: string matching `^https?://[^/?#\s]+$` | `{type: {value, isDefault}}` | `AU/socketSites.ts:126` |
| `sites.set` | sites | yes | P; `origin`*; `type`*: one of 22 kinds (`AU/socketSites.ts:12-15`); `value`*: allow\|block\|ask\|default | `{type: {value, isDefault}}` | `AU/socketSites.ts:128` |
| `sites.reset` | sites | yes | P; `origin`* | `{ok}` | `AU/socketSites.ts:136` |
| `sites.clearData` | sites | yes | P; `site`*: string | `{site, cookies, storage}` | `AU/socketSites.ts:147` |
| `sites.zoomLevels` | sites | no | P | `{host: zoom}` | `AU/socketSites.ts:154` |
| `sites.setZoom` | sites | yes | P; `host`*: string; `zoom`*: number 0.25–5 | `{host: zoom}` | `AU/socketSites.ts:155` |
| `splits.list` | splits | no | `window`: string | `[split]` | `AU/socketSplits.ts:83` |
| `splits.create` | splits | yes | `tabs`*: string[2–3]; `orientation`: horizontal\|vertical | split | `AU/socketSplits.ts:84` |
| `splits.separate` | splits | yes | `split`*: string | `{ok}` | `AU/socketSplits.ts:86` |
| `splits.removeTab` | splits | yes | `tab`* | `{ok}` | `AU/socketSplits.ts:99` |
| `splits.resize` | splits | yes | `split`*; `sizes`*: number[2–4], each >0 and ≤1,000,000 | split | `AU/socketSplits.ts:112` |
| `splits.flip` | splits | yes | `split`* | split | `AU/socketSplits.ts:114` |
| `splits.setOrientation` | splits | yes | `split`*; `orientation`*: horizontal\|vertical | split | `AU/socketSplits.ts:125` |
| `sync.status` | sync | no | none | `{status, enabled, folder, types, profiles, devices, remoteTabs, …}` | `AU/socketMisc.ts:104` |
| `sync.now` | sync | yes | none | `{status, lastSyncedAt, pending, error, types, profiles}` | `AU/socketSync.ts:101` |
| `sync.setType` | sync | yes | `type`*: a `DATA_TYPES` id (`apps/browser/src/state/sync/syncDoc.ts:11-34`); `on`*: bool | `{status, types, profiles, …}` | `AU/socketSync.ts:102` |
| `sync.setProfile` | sync | yes | `profile`*; `on`*: bool | `{status, types, profiles, …}` | `AU/socketSync.ts:103` |
| `sync.setOpenTabs` | sync | yes | `profile`*; `on`*: bool | `{status, types, profiles, …}` | `AU/socketSync.ts:104` |
| `sync.devices` | sync | no | P | `[{id, name, kind}]` | `AU/socketSync.ts:105` |
| `sync.sendTab` | sync | yes | `tab`*; `device`*: string | `{ok, device}` | `AU/socketSync.ts:106` |
| `tabs.list` | tabs | no | `window`: string; P | `[tab]` (row fields `AU/socketTabs.ts:19-46`) | `AU/socketTabs.ts:139` |
| `tabs.get` | tabs | no | `tab`* | tab | `AU/socketTabs.ts:140` |
| `tabs.navigate` | tabs | yes | `tab`*; `url`*: string | tab | `AU/socketTabs.ts:141` |
| `tabs.reload` | tabs | yes | `tab`*; `bypassCache`: bool | `{ok}` | `AU/socketTabs.ts:142` |
| `tabs.back` | tabs | yes | `tab`* | `{ok}` | `AU/socketTabs.ts:143` |
| `tabs.forward` | tabs | yes | `tab`* | `{ok}` | `AU/socketTabs.ts:144` |
| `tabs.duplicate` | tabs | yes | `tab`* | tab | `AU/socketTabs.ts:146` |
| `tabs.setPinnedUrl` | tabs | yes | `tab`*; `url`: string | tab | `AU/socketTabs.ts:157` |
| `tabs.returnToPinnedUrl` | tabs | yes | `tab`* | tab | `AU/socketTabs.ts:159` |
| `windows.get` | windows | no | `window`* | window | `AU/socketWindows.ts:93` |
| `windows.create` | windows | yes | P; `url`: string; `layout`: sidebar\|top | window | `AU/socketWindows.ts:95` |
| `windows.close` | windows | yes | `window`* | `{ok}` | `AU/socketWindows.ts:103` |
| `windows.focus` | windows | yes | `window`* | `{ok}` | `AU/socketWindows.ts:111` |
| `windows.setLayout` | windows | yes | `window`*; `layout`: sidebar\|top; `tabsShown`: bool | window | `AU/socketWindows.ts:123` |
| `windows.switchProfile` | windows | yes | `window`*; `profile`* | window | `AU/socketWindows.ts:130` |
| `windows.merge` | windows | yes | `window`* | window | `AU/socketWindows.ts:132` |
| `windows.geometry` | windows | no | `window`* | `{frame, zoomed, fullScreen, miniaturized, visible, display}` | `AU/socketWindowGeometry.ts:118` |
| `windows.setFrame` | windows | yes | `window`*; `frame`*: `[x, y, w, h]` numbers; x, y in ±100,000; w, h in 1…100,000 (`:101-114`) | window | `AU/socketWindowGeometry.ts:125` |
| `windows.setMaximized` | windows | yes | `window`*; `on`*: bool | window | `AU/socketWindowGeometry.ts:126` |
| `windows.setFullScreen` | windows | yes | `window`*; `on`*: bool | window | `AU/socketWindowGeometry.ts:127` |

Count per source file (registered): socketCatalog 17, socketTabs 12, socketSites 12, socketDownloads 11, socketLive 11, socketSplits 11, socketWindows 10, socketSettings 9, socketLibrary 8, socketAirTraffic 7, socketProfiles 7, socketMisc 6, socketPins 6, socketPrompts 6, socketSync 6, socketSurfaces 5, socketWindowGeometry 4, socketHistory 3, socketExtensions 2, socketPresets 2, socketFavicons 1, socketSharedFolders 1. Total 157.

## 4. Plugin helper channel (not the socket, but the same envelope)

Plugin helper processes use the native module `ArdiaPlugins` (`apps/browser/src/platform/plugins.ts:43`, `:49-50`, `:72`), not the Unix socket. The channel reuses `parseRequest`, `parseSubscription` and the `{id, result|error}` and `{event, time, data}` envelopes (`apps/browser/src/state/plugins/channel.ts:64-92`). [INFERENCE: the native transport is the helper's stdio; not read in Swift.]

- Channel protocol: `CHANNEL_PROTOCOL = 2`, legacy 1 for manifests without `main` (`apps/browser/src/state/plugins/channelApi.ts:23-24`, `:107`).
- No schema validation: there is no `checkedParams` on this path.
- An inactive plugin gets `refused` "This plugin is off." (`channel.ts:77`).
- Extra event: `plugin.values` (`channel.ts:118`).

| Method | Gate | Source |
| --- | --- | --- |
| `subscribe` | only events in the manifest's non-decide `hooks`; `fullUrls` from the manifest | `apps/browser/src/state/plugins/channel.ts:35-57`, `:60` |
| `hello` | none; returns `{app, version, protocol, plugin, values}` | `apps/browser/src/state/plugins/channelApi.ts:106-107` |
| `values` | none | `channelApi.ts:108-109` |
| `ready` | current run | `channelApi.ts:110-111` |
| `windows` | `tabs` permission; URLs cut unless `fullUrls` | `channelApi.ts:112-113` |
| `run` | `commands` permission | `channelApi.ts:114-115` |
| `open` | `commands` or `tabs: write`; 1–10 http(s) URLs | `channelApi.ts:71-83`, `:116-117` |
| `workspace` | `workspace` permission | `channelApi.ts:95-96`, `:118` |
| `apply` | `workspace`, or `tabs: write` for `open`/`tab.close`/`tab.activate` only | `channelApi.ts:85-101`, `:119-120` |
| `tabs.readActivePage` | `activePage` + recent user gesture | `channelApi.ts:121-122`, `apps/browser/src/state/plugins/channelData.ts:135-148` |
| `page.open` | manifest `page`; `context` needs `tabs` | `channelApi.ts:35-45`, `:123-124` |
| `tools.call` | callee's permissions must be a subset | `channelApi.ts:47-55`, `:125-126` |
| `net.fetch` | `network` not false; https only; allowed hosts | `channelApi.ts:127-128`, `channelData.ts:81-91` |
| `programs.status`, `programs.relink` | own programs | `channelApi.ts:129-134` |
| `result` | answers a pending tool call | `channelApi.ts:135-138` |
| `storage.get`, `storage.set`, `storage.delete`, `storage.keys` | `storage` permission | `channelApi.ts:140`, `channelData.ts:39` |
| `secrets.get`, `secrets.set`, `secrets.delete` | `secrets` permission | `channelApi.ts:141`, `channelData.ts:52` |
| `ui.showQuickPick`, `ui.showInputBox`, `ui.showConfirm`, `ui.showToast`, `surface.render`, `surface.close`, `toolbar.update`, `panel.open`, `panel.close` | per-surface and toolbar checks | `apps/browser/src/state/plugins/channelUi.ts:168`, `channelApi.ts:142` |

Channel total: 32 methods (`subscribe` + 14 in the switch + 4 storage + 3 secrets + 9 UI).

## 5. Notes for Halyard (Rust port)

- Keep byte limits exactly: 1 MiB per line, 32 in flight, 30 s stall, 16 live + 16 retiring, 34 MiB / 4 MiB / 1,000 queue, 64 B marker room. Shipped clients (`control.py`, `control.rs` per `docs/cli.md:80-82`) drop the connection on longer lines.
- The method catalogue is data (`MethodSpec`). A Rust port can keep one static table and generate `methods` from it. The `hidden` flag must exist for `menus.choose`.
- The validator has two layers: the schema check, and a "read only listed keys" guard. The second is a JS Proxy. In Rust, typed param structs with `deny_unknown_fields` give the same effect [INFERENCE].
- `hello` is the only method whose top-level schema is open.

## How I searched

| Command (repo root = Ardia) | Hits |
| --- | --- |
| `grep -n 'export const [A-Z_]*_METHODS' AU/*.ts apps/browser/src/state/plugins/*.ts` | 24 lines: 22 `MethodSpec[]` arrays (incl. `CORE_METHODS`), `PLUGIN_SESSION_METHODS`, `UI_METHODS` |
| `grep -c 'name: "' AU/socket*.ts` (non-zero files) | 21 files, 140 hits |
| awk over each `*_METHODS` array for `name: "x"` + `core("x"` in `socketCatalog.ts` | 157 names, 0 duplicates (`uniq -d` empty) |
| `grep -rln "registerSocketMethods\|handlePluginSocketMethods" apps packages` | 3 files (`controlSocket.ts`, `socketAreas.ts`, `socketCatalog.ts`) + `plugins/start.ts` |
| `rg -l "ArdiaAutomation" --glob '!node_modules'` | 17 files; Mac ones: `SH/AutomationModule.swift`, `apps/browser/src/platform/automation.ts` |
| `rg -n "AF_UNIX\|sockaddr_un\|control\.sock\|\.sock\""` | 30 lines shown (capped); Mac server `SH/ControlSocket.swift` (via `grep -rln ControlSocket SH`: 2 files) |
| Doc coverage: for each of 157 names, `grep -E "\`name[\` ]" docs/control-socket.md` | 0 missing (all 157 named, incl. hidden `menus.choose`) |
| Same against `docs/cli.md` | 152 of 157 not named (cli.md names areas, not methods) |
| `grep -rn cliControl apps/browser/src` (ts/tsx, non-test) | 8 lines |
| `grep -n "requestDecision" AU/*.ts` | 3 (definition + 2 callers) |
| Read whole: `controlSocket.ts`, `socketSchema.ts`, `socketCatalog.ts`, `socketAreas.ts`, `automationMethods.ts` (1-145), `socketWire.ts`, `socketSharedChokePoint.ts`, `decideSocket.ts`, `eventNames.ts`, `plugins/socketMethods.ts`, `channel.ts`, `channelApi.ts`, `ControlSocket.swift`, `ControlConnection.swift`, `AutomationModule.swift`; parts of `ControlOutput.swift`, `DocumentStore.swift`, every `*_METHODS` array | n/a |

Not checked: handler bodies past their schemas (most `run` functions), `password_manager_plugin` origin, plugin native transport, Linux server (`engine/ardiacore-linux`, out of scope), event payload fields.

## Drift (doc vs code)

| # | Doc says | Code says |
| --- | --- | --- |
| 1 | `docs/control-socket.md:17-19` lists the schema subset as `type, properties, required, enum, items, minimum, maximum, minItems, maxItems, additionalProperties: false, description`. | The validator also uses `anyOf`, `const`, `pattern`, `prefixItems`, `exclusiveMinimum`, and schema-valued `additionalProperties` (`AU/socketSchema.ts:24-66`). Specs use them: `pattern` `AU/socketSites.ts:17,122`; `prefixItems` `AU/socketWindowGeometry.ts:106`; `anyOf` `AU/socketExtensions.ts:103`; `exclusiveMinimum` `AU/socketSplits.ts:112`. |
| 2 | `docs/control-socket.md:256`: "Every method also takes nothing it doesn't list (`additionalProperties: false`)". | `hello` has an open schema and accepts unknown keys (`AU/socketCatalog.ts:76-79`). Its `caller` object is open too. |
| 3 | `docs/control-socket.md:115-118` error list. | It omits `unknown_command` (`AU/automationMethods.ts:92`) and `unsupported_protocol` (`AU/automationMethods.ts:69`). Both are in `ErrorCode` (`:11`). `docs/cli.md:66` does name `unsupported_protocol`. |
| 4 | `docs/cli.md:55` cites `state/app/cliControl.ts`. | No such file. The key lives in `apps/browser/src/state/app/programControlSetting.ts:3-5`. |
| 5 | `docs/control-socket.md:6-7`: the socket exists only while `cliControl` is on, and with it off every request answers `failed` "Control is off.". | Both are true at different times. Off → JS stops the native server and the file is unlinked (`AU/controlSocket.ts:326`, `SH/ControlSocket.swift:172-173`). "Control is off." only reaches requests in the gap before stop, or with a stale generation (`AU/controlSocket.ts:136`). |
| 6 | `docs/control-socket.md:299`: `live.open {folder, item, window?}`. | The schema also takes `background: bool` (`AU/socketLive.ts:237`). |
| 7 | `docs/control-socket.md:292`: `extensions.install {profile?, webStore}` (webStore looks required). | The schema has no `required` (`AU/socketExtensions.ts:116`). [Handler check not read.] |
| 8 | `docs/control-socket.md:499`: the geometry result includes `pageFullScreen` and `refused`. | The catalogue `result` string (what `methods` reports) omits both: `{frame, zoomed, fullScreen, miniaturized, visible, display}` (`AU/socketWindowGeometry.ts:122`). The handler does read `pageFullScreen` and `refused` from the native geometry (`AU/socketWindowGeometry.ts:38`, `:81`). The drift is in the self-description only. [INFERENCE: the payload passes them through; the pass-through was not traced.] |
| 9 | `docs/control-socket.md:704`: save/update password prompts are "listed and answered above". | `passwords.respond` always throws `refused` (`AU/socketPrompts.ts:80-84`). The same doc says so at `:596`. Internal doc contradiction. |
| 10 | `docs/control-socket.md:256` implies one rule for all methods. | Unknown methods are treated as mutating by the shared-folder guard (`AU/controlSocket.ts:217`, `?? true`). The doc does not say this. Minor. |
| 11 | (no doc) | Code-internal mismatch: `splits.resize` allows 2–4 `sizes` (`AU/socketSplits.ts:112`), but `MAX_SPLIT_PANES = 3` (`apps/browser/src/state/splits/splits.ts:41`) and `splits.create` caps at 3 (`AU/socketSplits.ts:84`). |
| 12 | `docs/control-socket.md:230` "the 109 keys of `SETTING_SPECS`". | Not verified in this pass. |

Matches I checked and found consistent: 1 MiB line, 32 pending, 30 s stall, 5 s retire, 16 connections, 34 MiB / 4 MiB / 1,000 events, 64 B marker (`docs/control-socket.md:160-201` vs `SH/ControlOutput.swift:5-9`, `SH/ControlConnection.swift:5-9`). Also: protocol 4 / min 4 (`docs/cli.md:65` vs `AU/automationMethods.ts:8-9`), mode 0600 + `getpeereid` (`docs/cli.md:58-59` vs `SH/ControlSocket.swift:138`, `:185`), `PLUGIN_SESSION_METHODS = {hello, methods}` (`docs/control-socket.md:547-551` vs `AU/socketCatalog.ts:27`), and `menus.choose` hidden (`docs/control-socket.md:600` vs `AU/socketSurfaces.ts:123`).
