// Ported from ardia apps/browser/src/platform/*.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of those files, with changes.

//! The native calls of Ardia: one [`PlatformCall`] variant for each `call(..)` of `ardiacore` and each method of a
//! `nativeModule(..)` that a `src/platform` wrapper makes.
//!
//! Each variant has the wire name of the call and the fields of the wrapper arguments. [`PlatformCall::name`] and
//! [`PlatformCall::args`] give the `{name, args}` pair that the replay fake of `nativeFake()` records, so a Rust run
//! and a TypeScript run compare as JSON (plan section 6.5, check S).
//!
//! Wire styles: `positional` is a `nativeModule` method, whose `args` are the arguments in order. `object` is an
//! `ardiacore` `call(name, params)`, whose `args` hold one object with the camelCase field names, or none when
//! the call has no field. A field named in the `[..]` list of an `object` variant is a TypeScript spread: its object
//! is merged into the params.
//!
//! Changes from Ardia: a TypeScript wrapper that returns a promise and awaits is two actions here (plan section 6.5),
//! so a variant holds only the request. Event listeners (`on`, `addListener`), the JS timer and event-bridge
//! plumbing, the Windows files and the no-op test marks are not calls and have no variant. A TypeScript type
//! that this crate does not have yet is a `serde_json::Value` field; the slice of its state module narrows it.

use serde_json::{Map, Value};

use crate::kernel::Number;

/// Converts a snake_case field name to the camelCase key of the wire.
fn camel(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut up = false;
    for ch in field.trim_start_matches("r#").chars() {
        if ch == '_' {
            up = true;
        } else if up {
            out.extend(ch.to_uppercase());
            up = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// The JSON of one field. A field type always serializes.
fn json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// The `args` of an `object` call: one params object, or none when it has no field.
fn object_args(fields: Vec<(&str, Value)>, merge: &[&str]) -> Vec<Value> {
    if fields.is_empty() {
        return Vec::new();
    }
    let mut params = Map::new();
    for (key, value) in fields {
        match value {
            Value::Object(spread) if merge.contains(&key) => params.extend(spread),
            value => {
                params.insert(camel(key), value);
            }
        }
    }
    vec![Value::Object(params)]
}

macro_rules! platform_calls {
    ($($(#[$doc:meta])* $Variant:ident($wire:literal, $style:ident[$($merge:literal),*]) { $($field:ident : $ty:ty),* $(,)? }),* $(,)?) => {
        /// One native call that an action asks for.
        #[derive(Debug, Clone, PartialEq)]
        pub enum PlatformCall {
            $($(#[$doc])* $Variant { $($field: $ty),* },)*
        }

        impl PlatformCall {
            /// The wire name: `module.method` of a `nativeModule`, or the name given to `call`.
            pub fn name(&self) -> &'static str {
                match self { $(Self::$Variant { .. } => $wire,)* }
            }

            /// The arguments as the replay fake of Ardia records them.
            pub fn args(&self) -> Vec<Value> {
                match self {
                    $(Self::$Variant { $($field),* } => {
                        let fields: Vec<(&str, Value)> = vec![$((stringify!($field).trim_start_matches("r#"), json($field))),*];
                        if stringify!($style) == "object" {
                            object_args(fields, &[$($merge),*])
                        } else {
                            fields.into_iter().map(|(_, value)| value).collect()
                        }
                    })*
                }
            }
        }
    };
}

platform_calls! {
    /// `platform/app.ts: engineInfo`.
    EngineInfo("engine.info", object[]) {  },
    /// `platform/app.ts: listComponents`.
    EngineComponents("engine.components", object[]) {  },
    /// `platform/app.ts: setAppearance`.
    ShellSetAppearance("ArdiaShell.setAppearance", positional[]) { mode: Value },
    /// `platform/app.ts: isDarkAppearance`.
    ShellIsDarkAppearance("ArdiaShell.isDarkAppearance", positional[]) {  },
    /// `platform/app.ts: systemInfo`.
    AppSystemInfo("ArdiaApp.systemInfo", positional[]) {  },
    /// `platform/app.ts: crashReports`.
    AppCrashReports("ArdiaApp.crashReports", positional[]) { since: Number },
    /// `platform/app.ts: hardwareInfo`.
    AppHardwareInfo("ArdiaApp.hardwareInfo", positional[]) {  },
    /// `platform/app.ts: diagnosticsSample`.
    AppDiagnosticsSample("ArdiaApp.diagnosticsSample", positional[]) {  },
    /// `platform/app.ts: openExternalURL`.
    AppOpenExternalURL("ArdiaApp.openExternalURL", positional[]) { url: String },
    /// `platform/app.ts: appIcons`.
    AppAppIcons("ArdiaApp.appIcons", positional[]) { size: Number },
    /// `platform/app.ts: currentAppIcon`.
    AppAppIcon("ArdiaApp.appIcon", positional[]) {  },
    /// `platform/app.ts: setAppIcon`.
    AppSetAppIcon("ArdiaApp.setAppIcon", positional[]) { id: String },
    /// `platform/app.ts: isInDock`.
    AppIsInDock("ArdiaApp.isInDock", positional[]) {  },
    /// `platform/app.ts: nativeInterfaceScale`.
    AppInterfaceScale("ArdiaApp.interfaceScale", positional[]) {  },
    /// `platform/app.ts: nativeInterfaceScaleRange`.
    AppInterfaceScaleRange("ArdiaApp.interfaceScaleRange", positional[]) {  },
    /// `platform/app.ts: setNativeInterfaceScale`.
    AppSetInterfaceScale("ArdiaApp.setInterfaceScale", positional[]) { scale: Number },
    /// `platform/app.ts: addToDock`.
    AppAddToDock("ArdiaApp.addToDock", positional[]) {  },
    /// `platform/app.ts: updaterState`.
    UpdaterUpdaterState("ArdiaUpdater.updaterState", positional[]) {  },
    /// `platform/app.ts: checkForUpdates`.
    UpdaterCheckForUpdates("ArdiaUpdater.checkForUpdates", positional[]) {  },
    /// `platform/app.ts: setAutomaticUpdateChecks`.
    UpdaterSetAutomaticUpdateChecks("ArdiaUpdater.setAutomaticUpdateChecks", positional[]) { on: bool },
    /// `platform/app.ts: setAutomaticUpdateDownloads`.
    UpdaterSetAutomaticUpdateDownloads("ArdiaUpdater.setAutomaticUpdateDownloads", positional[]) { on: bool },
    /// `platform/app.ts: setUpdateChannel`.
    UpdaterSetUpdateChannel("ArdiaUpdater.setUpdateChannel", positional[]) { channel: Value },
    /// `platform/autofill.ts: getAutofillSettings`.
    AutofillSettings("autofill.settings", object[]) { profile: String },
    /// `platform/autofill.ts: setAutofillSettings`.
    AutofillSetSettings("autofill.setSettings", object["settings"]) { profile: String, settings: Value },
    /// `platform/autofill.ts: listAddresses`.
    AutofillAddresses("autofill.addresses", object[]) { profile: String },
    /// `platform/autofill.ts: saveAddress`.
    AutofillSaveAddress("autofill.saveAddress", object[]) { profile: String, address: Value },
    /// `platform/autofill.ts: deleteAddress`.
    AutofillRemove("autofill.remove", object[]) { profile: String, id: String },
    /// `platform/autofill.ts: listCards`.
    AutofillCards("autofill.cards", object[]) { profile: String },
    /// `platform/autofill.ts: revealCardNumber`.
    AutofillCardNumber("autofill.cardNumber", object[]) { profile: String, id: String },
    /// `platform/autofill.ts: showAutofillSuggestions`.
    AutofillShowSuggestions("autofill.showSuggestions", object[]) { tab_id: String, passwords: Value },
    /// `platform/automation.ts: startControlSocket`.
    AutomationStart("ArdiaAutomation.start", positional[]) {  },
    /// `platform/automation.ts: stopControlSocket`.
    AutomationStop("ArdiaAutomation.stop", positional[]) {  },
    /// `platform/automation.ts: replyOnControlSocket`.
    AutomationReply("ArdiaAutomation.reply", positional[]) { connection: Number, line: String, ticket: Number },
    /// `platform/automation.ts: broadcastOnControlSocket`.
    AutomationBroadcast("ArdiaAutomation.broadcast", positional[]) { connections: Vec<Number>, line: String },
    /// `platform/bookmarks.ts: bookmarkTree`.
    BookmarksTree("bookmarks.tree", object[]) { profile: String },
    /// `platform/bookmarks.ts: applyBookmarkOps`.
    BookmarksApply("bookmarks.apply", object[]) { profile: String, ops: Value },
    /// `platform/bookmarks.ts: watchBookmarks`.
    BookmarksWatch("bookmarks.watch", object[]) { profile: String },
    /// `platform/bookmarks.ts: exportBookmarksHtml`.
    BookmarksExportHtml("bookmarks.exportHtml", object[]) { profile: String },
    /// `platform/calendar.ts: calendarAuthorization`.
    CalendarAuthorizationStatus("ArdiaCalendar.authorizationStatus", positional[]) {  },
    /// `platform/calendar.ts: requestCalendarAccess`.
    CalendarRequestAccess("ArdiaCalendar.requestAccess", positional[]) {  },
    /// `platform/calendar.ts: systemCalendars`.
    CalendarCalendars("ArdiaCalendar.calendars", positional[]) {  },
    /// `platform/calendar.ts: systemCalendarEvents`.
    CalendarEvents("ArdiaCalendar.events", positional[]) { start: Number, end: Number, calendar_ids: Vec<String> },
    /// `platform/clipboard.ts: copyText`.
    ShellCopyText("ArdiaShell.copyText", positional[]) { text: String },
    /// `platform/clipboard.ts: readText`.
    ShellReadText("ArdiaShell.readText", positional[]) {  },
    /// `platform/commandInstall.ts: installedCommandVersion`.
    CliInstalledVersion("ArdiaCli.installedVersion", positional[]) {  },
    /// `platform/commandInstall.ts: canInstallCommandUnattended`.
    CliCanInstallUnattended("ArdiaCli.canInstallUnattended", positional[]) {  },
    /// `platform/commandInstall.ts: installCommand`.
    CliInstall("ArdiaCli.install", positional[]) {  },
    /// `platform/commands.ts: menuShortcuts`.
    MenusMenuShortcuts("ArdiaMenus.menuShortcuts", positional[]) {  },
    /// `platform/commands.ts: recordShortcut`.
    MenusRecordShortcut("ArdiaMenus.recordShortcut", positional[]) {  },
    /// `platform/commands.ts: cancelShortcutRecording`.
    MenusCancelRecording("ArdiaMenus.cancelRecording", positional[]) {  },
    /// `platform/commands.ts: setMenuStateParts`.
    MenusSetMenuBookmarks("ArdiaMenus.setMenuBookmarks", positional[]) { bookmarks: Value },
    /// `platform/commands.ts: setMenuStateParts`.
    MenusSetMenuState("ArdiaMenus.setMenuState", positional[]) { state: Value },
    /// `platform/configFolder.ts: readConfigFile`.
    ConfigReadFile("ArdiaConfig.readFile", positional[]) { name: String },
    /// `platform/configFolder.ts: watchConfigFiles`.
    ConfigWatch("ArdiaConfig.watch", positional[]) { names: Vec<String> },
    /// `platform/configFolder.ts: writeConfigSchemas`.
    ConfigWriteSchemas("ArdiaConfig.writeSchemas", positional[]) { files: Value },
    /// `platform/contentBlocker.ts: getContentBlocker`.
    ContentBlockerState("contentBlocker.state", object[]) {  },
    /// `platform/contentBlocker.ts: setContentBlockerEnabled`.
    ContentBlockerSetEnabled("contentBlocker.setEnabled", object[]) { enabled: bool },
    /// `platform/contentBlocker.ts: setFilterListEnabled`.
    ContentBlockerSetListEnabled("contentBlocker.setListEnabled", object[]) { id: String, enabled: bool },
    /// `platform/contentBlocker.ts: addFilterList`.
    ContentBlockerAddList("contentBlocker.addList", object[]) { url: String },
    /// `platform/contentBlocker.ts: removeFilterList`.
    ContentBlockerRemoveList("contentBlocker.removeList", object[]) { id: String },
    /// `platform/contentBlocker.ts: isContentBlockerAllowed`.
    ContentBlockerIsAllowed("contentBlocker.isAllowed", object[]) { host: String },
    /// `platform/contentBlocker.ts: setContentBlockerAllowed`.
    ContentBlockerSetAllowed("contentBlocker.setAllowed", object[]) { host: String, allowed: bool },
    /// `platform/contentBlocker.ts: getHiddenElements`.
    ContentBlockerHiddenElements("contentBlocker.hiddenElements", object[]) { profile: String },
    /// `platform/contentBlocker.ts: hideElements`.
    ContentBlockerHideElements("contentBlocker.hideElements", object[]) { profile: String, filters: Vec<String> },
    /// `platform/contentBlocker.ts: restoreElements`.
    ContentBlockerRestoreElements("contentBlocker.restoreElements", object[]) { profile: String, filters: Vec<String> },
    /// `platform/control.ts: setControl`.
    EngineSetCliControl("engine.setCliControl", object[]) { on: bool },
    /// `platform/control.ts: setControlActiveTab`.
    EngineSetCliActiveTab("engine.setCliActiveTab", object[]) { tab_id: Option<String> },
    /// `platform/control.ts: controlStatus`.
    EngineCliControlStatus("engine.cliControlStatus", object[]) {  },
    /// `platform/control.ts: controlTabTargets`.
    EngineCliTabTargets("engine.cliTabTargets", object[]) { tab_ids: Vec<String> },
    /// `platform/control.ts: controlOwnsTab`.
    EngineCliOwnsTab("engine.cliOwnsTab", object[]) { tab_id: String },
    /// `platform/control.ts: controlMoveOffscreen`.
    EngineCliMoveOffscreen("engine.cliMoveOffscreen", object[]) { tab_id: String },
    /// `platform/control.ts: controlCloseOffscreenTabs`.
    EngineCliCloseOffscreenTabs("engine.cliCloseOffscreenTabs", object[]) {  },
    /// `platform/dev.ts: devCrash`.
    DevDevCrash("ArdiaDev.devCrash", positional[]) { kind: Option<String> },
    /// `platform/dev.ts: devRenderIntroMusic`.
    DevDevRenderIntroMusic("ArdiaDev.devRenderIntroMusic", positional[]) { cues: Value, path: String },
    /// `platform/dev.ts: devRunAppleScript`.
    DevDevRunAppleScript("ArdiaDev.devRunAppleScript", positional[]) { source: String },
    /// `platform/dev.ts: devSnapshotWindow`.
    DevDevSnapshotWindow("ArdiaDev.devSnapshotWindow", positional[]) { window_id: String, path: String, transparent: bool },
    /// `platform/dev.ts: devSetPointer`.
    DevDevSetPointer("ArdiaDev.devSetPointer", positional[]) { window_id: String, x: Number, y: Number, placed: bool },
    /// `platform/dev.ts: devSetMouseButtons`.
    DevDevSetMouseButtons("ArdiaDev.devSetMouseButtons", positional[]) { state: Number },
    /// `platform/dev.ts: devHoverStats`.
    DevDevHoverStats("ArdiaDev.devHoverStats", positional[]) { reset: bool },
    /// `platform/dev.ts: devSidebarEdgePointer`.
    DevDevSidebarEdgePointer("ArdiaDev.devSidebarEdgePointer", positional[]) { window_id: String, x: Number, y: Number, placed: bool },
    /// `platform/dev.ts: devSidebarEdgeStats`.
    DevDevSidebarEdgeStats("ArdiaDev.devSidebarEdgeStats", positional[]) { reset: bool },
    /// `platform/dev.ts: devTypeKeys`.
    DevDevTypeKeys("ArdiaDev.devTypeKeys", positional[]) { window_id: String, text: String, interval: Number },
    /// `platform/dev.ts: devKeyEquivalent`.
    DevDevKeyEquivalent("ArdiaDev.devKeyEquivalent", positional[]) { window_id: String, press: Value },
    /// `platform/dev.ts: devClick`.
    DevDevClick("ArdiaDev.devClick", positional[]) { window_id: String, click: Value },
    /// `platform/dev.ts: devMenuCommand`.
    DevDevMenuCommand("ArdiaDev.devMenuCommand", positional[]) { command: String, arg: Option<String> },
    /// `platform/dialogs.ts: confirm`.
    DialogsConfirm("ArdiaDialogs.confirm", positional[]) { options: Value },
    /// `platform/dialogs.ts: prompt`.
    DialogsPrompt("ArdiaDialogs.prompt", positional[]) { options: Value },
    /// `platform/dialogs.ts: pickFiles`.
    DialogsPickFiles("ArdiaDialogs.pickFiles", positional[]) {  },
    /// `platform/dialogs.ts: pickFolder`.
    DialogsPickFolder("ArdiaDialogs.pickFolder", positional[]) { current: Option<String>, prompt: Option<String> },
    /// `platform/documents.ts: readDocument`.
    DocumentsReadDocument("ArdiaDocuments.readDocument", positional[]) { name: String },
    /// `platform/documents.ts: writeDocument`.
    DocumentsWriteDocument("ArdiaDocuments.writeDocument", positional[]) { name: String, contents: String },
    /// `platform/documents.ts: saveDocument`.
    DocumentsSaveDocument("ArdiaDocuments.saveDocument", positional[]) { name: String, contents: String },
    /// `platform/documents.ts: documentSaved`.
    DocumentsDocumentSaved("ArdiaDocuments.documentSaved", positional[]) { name: String },
    /// `platform/documents.ts: removeDocument`.
    DocumentsRemoveDocument("ArdiaDocuments.removeDocument", positional[]) { name: String },
    /// `platform/downloads.ts: cancelDownload`.
    DownloadsCancel("downloads.cancel", object[]) { id: String },
    /// `platform/downloads.ts: pauseDownload`.
    DownloadsPause("downloads.pause", object[]) { id: String },
    /// `platform/downloads.ts: resumeDownload`.
    DownloadsResume("downloads.resume", object[]) { id: String },
    /// `platform/downloads.ts: keepDownload`.
    DownloadsKeep("downloads.keep", object[]) { id: String },
    /// `platform/downloads.ts: getDownloadSettings`.
    DownloadsSettings("downloads.settings", object[]) { profile: String },
    /// `platform/downloads.ts: setDownloadSettings`.
    DownloadsSetSettings("downloads.setSettings", object["patch"]) { patch: Value, profile: String },
    /// `platform/downloads.ts: removeDownloadStaging`.
    WebKitRemoveDownloadStaging("webKit.removeDownloadStaging", object[]) { path: String },
    /// `platform/extensions.ts: listExtensions`.
    ExtensionsList("extensions.list", object[]) { profile: String },
    /// `platform/extensions.ts: inspectUnpackedExtension`.
    ExtensionsInspectUnpacked("extensions.inspectUnpacked", object[]) { path: String },
    /// `platform/extensions.ts: installExtension`.
    ExtensionsInstall("extensions.install", object[]) { profile: String, path: Value },
    /// `platform/extensions.ts: installExtensionCrx`.
    ExtensionsInstallCrx("extensions.installCrx", object[]) { profile: String, path: String },
    /// `platform/extensions.ts: chooseExtensionFolder`.
    ExtensionsChooseFolder("extensions.chooseFolder", object[]) {  },
    /// `platform/extensions.ts: setExtensionEnabled`.
    ExtensionsSetEnabled("extensions.setEnabled", object[]) { profile: String, id: String, enabled: bool },
    /// `platform/extensions.ts: uninstallExtension`.
    ExtensionsUninstall("extensions.uninstall", object[]) { profile: String, id: String },
    /// `platform/extensions.ts: reloadExtension`.
    ExtensionsReload("extensions.reload", object[]) { profile: String, id: String },
    /// `platform/extensions.ts: configureExtension`.
    ExtensionsConfigure("extensions.configure", object[]) { profile: String, id: String, options: Value },
    /// `platform/extensions.ts: extensionActionStates`.
    ExtensionsActionStates("extensions.actionStates", object[]) { profile: String, tab_id: String, ids: Vec<String> },
    /// `platform/extensions.ts: watchExtensionActions`.
    ExtensionsWatchActions("extensions.watchActions", object[]) { profile: String },
    /// `platform/extensions.ts: resolveExtensionInstallPrompt`.
    ExtensionsResolveInstallPrompt("extensions.resolveInstallPrompt", object[]) { profile: Value, request_id: Value, accepted: bool },
    /// `platform/extensions.ts: extensionSidePanelUrl`.
    ExtensionsSidePanelUrl("extensions.sidePanelUrl", object[]) { profile: String, tab_id: String, extension_id: String },
    /// `platform/extensions.ts: extensionsSyncState`.
    ExtensionsSyncState("extensions.syncState", object[]) { profile: String },
    /// `platform/extensions.ts: extensionsLoadHealth`.
    ExtensionsLoadHealth("extensions.loadHealth", object[]) { profile: String },
    /// `platform/extensions.ts: applyExtensionsSync`.
    ExtensionsSyncApply("extensions.syncApply", object["apply"]) { apply: Value, profile: String },
    /// `platform/focus.ts: setSwitcherCapture`.
    FocusSetSwitcherCapture("ArdiaFocus.setSwitcherCapture", positional[]) { active: bool },
    /// `platform/focus.ts: captureEscape`.
    FocusSetDragEscapeCapture("ArdiaFocus.setDragEscapeCapture", positional[]) { now: Value },
    /// `platform/focus.ts: cancelFocusMove`.
    FocusCancelFocusMove("ArdiaFocus.cancelFocusMove", positional[]) { generation: Number, restoring_source: bool },
    /// `platform/focus.ts: claimFocus`.
    FocusClaimFocus("ArdiaFocus.claimFocus", positional[]) { generation: Number, tag: Number },
    /// `platform/focus.ts: saveKeyResponder`.
    FocusSaveKeyResponder("ArdiaFocus.saveKeyResponder", positional[]) { window_id: String },
    /// `platform/focus.ts: restoreKeyResponder`.
    FocusRestoreKeyResponder("ArdiaFocus.restoreKeyResponder", positional[]) { token: Number },
    /// `platform/focus.ts: discardKeyResponder`.
    FocusDiscardKeyResponder("ArdiaFocus.discardKeyResponder", positional[]) { token: Number },
    /// `platform/focus.ts: pressRepeated`.
    FocusPressRepeated("ArdiaFocus.pressRepeated", positional[]) {  },
    /// `platform/focus.ts: pressesWithoutRelease`.
    FocusPressesWithoutRelease("ArdiaFocus.pressesWithoutRelease", positional[]) {  },
    /// `platform/focus.ts: switcherPointerMoved`.
    FocusSwitcherPointerMoved("ArdiaFocus.switcherPointerMoved", positional[]) {  },
    /// `platform/introMusic.ts: playIntroMusic`.
    AppPlayIntroMusic("ArdiaApp.playIntroMusic", positional[]) { cues: Value, muted: bool },
    /// `platform/introMusic.ts: setIntroMusicMuted`.
    AppSetIntroMusicMuted("ArdiaApp.setIntroMusicMuted", positional[]) { muted: bool },
    /// `platform/introMusic.ts: stopIntroMusic`.
    AppStopIntroMusic("ArdiaApp.stopIntroMusic", positional[]) { fade: Number },
    /// `platform/keychain.ts: keychainGet`.
    KeychainGet("ArdiaKeychain.get", positional[]) { account: String },
    /// `platform/keychain.ts: keychainRead`.
    KeychainRead("ArdiaKeychain.read", positional[]) { account: String },
    /// `platform/keychain.ts: keychainSet`.
    KeychainSet("ArdiaKeychain.set", positional[]) { account: String, secret: String },
    /// `platform/keychain.ts: keychainDelete`.
    KeychainDelete("ArdiaKeychain.delete", positional[]) { account: String },
    /// `platform/location.ts: locationPermission`.
    LocationLocationPermission("ArdiaLocation.locationPermission", positional[]) {  },
    /// `platform/location.ts: requestLocationPermission`.
    LocationRequestLocationPermission("ArdiaLocation.requestLocationPermission", positional[]) {  },
    /// `platform/location.ts: openLocationSettings`.
    LocationOpenLocationSettings("ArdiaLocation.openLocationSettings", positional[]) {  },
    /// `platform/media.ts: getDisplayMediaSources`.
    MediaDisplayMediaSources("media.displayMediaSources", object[]) {  },
    /// `platform/media.ts: getDisplayMediaThumbnails`.
    MediaDisplayMediaThumbnails("media.displayMediaThumbnails", object[]) { ids: Vec<String>, width: Number },
    /// `platform/media.ts: screenRecordingAllowed`.
    MediaScreenRecordingAllowed("media.screenRecordingAllowed", object[]) {  },
    /// `platform/media.ts: requestScreenRecording`.
    MediaRequestScreenRecording("media.requestScreenRecording", object[]) {  },
    /// `platform/media.ts: setDisplayMediaPicker`.
    WebViewSetDisplayMediaPicker("webView.setDisplayMediaPicker", object[]) { enabled: bool },
    /// `platform/media.ts: setAutoPictureInPicture`.
    WebViewSetAutoPictureInPicture("webView.setAutoPictureInPicture", object[]) { enabled: bool },
    /// `platform/media.ts: setVideoUpscaling`.
    WebViewSetVideoUpscaling("webView.setVideoUpscaling", object[]) { enabled: bool },
    /// `platform/media.ts: showCastDialog`.
    MediaShowCastDialog("media.showCastDialog", object[]) { tab_id: String },
    /// `platform/media.ts: startCasting`.
    MediaStartCasting("media.startCasting", object[]) { id: Number, sink: String, mode: Number },
    /// `platform/media.ts: stopCasting`.
    MediaStopCasting("media.stopCasting", object[]) { id: Number, route: String },
    /// `platform/media.ts: closeCastDialog`.
    MediaCloseCastDialog("media.closeCastDialog", object[]) { id: Number },
    /// `platform/media.ts: watchCastRoutes`.
    MediaWatchCastRoutes("media.watchCastRoutes", object[]) { profile: String },
    /// `platform/media.ts: terminateCastRoute`.
    MediaTerminateCastRoute("media.terminateCastRoute", object[]) { route: String },
    /// `platform/media.ts: changeCaptureSource`.
    MediaChangeCaptureSource("media.changeCaptureSource", object[]) { capturer: String, target: String },
    /// `platform/media.ts: stopCapture`.
    MediaStopCapture("media.stopCapture", object[]) { tab_id: String },
    /// `platform/notifications.ts: notificationPermission`.
    NotificationsNotificationPermission("ArdiaNotifications.notificationPermission", positional[]) {  },
    /// `platform/notifications.ts: requestNotificationPermission`.
    NotificationsRequestNotificationPermission("ArdiaNotifications.requestNotificationPermission", positional[]) {  },
    /// `platform/notifications.ts: postNotification`.
    NotificationsPostNotification("ArdiaNotifications.postNotification", positional[]) { options: Value },
    /// `platform/notifications.ts: removeNotifications`.
    NotificationsRemoveNotifications("ArdiaNotifications.removeNotifications", positional[]) { ids: Vec<String> },
    /// `platform/notifications.ts: openNotificationSettings`.
    NotificationsOpenNotificationSettings("ArdiaNotifications.openNotificationSettings", positional[]) {  },
    /// `platform/pageOptions.ts: setPeekModifier`.
    WebViewSetPeekModifier("webView.setPeekModifier", object[]) { modifier: String },
    /// `platform/pageOptions.ts: setSplitLinkModifier`.
    WebViewSetSplitLinkModifier("webView.setSplitLinkModifier", object[]) { modifier: String },
    /// `platform/pageOptions.ts: setPeekOtherSiteTabs`.
    WebViewSetPeekOtherSiteTabs("webView.setPeekOtherSiteTabs", object[]) { tab_ids: Vec<String> },
    /// `platform/pageOptions.ts: setSelectionPopover`.
    WebViewSetSelectionPopover("webView.setSelectionPopover", object[]) { enabled: bool },
    /// `platform/pageOptions.ts: setBlockAutoplayEverywhere`.
    WebViewSetBlockAutoplayEverywhere("webView.setBlockAutoplayEverywhere", object[]) { enabled: bool },
    /// `platform/pageOptions.ts: setSearchEngineName`.
    SearchSetEngineName("search.setEngineName", object[]) { name: Value, keyword: Value, url: Value },
    /// `platform/pageOptions.ts: setLittleArdiaOpensNewTabLinks`.
    WebViewSetLittleArdiaOpensNewTabLinks("webView.setLittleArdiaOpensNewTabLinks", object[]) { enabled: bool },
    /// `platform/pageOptions.ts: setPopupsOpenAsTabs`.
    WebViewSetPopupsOpenAsTabs("webView.setPopupsOpenAsTabs", object[]) { enabled: bool },
    /// `platform/passwordProviders.ts: setFieldProviders`.
    PasswordsSetFieldProviders("passwords.setFieldProviders", object[]) { generation: Number, profiles: Value, session: Value },
    /// `platform/passwordProviders.ts: showSystemPanel`.
    PasswordsShowSystemPanel("passwords.showSystemPanel", object[]) { tab: String, generation: Number },
    /// `platform/passwordProviders.ts: showPanelHint`.
    PasswordsShowPanelHint("passwords.showPanelHint", object[]) { tab: String, label: String, side: String },
    /// `platform/passwordProviders.ts: systemPanelAvailable`.
    PasswordsSystemPanelAvailable("passwords.systemPanelAvailable", object[]) {  },
    /// `platform/passwordProviders.ts: otpauthHandler`.
    PasswordsOtpauthHandler("passwords.otpauthHandler", object[]) {  },
    /// `platform/passwordProviders.ts: setUpVerificationCode`.
    PasswordsSetUpVerificationCode("passwords.setUpVerificationCode", object[]) { tab: String, token: String, generation: Number },
    /// `platform/passwords.ts: listPasswords`.
    PasswordsList("passwords.list", object[]) { profile: String },
    /// `platform/passwords.ts: unlockPasswords`.
    PasswordsUnlock("passwords.unlock", object[]) { profile: String },
    /// `platform/passwords.ts: getPassword`.
    PasswordsReveal("passwords.reveal", object[]) { profile: String, origin: String, username: String },
    /// `platform/passwords.ts: deletePassword`.
    PasswordsRemove("passwords.remove", object[]) { profile: String, origin: String, username: String },
    /// `platform/passwords.ts: setNeverSavePasswords`.
    PasswordsAllowSaving("passwords.allowSaving", object[]) { profile: String, origin: String },
    /// `platform/passwords.ts: getNeverSavePasswordOrigins`.
    PasswordsNeverSaveOrigins("passwords.neverSaveOrigins", object[]) { profile: String },
    /// `platform/passwords.ts: exportPasswords`.
    PasswordsExport("passwords.export", object[]) { profile: String },
    /// `platform/passwords.ts: getPasswordAutofill`.
    PasswordsAutofillEnabled("passwords.autofillEnabled", object[]) { profile: String },
    /// `platform/passwords.ts: setPasswordAutofill`.
    PasswordsSetAutofillEnabled("passwords.setAutofillEnabled", object[]) { profile: String, enabled: bool },
    /// `platform/performance.ts: listTasks`.
    EngineTasks("engine.tasks", object[]) {  },
    /// `platform/performance.ts: systemState`.
    EngineSystemState("engine.systemState", object[]) {  },
    /// `platform/performance.ts: setBatterySaver`.
    EngineBatterySaver("engine.batterySaver", object[]) { mode: Value },
    /// `platform/performance.ts: setMemorySaver`.
    EngineMemorySaver("engine.memorySaver", object["request"]) { request: Value },
    /// `platform/performance.ts: isKeepActiveSiteValid`.
    EngineValidateKeepActiveSite("engine.validateKeepActiveSite", object[]) { site: String },
    /// `platform/performance.ts: trimEngineMemory`.
    EngineTrimMemory("engine.trimMemory", object[]) {  },
    /// `platform/performance.ts: beginTracing`.
    EngineBeginTracing("engine.beginTracing", object[]) {  },
    /// `platform/performance.ts: endTracing`.
    EngineEndTracing("engine.endTracing", object[]) { keep: bool },
    /// `platform/performance.ts: isTracing`.
    EngineIsTracing("engine.isTracing", object[]) {  },
    /// `platform/performance.ts: openTaskManager`.
    EngineOpenTaskManager("engine.openTaskManager", object[]) {  },
    /// `platform/permissions.ts: resolvePermission`.
    PermissionsResolve("permissions.resolve", object[]) { id: String, result: Value, remember: bool },
    /// `platform/permissions.ts: selectDevice`.
    MediaSelectDevice("media.selectDevice", object[]) { id: Number, index: Number },
    /// `platform/permissions.ts: cancelDeviceChooser`.
    MediaCancelDeviceChooser("media.cancelDeviceChooser", object[]) { id: Number },
    /// `platform/permissions.ts: refreshDeviceChooser`.
    MediaRefreshDeviceChooser("media.refreshDeviceChooser", object[]) { id: Number },
    /// `platform/permissions.ts: openBluetoothSettings`.
    MediaOpenBluetoothSettings("media.openBluetoothSettings", object[]) { id: Number },
    /// `platform/plugins.ts: pluginsFolder`.
    PluginsFolder("ArdiaPlugins.folder", positional[]) {  },
    /// `platform/plugins.ts: listPluginFolders`.
    PluginsList("ArdiaPlugins.list", positional[]) {  },
    /// `platform/plugins.ts: startPluginProcess`.
    PluginsStart("ArdiaPlugins.start", positional[]) { spec: Value },
    /// `platform/plugins.ts: stopPluginProcess`.
    PluginsStop("ArdiaPlugins.stop", positional[]) { name: String, run: Number, grace_ms: Number },
    /// `platform/plugins.ts: replyToPlugin`.
    PluginsReply("ArdiaPlugins.reply", positional[]) { name: String, run: Number, line: String },
    /// `platform/plugins.ts: sendToPlugin`.
    PluginsEvent("ArdiaPlugins.event", positional[]) { name: String, run: Number, line: String },
    /// `platform/plugins.ts: callPlugin`.
    PluginsCall("ArdiaPlugins.call", positional[]) { name: String, run: Number, line: String },
    /// `platform/plugins.ts: removePluginFiles`.
    PluginsRemove("ArdiaPlugins.remove", positional[]) { name: String, official: bool, folder_id: Option<String> },
    /// `platform/plugins.ts: linkPluginPrograms`.
    PluginsLinkPrograms("ArdiaPlugins.linkPrograms", positional[]) { spec: Value },
    /// `platform/plugins.ts: unlinkPluginPrograms`.
    PluginsUnlinkPrograms("ArdiaPlugins.unlinkPrograms", positional[]) { name: String, programs: Vec<String>, remove: bool },
    /// `platform/plugins.ts: sweepPluginPrograms`.
    PluginsSweepPrograms("ArdiaPlugins.sweepPrograms", positional[]) { active: Vec<String> },
    /// `platform/plugins.ts: takeAfterUpdateQuitStops`.
    PluginsTakeAfterUpdateQuitStops("ArdiaPlugins.takeAfterUpdateQuitStops", positional[]) {  },
    /// `platform/plugins.ts: setAfterUpdateQuitConsent`.
    PluginsSetAfterUpdateQuitConsent("ArdiaPlugins.setAfterUpdateQuitConsent", positional[]) { consent: String },
    /// `platform/plugins.ts: legacyProgramsInstalled`.
    PluginsLegacyProgramsInstalled("ArdiaPlugins.legacyProgramsInstalled", positional[]) {  },
    /// `platform/plugins.ts: programRegistry`.
    PluginsProgramRegistry("ArdiaPlugins.programRegistry", positional[]) {  },
    /// `platform/plugins.ts: revealPluginLog`.
    PluginsRevealLog("ArdiaPlugins.revealLog", positional[]) { name: String },
    /// `platform/plugins.ts: revealPluginsFolder`.
    PluginsRevealFolder("ArdiaPlugins.revealFolder", positional[]) {  },
    /// `platform/plugins.ts: devSnapshotPluginPage`.
    PluginsDevSnapshotPage("ArdiaPlugins.devSnapshotPage", positional[]) { name: String, path: String },
    /// `platform/plugins.ts: officialPluginsPinned`.
    PluginsOfficialPlugins("ArdiaPlugins.officialPlugins", positional[]) {  },
    /// `platform/plugins.ts: fetchOfficialPlugin`.
    PluginsFetchOfficial("ArdiaPlugins.fetchOfficial", positional[]) { name: String },
    /// `platform/plugins.ts: cancelOfficialPluginFetch`.
    PluginsCancelOfficialFetch("ArdiaPlugins.cancelOfficialFetch", positional[]) { name: String },
    /// `platform/plugins.ts: promoteOfficialPlugin`.
    PluginsPromoteOfficial("ArdiaPlugins.promoteOfficial", positional[]) { name: String, folder_id: String, approved: String },
    /// `platform/plugins.ts: hostFetchPlugin`.
    PluginsHostFetch("ArdiaPlugins.hostFetch", positional[]) { spec: Value },
    /// `platform/plugins.ts: readPluginStorage`.
    PluginsStorageRead("ArdiaPlugins.storageRead", positional[]) { name: String },
    /// `platform/plugins.ts: writePluginStorage`.
    PluginsStorageWrite("ArdiaPlugins.storageWrite", positional[]) { name: String, text: String },
    /// `platform/plugins.ts: readPluginFile`.
    PluginsReadPluginFile("ArdiaPlugins.readPluginFile", positional[]) { name: String, folder_id: String, approved: String, path: String },
    /// `platform/profiles.ts: clearBrowsingData`.
    ProfilesClearBrowsingData("profiles.clearBrowsingData", object[]) { profile: String, types: Value, since: Number },
    /// `platform/profiles.ts: releaseProfile`.
    ProfilesRelease("profiles.release", object[]) { profile: String },
    /// `platform/profiles.ts: unloadIdleProfile`.
    ProfilesUnloadIdle("profiles.unloadIdle", object[]) { profile: String },
    /// `platform/profiles.ts: deleteProfileData`.
    ProfilesDeleteData("profiles.deleteData", object[]) { profile: String },
    /// `platform/profiles.ts: chromeCtrlTabMru`.
    ProfilesCtrlTabMru("profiles.ctrlTabMru", object[]) {  },
    /// `platform/profiles.ts: loadedProfiles`.
    ProfilesLoaded("profiles.loaded", object[]) {  },
    /// `platform/report.ts: draftDirectory`.
    ReportDraftDirectory("ArdiaReport.draftDirectory", positional[]) {  },
    /// `platform/report.ts: fileSize`.
    ReportFileSize("ArdiaReport.fileSize", positional[]) { path: String },
    /// `platform/report.ts: removeReportFile`.
    ReportRemoveFile("ArdiaReport.removeFile", positional[]) { path: String },
    /// `platform/report.ts: startWindowRecording`.
    ReportStartRecording("ArdiaReport.startRecording", positional[]) { window_id: String, include_pages: bool, path: String, max_seconds: Number, fps: Option<Number>, max_pixel_width: Option<Number> },
    /// `platform/report.ts: stopWindowRecording`.
    ReportStopRecording("ArdiaReport.stopRecording", positional[]) {  },
    /// `platform/report.ts: cancelWindowRecording`.
    ReportCancelRecording("ArdiaReport.cancelRecording", positional[]) {  },
    /// `platform/report.ts: recordingThumbnail`.
    ReportRecordingThumbnail("ArdiaReport.recordingThumbnail", positional[]) { path: String, out: String },
    /// `platform/report.ts: gzipFile`.
    ReportGzipFile("ArdiaReport.gzipFile", positional[]) { source: String, destination: String, remove_source: bool },
    /// `platform/report.ts: gzipText`.
    ReportGzipText("ArdiaReport.gzipText", positional[]) { text: String, destination: String },
    /// `platform/report.ts: uploadFile`.
    ReportUpload("ArdiaReport.upload", positional[]) { id: String, url: String, method: String, headers: Value, path: String },
    /// `platform/report.ts: cancelUploads`.
    ReportCancelUploads("ArdiaReport.cancelUploads", positional[]) {  },
    /// `platform/scripting.ts: setScriptState`.
    ScriptingSetScriptState("ArdiaScripting.setScriptState", positional[]) { state: Value },
    /// `platform/scripting.ts: replyToScript`.
    ScriptingReplyToScript("ArdiaScripting.replyToScript", positional[]) { id: String, result: Value, error: Option<String> },
    /// `platform/search.ts: fetchSearchSuggestions`.
    SearchSuggest("search.suggest", object[]) { profile: String, url: String, id: Number },
    /// `platform/search.ts: stopSearchSuggestions`.
    SearchStopSuggest("search.stopSuggest", object[]) { profile: String, id: Number },
    /// `platform/share.ts: sharePage`.
    AppShare("ArdiaApp.share", positional[]) { url: String, title: Option<String>, window_id: Option<String>, anchor: Value },
    /// `platform/share.ts: shareText`.
    AppShareText("ArdiaApp.shareText", positional[]) { text: String, window_id: Option<String>, anchor: Value },
    /// `platform/share.ts: sharePageVia`.
    AppShareVia("ArdiaApp.shareVia", positional[]) { service: String, url: String, title: Option<String>, window_id: Option<String> },
    /// `platform/siteSettings.ts: setSiteSetting`.
    SitesSetSetting("sites.setSetting", object[]) { profile: String, origin: String, r#type: Value, value: Value, engine: Value },
    /// `platform/siteSettings.ts: getSiteSettings`.
    SitesSettings("sites.settings", object[]) { profile: String, origin: String, engine: Value },
    /// `platform/siteSettings.ts: getSiteSettingsOrigins`.
    SitesOrigins("sites.origins", object[]) { profile: String },
    /// `platform/siteSettings.ts: resetSiteSettings`.
    SitesReset("sites.reset", object[]) { profile: String, origin: String },
    /// `platform/siteSettings.ts: clearSiteData`.
    SitesClearData("sites.clearData", object[]) { profile: String, origin: String, engine: Value },
    /// `platform/siteSettings.ts: setZoom`.
    SitesSetZoom("sites.setZoom", object[]) { profile: String, host: String, zoom: Number },
    /// `platform/siteSettings.ts: getZoomLevels`.
    SitesZoomLevels("sites.zoomLevels", object[]) { profile: String },
    /// `platform/siteSettings.ts: getPreloadPages`.
    SitesPreloadPages("sites.preloadPages", object[]) { profile: String },
    /// `platform/siteSettings.ts: setPreloadPages`.
    SitesSetPreloadPages("sites.setPreloadPages", object[]) { profile: String, on: bool },
    /// `platform/siteSettings.ts: setCommandScrollZoom`.
    WebViewSetCommandScrollZoom("webView.setCommandScrollZoom", object[]) { enabled: bool },
    /// `platform/siteSettings.ts: resolveExternalApp`.
    PermissionsResolveExternalApp("permissions.resolveExternalApp", object[]) { id: String, open: bool, remember: bool },
    /// `platform/siteSettings.ts: getExternalAppAllowances`.
    PermissionsExternalAppAllowances("permissions.externalAppAllowances", object[]) { profile: String },
    /// `platform/siteSettings.ts: removeExternalAppAllowance`.
    PermissionsRemoveExternalAppAllowance("permissions.removeExternalAppAllowance", object[]) { profile: String, origin: String, scheme: String },
    /// `platform/siteSettings.ts: registrableDomainsOf`.
    SitesRegistrableDomains("sites.registrableDomains", object[]) { profile: String, hosts: Vec<String> },
    /// `platform/siteSettings.ts: readChromiumStorage`.
    SitesReadStorage("sites.readStorage", object[]) { profile: String, origin: String },
    /// `platform/siteSettings.ts: writeChromiumStorage`.
    SitesWriteStorage("sites.writeStorage", object[]) { profile: String, origin: String, set: Value, remove: Vec<String> },
    /// `platform/surfaces.ts: openSurfaces`.
    MenusOpenSurfaces("ArdiaMenus.openSurfaces", positional[]) {  },
    /// `platform/surfaces.ts: capturePageMenus`.
    MenusCapturePageMenus("ArdiaMenus.capturePageMenus", positional[]) { captures: bool },
    /// `platform/surfaces.ts: dismissSurface`.
    MenusDismissSurface("ArdiaMenus.dismissSurface", positional[]) { id: String },
    /// `platform/surfaces.ts: menuBar`.
    MenusMenuBar("ArdiaMenus.menuBar", positional[]) {  },
    /// `platform/surfaces.ts: performMenuBarItem`.
    MenusPerformMenuBarItem("ArdiaMenus.performMenuBarItem", positional[]) { r#ref: String, window_id: String },
    /// `platform/systemSignals.ts: onUserIdle`.
    SystemSignalsWatchIdle("ArdiaSystemSignals.watchIdle", positional[]) { thresholds: Value },
    /// `platform/systemSignals.ts: setHoldIncomingLinks`.
    SystemSignalsSetHoldIncomingLinks("ArdiaSystemSignals.setHoldIncomingLinks", positional[]) { hold: bool, hooks_stamp: Option<String> },
    /// `platform/systemSignals.ts: setApprovedDownloadFolders`.
    SystemSignalsSetApprovedDownloadFolders("ArdiaSystemSignals.setApprovedDownloadFolders", positional[]) { folders: Vec<String> },
    /// `platform/systemSignals.ts: moveCompletedDownload`.
    SystemSignalsMoveCompletedDownload("ArdiaSystemSignals.moveCompletedDownload", positional[]) { path: String, folder: String },
    /// `platform/tabs.ts: prepareTabTransfer`.
    TabsPrepareTransfer("ArdiaTabs.prepareTransfer", positional[]) { tab_id: String },
    /// `platform/tabs.ts: releaseTabTransfer`.
    TabsReleaseTransfer("ArdiaTabs.releaseTransfer", positional[]) { tab_id: String },
    /// `platform/tabs.ts: claimLaunchTab`.
    TabsClaimLaunchTab("ArdiaTabs.claimLaunchTab", positional[]) { tab_id: String, url: String, profile: String },
    /// `platform/tabs.ts: revealTab`.
    TabsRevealTab("ArdiaTabs.revealTab", positional[]) { tab_id: String, press: Number },
    /// `platform/tabs.ts: dropTabReveals`.
    TabsDropTabReveals("ArdiaTabs.dropTabReveals", positional[]) { over_tab_id: String, press: Number, to_tab_id: String },
    /// `platform/tabs.ts: claimTabPress`.
    TabsClaimTabPress("ArdiaTabs.claimTabPress", positional[]) { tab_id: String, started_at: Number },
    /// `platform/tabs.ts: dropTabPress`.
    TabsDropTabPress("ArdiaTabs.dropTabPress", positional[]) { press: Number },
    /// `platform/tabs.ts: setTabsInSplits`.
    TabsSetTabsInSplits("ArdiaTabs.setTabsInSplits", positional[]) { tab_ids: Vec<String> },
    /// `platform/tabs.ts: setNativeTabSwitch`.
    TabsSetNativeTabSwitch("ArdiaTabs.setNativeTabSwitch", positional[]) { enabled: bool },
    /// `platform/tabs.ts: revealOffered`.
    TabsRevealOffered("ArdiaTabs.revealOffered", positional[]) { adopt_id: String, over_tab_id: String, press: Number },
    /// `platform/tabs.ts: discardOffered`.
    TabsDiscardOffered("ArdiaTabs.discardOffered", positional[]) { adopt_id: String },
    /// `platform/tabs.ts: routeLinkTabs`.
    TabsRouteLinkTabs("tabs.routeLinkTabs", object[]) { urls: Vec<String> },
    /// `platform/tabs.ts: tabStrips`.
    TabsStrips("tabs.strips", object[]) {  },
    /// `platform/tabs.ts: reportPageCommits`.
    TabsReportCommits("tabs.reportCommits", object[]) { on: bool },
    /// `platform/tabs.ts: lastTabClosesWindow`.
    TabsLastTabPolicy("tabs.lastTabPolicy", object[]) {  },
    /// `platform/telemetryLocal.ts: appendLocal`.
    TelemetryLocalAppend("ArdiaTelemetryLocal.append", positional[]) { day: String, text: String },
    /// `platform/telemetryLocal.ts: appendLocalNow`.
    TelemetryLocalAppendNow("ArdiaTelemetryLocal.appendNow", positional[]) { day: String, text: String },
    /// `platform/telemetryLocal.ts: configureLocal`.
    TelemetryLocalConfigure("ArdiaTelemetryLocal.configure", positional[]) { max_bytes: Number, keep_days: Number },
    /// `platform/telemetryLocal.ts: pruneLocal`.
    TelemetryLocalPrune("ArdiaTelemetryLocal.prune", positional[]) { now: Number },
    /// `platform/telemetryLocal.ts: clearLocal`.
    TelemetryLocalClear("ArdiaTelemetryLocal.clear", positional[]) {  },
    /// `platform/telemetryLocal.ts: exportLocalForReport`.
    TelemetryLocalExportForReport("ArdiaTelemetryLocal.exportForReport", positional[]) { since: Number },
    /// `platform/telemetryLocal.ts: exportLocalToDownloads`.
    TelemetryLocalExportToDownloads("ArdiaTelemetryLocal.exportToDownloads", positional[]) { now: Number },
    /// `platform/userScripts.ts: watchUserScripts`.
    UserScriptsWatch("ArdiaUserScripts.watch", positional[]) { on: bool },
    /// `platform/userScripts.ts: readUserScripts`.
    UserScriptsRead("ArdiaUserScripts.read", positional[]) {  },
    /// `platform/userScripts.ts: readUserScriptsStatus`.
    UserScriptsReadStatus("ArdiaUserScripts.readStatus", positional[]) {  },
    /// `platform/userScripts.ts: writeUserScriptsStatus`.
    UserScriptsWriteStatus("ArdiaUserScripts.writeStatus", positional[]) { json: String },
    /// `platform/webkit.ts: endWebKitPrivateSession`.
    WebKitEndPrivateSession("webKit.endPrivateSession", object[]) { session: String },
    /// `platform/webkit.ts: webKitCookieStoreKey`.
    WebKitCookieStoreKey("webKit.cookieStoreKey", object[]) { profile: String },
    /// `platform/webkit.ts: importWebKitCookies`.
    WebKitImportCookies("webKit.importCookies", object[]) { profile: String, store_key: String, cookies: Value },
    /// `platform/webkit.ts: exportWebKitCookies`.
    WebKitExportCookies("webKit.exportCookies", object["extra"]) { profile: String, urls: Value, extra: Value },
    /// `platform/webkit.ts: setWebKitFraudWarnings`.
    WebKitSetFraudWarnings("webKit.setFraudWarnings", object[]) { enabled: bool },
    /// `platform/webkit.ts: watchWebKitSiteData`.
    WebKitWatchSiteData("webKit.watchSiteData", object[]) { enabled: bool },
    /// `platform/webkit.ts: releaseWebKitSiteDataGate`.
    WebKitSiteDataReady("webKit.siteDataReady", object[]) { id: String },
    /// `platform/webkit.ts: readWebKitSiteData`.
    WebKitReadSiteData("webKit.readSiteData", object[]) { profile: String, domains: Vec<String>, origins: Vec<String> },
    /// `platform/webkit.ts: writeWebKitSiteData`.
    WebKitWriteSiteData("webKit.writeSiteData", object[]) { profile: String, set: Value, remove: Value, storage: Value },
    /// `platform/webkit.ts: setWebKitZoom`.
    WebKitSetZoom("webKit.setZoom", object[]) { profile: String, host: String, zoom: Number },
    /// `platform/webkit.ts: webKitNotificationAction`.
    WebKitNotificationAction("webKit.notificationAction", object[]) { id: String, action: String },
    /// `platform/webkit.ts: getPasskeyAccess`.
    WebKitPasskeyAccess("webKit.passkeyAccess", object[]) {  },
    /// `platform/webkit.ts: requestPasskeyAccess`.
    WebKitRequestPasskeyAccess("webKit.requestPasskeyAccess", object[]) {  },
    /// `platform/windows.ts: windowCornerRadius`.
    AppWindowsWindowCornerRadius("ArdiaAppWindows.windowCornerRadius", positional[]) {  },
    /// `platform/windows.ts: drawnWindowCornerRadius`.
    AppWindowsDrawnWindowCornerRadius("ArdiaAppWindows.drawnWindowCornerRadius", positional[]) {  },
    /// `platform/windows.ts: openWindow`.
    AppWindowsOpenWindow("ArdiaAppWindows.openWindow", positional[]) { id: String, options: Value },
    /// `platform/windows.ts: closeWindow`.
    AppWindowsCloseWindow("ArdiaAppWindows.closeWindow", positional[]) { id: String },
    /// `platform/windows.ts: animateWindowClose`.
    AppWindowsAnimateWindowClose("ArdiaAppWindows.animateWindowClose", positional[]) { id: String, after: Option<String> },
    /// `platform/windows.ts: restoreFullScreen`.
    AppWindowsRestoreFullScreen("ArdiaAppWindows.restoreFullScreen", positional[]) { ids: Vec<String>, focus: Option<String> },
    /// `platform/windows.ts: focusWindow`.
    AppWindowsFocusWindow("ArdiaAppWindows.focusWindow", positional[]) { id: String },
    /// `platform/windows.ts: setWindowTitle`.
    AppWindowsSetWindowTitle("ArdiaAppWindows.setWindowTitle", positional[]) { id: String, title: String },
    /// `platform/windows.ts: setTrafficLightsCenter`.
    AppWindowsSetTrafficLightsCenter("ArdiaAppWindows.setTrafficLightsCenter", positional[]) { id: String, center: Value, right: bool },
    /// `platform/windows.ts: watchSidebarEdge`.
    AppWindowsWatchSidebarEdge("ArdiaAppWindows.watchSidebarEdge", positional[]) { id: String, on: bool, hold: Number, right: bool, while_pressed: bool },
    /// `platform/windows.ts: windowFrame`.
    AppWindowsWindowFrame("ArdiaAppWindows.windowFrame", positional[]) { id: String },
    /// `platform/windows.ts: frameCornerRadius`.
    AppWindowsFrameCornerRadius("ArdiaAppWindows.frameCornerRadius", positional[]) { id: String },
    /// `platform/windows.ts: windowIds`.
    AppWindowsWindowIds("ArdiaAppWindows.windowIds", positional[]) {  },
    /// `platform/windows.ts: windowGeometry`.
    AppWindowsWindowGeometry("ArdiaAppWindows.windowGeometry", positional[]) { id: String },
    /// `platform/windows.ts: setWindowFrame`.
    AppWindowsSetWindowFrame("ArdiaAppWindows.setWindowFrame", positional[]) { id: String, frame: [Number; 4] },
    /// `platform/windows.ts: setWindowZoomed`.
    AppWindowsSetWindowZoomed("ArdiaAppWindows.setWindowZoomed", positional[]) { id: String, on: bool },
    /// `platform/windows.ts: setWindowFullScreen`.
    AppWindowsSetWindowFullScreen("ArdiaAppWindows.setWindowFullScreen", positional[]) { id: String, on: bool },
    /// `platform/windows.ts: replyToTerminate`.
    AppWindowsReplyToTerminate("ArdiaAppWindows.replyToTerminate", positional[]) { ok: bool },
    /// `platform/windows.ts: raiseView`.
    AppWindowsRaiseView("ArdiaAppWindows.raiseView", positional[]) { tag: Option<Number>, levels: Number, raised: bool },
    /// `platform/windows.ts: dragPreview`.
    AppWindowsDragPreviewBegin("ArdiaAppWindows.dragPreviewBegin", positional[]) { window_id: Value, chip: Value, grab: Value },
    /// `platform/windows.ts: dragPreview`.
    AppWindowsDragPreviewPage("ArdiaAppWindows.dragPreviewPage", positional[]) { base64: Value, frame: Value },
    /// `platform/windows.ts: dragPreview`.
    AppWindowsDragPreviewUpdate("ArdiaAppWindows.dragPreviewUpdate", positional[]) { shape: Value, point: Value },
    /// `platform/windows.ts: dragPreview`.
    AppWindowsDragPreviewEnd("ArdiaAppWindows.dragPreviewEnd", positional[]) { window_id: Value, frame: Value },
    /// `platform/windows.ts: dragPreview`.
    AppWindowsDragPreviewCancel("ArdiaAppWindows.dragPreviewCancel", positional[]) {  },
    /// `platform/windows.ts: dragPreview`.
    AppWindowsDragPreviewPlaceholder("ArdiaAppWindows.dragPreviewPlaceholder", positional[]) { title: Value, favicon: Value },
    /// `platform/windows.ts: chromeWindows`.
    WindowsList("windows.list", object[]) {  },
    /// `platform/windows.ts: devWindowAction`.
    WindowsDev("windows.dev", object[]) { window: Number, action: String },
    /// `ardiacore history.ts:19`.
    HistoryQuery("history.query", object[]) { profile: String, max_urls: Number, max_visits: Number },
    /// `ardiacore history.ts:26`.
    HistoryAdd("history.add", object[]) { profile: String, pages: Value, tolerance_ms: Number },
    /// `ardiacore history.ts:64`.
    HistoryImport("history.import", object[]) { profile: String, rows: Value },
    /// `ardiacore history.ts:80`.
    HistoryDeleteUrls("history.deleteUrls", object[]) { profile: String, urls: Value },
    /// `ardiacore history.ts:83`.
    HistoryWatch("history.watch", object[]) { profile: String },
    /// `ardiacore favicons.ts:5`.
    FaviconsGet("favicons.get", object[]) { profile: String, pages: Vec<String>, icons: Vec<String>, size: Number },
    /// `ardiacore favicons.ts:14`.
    FaviconsSet("favicons.set", object[]) { profile: String, page: String, icon: String, png: String },
    /// `ardiacore favicons.ts:18`.
    FaviconsFetch("favicons.fetch", object[]) { profile: String, url: String },
    /// `ardiacore autofill.ts:34`.
    AutofillSaveCard("ArdiaAutofill.saveCard", positional[]) { profile: String, card: Value, number: Option<String> },
    /// `ardiacore passwords.ts:15`.
    PasswordsSave("ArdiaPasswords.save", positional[]) { profile: String, origin: String, username: String, password: String },
    /// `ardiacore passwords.ts:17`.
    PasswordsUpdate("ArdiaPasswords.update", positional[]) { profile: String, origin: String, username: String, username_change: Option<String>, password_change: Option<String> },
    /// `ardiacore tabs.ts:54`.
    TabsStripCommand("tabs.stripCommand", object[]) { id: Number, command: Value },
    /// `ardiacore engineDomain.ts:59`.
    EngineInputIdleSeconds("ArdiaEngine.inputIdleSeconds", positional[]) {  },
    /// `text.ts:5`.
    ShellStartDictation("ArdiaShell.startDictation", positional[]) {  },
    /// `text.ts:6`.
    ShellDictationAvailable("ArdiaShell.dictationAvailable", positional[]) {  },
    /// `documents.ts:10`.
    ShellFetchBangList("ArdiaShell.fetchBangList", positional[]) { etag: Option<String> },
    /// `launchConfig.ts:5`.
    AppLaunchConfig("ArdiaApp.launchConfig", positional[]) {  },
    /// `windows.ts:54`.
    AppSetWindowActivity("ArdiaApp.setWindowActivity", positional[]) { window_id: String, url: Option<String>, title: Option<String> },
    /// `shell inlineCompletion.ts:12`.
    InlineCompletionComplete("ArdiaInlineCompletion.complete", positional[]) { tag: Number, typed: String, completion: String },
    /// `shell inlineCompletion.ts:15`.
    InlineCompletionCopyFieldAs("ArdiaInlineCompletion.copyFieldAs", positional[]) { shown: Option<String>, url: Option<String> },
    /// `sharedFolders.ts:11`.
    SharedFoldersLocation("ArdiaSharedFolders.location", positional[]) {  },
    /// `sharedFolders.ts:12`.
    SharedFoldersRead("ArdiaSharedFolders.read", positional[]) { path: String, identity: Value },
    /// `sharedFolders.ts:13`.
    SharedFoldersApply("ArdiaSharedFolders.apply", positional[]) { path: String, ops: String, identity: Value },
    /// `sharedFolders.ts:15`.
    SharedFoldersResolve("ArdiaSharedFolders.resolve", positional[]) { path: String },
    /// `sharedFolders.ts:16`.
    SharedFoldersDiscard("ArdiaSharedFolders.discard", positional[]) { path: String, identity: Value },
    /// `sharedFolders.ts:17`.
    SharedFoldersCreate("ArdiaSharedFolders.create", positional[]) { parent: String, parent_identity: Value, name: String },
    /// `sharedFolders.ts:18`.
    SharedFoldersWatch("ArdiaSharedFolders.watch", positional[]) { paths: Vec<String> },
    /// `sharedFolders.ts:19`.
    SyncRevealFolder("ArdiaSync.revealFolder", positional[]) { path: String },
}
