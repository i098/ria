// Ported from ardia engine/ardiacore/src/ardia/core/public/ArdiaCore.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `objc2` bindings of the classes of `ArdiaCore.h`: `ArdiaCoreEngine`, `ArdiaCoreProfile`, `ArdiaCoreWindow`,
//! `ArdiaCoreTab` and `ArdiaCorePageView`. The classes live in the engine framework, so the framework must be
//! loaded (`dlopen`) before the first use of a class here.
//!
//! The delegate protocols `ArdiaCoreWindowDelegate` and `ArdiaCoreTabDelegate` are not here: the window slice and the
//! tab slice implement them, and they set the delegate with `set_delegate`.
//! All methods must run on the main thread. Each nullable result is an `Option`.

use std::ffi::{c_char, c_int, c_void};

use block2::DynBlock;
use block2::RcBlock;
use objc2::encode::{Encode, Encoding};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObject};
use objc2::{extern_class, extern_methods};
use objc2_app_kit::{NSColor, NSEvent, NSImage, NSView, NSWindow};
use objc2_foundation::{
    NSArray, NSData, NSDate, NSDictionary, NSNumber, NSRect, NSSize, NSString, NSTimeInterval,
};

/// `NSDictionary<NSString *, id>`: the shape of each payload of the engine.
pub type Payload = NSDictionary<NSString, AnyObject>;
/// A `CGImageRef` that the engine hands to a block. It is valid only inside the block.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CGImageRef(pub *mut c_void);

// SAFETY: `CGImageRef` is a pointer to the opaque struct `CGImage`.
unsafe impl Encode for CGImageRef {
    const ENCODING: Encoding = Encoding::Pointer(&Encoding::Struct("CGImage", &[]));
}

/// `ArdiaCoreAdoption`.
#[repr(isize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adoption {
    Refused = 0,
    Adopted = 1,
    Busy = 2,
}

// SAFETY: `ArdiaCoreAdoption` is an `NSInteger` enum.
unsafe impl Encode for Adoption {
    const ENCODING: Encoding = isize::ENCODING;
}

/// `ArdiaCorePageScrollAck`.
#[repr(isize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageScrollAck {
    Begin = 0,
    Update = 1,
}

// SAFETY: `ArdiaCorePageScrollAck` is an `NSInteger` enum.
unsafe impl Encode for PageScrollAck {
    const ENCODING: Encoding = isize::ENCODING;
}

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    #[name = "ArdiaCoreEngine"]
    pub struct Engine;
);

impl Engine {
    extern_methods!(
        /// `+runWithArgc:argv:delegate:`: runs Chromium and returns its exit code at quit.
        #[unsafe(method(runWithArgc:argv:delegate:))]
        pub unsafe fn run(argc: c_int, argv: *const *const c_char, delegate: &AnyObject) -> c_int;
        #[unsafe(method(sharedEngine))]
        pub fn shared() -> Option<Retained<Engine>>;

        #[unsafe(method(defaultProfile))]
        pub fn default_profile(&self) -> Retained<Profile>;
        #[unsafe(method(loadProfile:completion:))]
        fn load_profile_raw(
            &self,
            directory_name: &NSString,
            completion: &DynBlock<dyn Fn(*mut AnyObject)>,
        );
        #[unsafe(method(quit))]
        pub fn quit(&self);
        #[unsafe(method(setPageScript:))]
        pub fn set_page_script(&self, script: Option<&NSString>);
        #[unsafe(method(pageScript))]
        pub fn page_script(&self) -> Option<Retained<NSString>>;

        #[unsafe(method(resolvePermission:result:remember:))]
        pub fn resolve_permission(request_id: &NSString, result: &NSString, remember: bool);
        #[unsafe(method(resolveExtensionInstallPrompt:accepted:))]
        pub fn resolve_extension_install_prompt(request_id: &NSString, accepted: bool);
        #[unsafe(method(startCasting:sink:mode:))]
        pub fn start_casting(dialog_id: c_int, sink_id: &NSString, mode: c_int);
        #[unsafe(method(stopCasting:route:))]
        pub fn stop_casting(dialog_id: c_int, route_id: &NSString);
        #[unsafe(method(closeCastDialog:))]
        pub fn close_cast_dialog(dialog_id: c_int);
        #[unsafe(method(terminateCastRoute:))]
        pub fn terminate_cast_route(route_id: &NSString);
        #[unsafe(method(setAutoPictureInPicture:))]
        pub fn set_auto_picture_in_picture(enabled: bool);
        #[unsafe(method(setVideoUpscaling:))]
        pub fn set_video_upscaling(enabled: bool);
        #[unsafe(method(setVideoUpscalingTestConditions:))]
        pub fn set_video_upscaling_test_conditions(conditions: &NSDictionary<NSString, NSNumber>);
        #[unsafe(method(videoUpscalingState))]
        pub fn video_upscaling_state() -> Retained<Payload>;
        #[unsafe(method(reservedShortcutsChanged))]
        pub fn reserved_shortcuts_changed();
        #[unsafe(method(selectDevice:index:))]
        pub fn select_device(chooser_id: c_int, index: c_int);
        #[unsafe(method(cancelDeviceChooser:))]
        pub fn cancel_device_chooser(chooser_id: c_int);
        #[unsafe(method(refreshDeviceChooser:))]
        pub fn refresh_device_chooser(chooser_id: c_int);
        #[unsafe(method(openDeviceChooserSettings:))]
        pub fn open_device_chooser_settings(chooser_id: c_int);
        #[unsafe(method(deleteProfile:completion:))]
        pub fn delete_profile(
            &self,
            profile: &Profile,
            completion: Option<&DynBlock<dyn Fn(Bool)>>,
        );
        #[unsafe(method(releaseProfile:))]
        pub fn release_profile(&self, profile: &Profile);
        #[unsafe(method(resolveExternalApp:open:remember:))]
        pub fn resolve_external_app(request_id: &NSString, open: bool, remember: bool);
        #[unsafe(method(testExternalLaunches))]
        pub fn test_external_launches() -> Retained<NSArray<NSDictionary>>;
        #[unsafe(method(testChooserEvents))]
        pub fn test_chooser_events() -> Retained<NSArray<NSDictionary>>;
        #[unsafe(method(allowDesktopCapture:tab:frame:origin:audio:))]
        pub fn allow_desktop_capture(
            source_id: &NSString,
            tab: &Tab,
            frame_id: Option<&NSString>,
            origin: Option<&NSString>,
            audio: Option<&NSString>,
        );
        #[unsafe(method(desktopCaptureAudio))]
        pub fn desktop_capture_audio() -> Retained<NSDictionary<NSString, NSNumber>>;
        #[unsafe(method(pagesPickDisplayMedia))]
        pub fn pages_pick_display_media() -> bool;
        #[unsafe(method(setPagesPickDisplayMedia:))]
        pub fn set_pages_pick_display_media(on: bool);
        #[unsafe(method(offTheRecordProfileFor:completion:))]
        fn off_the_record_profile_for_raw(
            &self,
            profile: &Profile,
            completion: &DynBlock<dyn Fn(*mut AnyObject, *mut AnyObject)>,
        );
        #[unsafe(method(chromiumVersion))]
        pub fn chromium_version(&self) -> Retained<NSString>;
        #[unsafe(method(keepAliveState))]
        pub fn keep_alive_state(&self) -> Retained<NSString>;
        #[unsafe(method(pictureInPictureCloses))]
        pub fn picture_in_picture_closes(&self) -> Retained<NSString>;

        // Category `IdleProfiles`.
        #[unsafe(method(sleepIdleProfile:hostedTabs:))]
        pub fn sleep_idle_profile(
            &self,
            profile: &Profile,
            hosted_tabs: &NSArray<Tab>,
        ) -> Option<Retained<NSString>>;
        #[unsafe(method(unloadIdleProfile:))]
        pub fn unload_idle_profile(&self, profile: &Profile) -> Retained<NSString>;

        // Category `Tracing`.
        #[unsafe(method(beginTracing:))]
        pub fn begin_tracing(completion: &DynBlock<dyn Fn(Bool)>);
        #[unsafe(method(endTracing:completion:))]
        fn end_tracing_raw(keep: bool, completion: &DynBlock<dyn Fn(*mut AnyObject)>);
        #[unsafe(method(isTracing))]
        pub fn is_tracing() -> bool;
    );
}

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    #[name = "ArdiaCoreProfile"]
    pub struct Profile;
);

impl Profile {
    extern_methods!(
        #[unsafe(method(name))]
        pub fn name(&self) -> Retained<NSString>;
        #[unsafe(method(path))]
        pub fn path(&self) -> Retained<NSString>;
        #[unsafe(method(offTheRecord))]
        pub fn off_the_record(&self) -> bool;
        #[unsafe(method(destroyed))]
        pub fn destroyed(&self) -> bool;
        #[unsafe(method(loadComponentExtension:))]
        pub fn load_component_extension(&self, path: &NSString) -> Option<Retained<NSString>>;
        #[unsafe(method(unloadComponentExtension:))]
        pub fn unload_component_extension(&self, extension_id: &NSString);
        #[unsafe(method(warmSpareRenderer))]
        pub fn warm_spare_renderer(&self);
        #[unsafe(method(holdNavigationsUntilRulesetsOf:))]
        pub fn hold_navigations_until_rulesets_of(&self, extension_id: &NSString);
        #[unsafe(method(releaseNavigationHold:))]
        pub fn release_navigation_hold(&self, reason: &NSString);
        #[unsafe(method(releaseNavigationHoldReady))]
        pub fn release_navigation_hold_ready(&self);
        #[unsafe(method(watchCastRoutes))]
        pub fn watch_cast_routes(&self);
        #[unsafe(method(passwordsOffered))]
        pub fn passwords_offered(&self) -> bool;
        #[unsafe(method(setPasswordsOffered:))]
        pub fn set_passwords_offered(&self, on: bool);
        #[unsafe(method(addressesAutofilled))]
        pub fn addresses_autofilled(&self) -> bool;
        #[unsafe(method(setAddressesAutofilled:))]
        pub fn set_addresses_autofilled(&self, on: bool);
        #[unsafe(method(cardsAutofilled))]
        pub fn cards_autofilled(&self) -> bool;
        #[unsafe(method(setCardsAutofilled:))]
        pub fn set_cards_autofilled(&self, on: bool);
        #[unsafe(method(preloadPages))]
        pub fn preload_pages(&self) -> bool;
        #[unsafe(method(setPreloadPages:))]
        pub fn set_preload_pages(&self, on: bool);
        #[unsafe(method(clearBrowsingData:since:completion:))]
        pub fn clear_browsing_data(
            &self,
            types: &NSArray<NSString>,
            since: Option<&NSDate>,
            completion: Option<&DynBlock<dyn Fn()>>,
        );

        // Category `Icons`.
        #[unsafe(method(fetchIcon:maxBytes:completion:))]
        fn fetch_icon_raw(
            &self,
            url: &NSString,
            max_bytes: usize,
            completion: &DynBlock<dyn Fn(*mut AnyObject)>,
        );
    );
}

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    #[name = "ArdiaCoreWindow"]
    pub struct Window;
);

impl Window {
    extern_methods!(
        #[unsafe(method(embeddedRootOfWindow:))]
        pub fn embedded_root_of(window: &NSWindow) -> Option<Retained<NSView>>;
        #[unsafe(method(contentViewClass))]
        pub fn content_view_class() -> &'static objc2::runtime::AnyClass;
        #[unsafe(method(isChromeFramelessWindow:))]
        pub fn is_chrome_frameless_window(window: &NSWindow) -> bool;
        #[unsafe(method(initWithContentRect:))]
        pub unsafe fn init_with_content_rect(
            this: objc2::rc::Allocated<Self>,
            rect: NSRect,
        ) -> Retained<Self>;
        #[unsafe(method(window))]
        pub fn window(&self) -> Retained<NSWindow>;
        #[unsafe(method(hostView))]
        pub fn host_view(&self) -> Retained<NSView>;
        /// The delegate is weak; it must conform to `ArdiaCoreWindowDelegate`.
        #[unsafe(method(setDelegate:))]
        pub unsafe fn set_delegate(&self, delegate: Option<&AnyObject>);

        #[unsafe(method(openTab:profile:foreground:))]
        pub fn open_tab(
            &self,
            url: &NSString,
            profile: &Profile,
            foreground: bool,
        ) -> Retained<Tab>;
        #[unsafe(method(openExtensionView:profile:kind:))]
        pub fn open_extension_view(
            &self,
            url: &NSString,
            profile: &Profile,
            kind: &NSString,
        ) -> Option<Retained<Tab>>;
        #[unsafe(method(activeProfile))]
        pub fn active_profile(&self) -> Option<Retained<Profile>>;
        #[unsafe(method(setActiveProfile:))]
        pub fn set_active_profile(&self, profile: Option<&Profile>);
        #[unsafe(method(activateTab:))]
        pub fn activate_tab(&self, tab: &Tab);
        #[unsafe(method(tabsForProfile:))]
        pub fn tabs_for_profile(&self, profile: &Profile) -> Retained<NSArray<Tab>>;
        #[unsafe(method(chromeWindowIdForProfile:))]
        pub fn chrome_window_id_for_profile(&self, profile: &Profile) -> c_int;
        #[unsafe(method(executeChromeCommand:profile:))]
        pub fn execute_chrome_command(&self, command_id: c_int, profile: &Profile) -> bool;
        #[unsafe(method(showInactive))]
        pub fn show_inactive(&self);
        #[unsafe(method(actedFullScreen))]
        pub fn acted_full_screen(&self) -> bool;
        #[unsafe(method(setActedFullScreen:))]
        pub fn set_acted_full_screen(&self, on: bool);
        #[unsafe(method(close))]
        pub fn close(&self);

        #[unsafe(method(prepareProfile:))]
        pub fn prepare_profile(&self, profile: &Profile);
        #[unsafe(method(adoptTab:))]
        pub fn adopt_tab(&self, tab: &Tab) -> Adoption;
        #[unsafe(method(canAdoptTab:))]
        pub fn can_adopt_tab(&self, tab: &Tab) -> Adoption;
        #[unsafe(method(restoreTab:profile:foreground:))]
        pub fn restore_tab(
            &self,
            state: Option<&NSString>,
            profile: &Profile,
            foreground: bool,
        ) -> Option<Retained<Tab>>;
        #[unsafe(method(duplicateTab:profile:foreground:))]
        pub fn duplicate_tab(
            &self,
            source: &Tab,
            profile: &Profile,
            foreground: bool,
        ) -> Option<Retained<Tab>>;
        #[unsafe(method(placeTab:index:pinned:))]
        pub fn place_tab(&self, tab: &Tab, index: c_int, pinned: bool);
        #[unsafe(method(internal))]
        pub fn internal(&self) -> bool;
        #[unsafe(method(setInternal:))]
        pub fn set_internal(&self, on: bool);
        #[unsafe(method(automation))]
        pub fn automation(&self) -> bool;
        #[unsafe(method(setAutomation:))]
        pub fn set_automation(&self, on: bool);

        // Category `Picture`.
        #[unsafe(method(copyPictureOfChromeWindow:completion:))]
        pub fn copy_picture_of_chrome_window(
            window: &NSWindow,
            completion: &DynBlock<dyn Fn(CGImageRef)>,
        );
    );
}

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    #[name = "ArdiaCoreTab"]
    pub struct Tab;
);

impl Tab {
    extern_methods!(
        #[unsafe(method(view))]
        pub fn view(&self) -> Option<Retained<NSView>>;
        #[unsafe(method(profile))]
        pub fn profile(&self) -> Option<Retained<Profile>>;
        #[unsafe(method(tabId))]
        pub fn tab_id(&self) -> c_int;
        #[unsafe(method(url))]
        pub fn url(&self) -> Retained<NSString>;
        #[unsafe(method(title))]
        pub fn title(&self) -> Retained<NSString>;
        #[unsafe(method(loading))]
        pub fn loading(&self) -> bool;
        #[unsafe(method(progress))]
        pub fn progress(&self) -> f64;
        #[unsafe(method(favicon))]
        pub fn favicon(&self) -> Option<Retained<NSImage>>;
        #[unsafe(method(canGoBack))]
        pub fn can_go_back(&self) -> bool;
        #[unsafe(method(canGoForward))]
        pub fn can_go_forward(&self) -> bool;
        #[unsafe(method(closed))]
        pub fn closed(&self) -> bool;
        /// The delegate is weak; it must conform to `ArdiaCoreTabDelegate`.
        #[unsafe(method(setDelegate:))]
        pub unsafe fn set_delegate(&self, delegate: Option<&AnyObject>);

        #[unsafe(method(focus))]
        pub fn focus(&self);
        #[unsafe(method(find:forward:))]
        pub fn find(&self, text: &NSString, forward: bool);
        #[unsafe(method(stopFinding))]
        pub fn stop_finding(&self);
        #[unsafe(method(close))]
        pub fn close(&self);
        #[unsafe(method(passwordPrompt))]
        pub fn password_prompt(&self) -> Option<Retained<Payload>>;
        #[unsafe(method(resolvePasswordPrompt:username:password:))]
        pub fn resolve_password_prompt(
            &self,
            action: &NSString,
            username: Option<&NSString>,
            password: Option<&NSString>,
        );
        #[unsafe(method(autofillPrompt))]
        pub fn autofill_prompt(&self) -> Option<Retained<Payload>>;
        #[unsafe(method(resolveAutofillPrompt:action:))]
        pub fn resolve_autofill_prompt(&self, prompt_id: isize, action: &NSString);

        #[unsafe(method(faviconURL))]
        pub fn favicon_url(&self) -> Option<Retained<NSString>>;
        #[unsafe(method(themeColor))]
        pub fn theme_color(&self) -> Option<Retained<NSString>>;
        #[unsafe(method(audible))]
        pub fn audible(&self) -> bool;
        #[unsafe(method(muted))]
        pub fn muted(&self) -> bool;
        #[unsafe(method(setMuted:))]
        pub fn set_muted(&self, on: bool);
        #[unsafe(method(browserId))]
        pub fn browser_id(&self) -> c_int;
        #[unsafe(method(committedOrigin))]
        pub fn committed_origin(&self) -> Option<Retained<NSString>>;
        #[unsafe(method(originForURL:))]
        pub fn origin_for_url(url: &NSString) -> Option<Retained<NSString>>;
        #[unsafe(method(initialDocumentAccessed))]
        pub fn initial_document_accessed(&self) -> bool;
        #[unsafe(method(committedInitial))]
        pub fn committed_initial(&self) -> bool;
        #[unsafe(method(suppressInput))]
        pub fn suppress_input(&self) -> Option<Retained<AnyObject>>;
        #[unsafe(method(closeNow))]
        pub fn close_now(&self);
        #[unsafe(method(temporary))]
        pub fn temporary(&self) -> bool;
        #[unsafe(method(setTemporary:))]
        pub fn set_temporary(&self, on: bool);
        #[unsafe(method(setTemporarySource:))]
        pub fn set_temporary_source(&self, source: Option<&Tab>);
        #[unsafe(method(deliverHeldPageMessages))]
        pub fn deliver_held_page_messages(&self);
        #[unsafe(method(securityInfo))]
        pub fn security_info(&self) -> Retained<Payload>;
        #[unsafe(method(discard))]
        pub fn discard(&self) -> bool;
        #[unsafe(method(sleepIfHiddenFor:))]
        pub fn sleep_if_hidden_for(&self, seconds: NSTimeInterval) -> Retained<NSArray<NSNumber>>;
        #[unsafe(method(discarded))]
        pub fn discarded(&self) -> bool;
        #[unsafe(method(unloadKeepingHistory))]
        pub fn unload_keeping_history(&self) -> bool;
        #[unsafe(method(discardReason))]
        pub fn discard_reason(&self) -> Option<Retained<NSString>>;
        #[unsafe(method(frozen))]
        pub fn frozen(&self) -> bool;
        #[unsafe(method(setFrozen:))]
        pub fn set_frozen(&self, on: bool);
        #[unsafe(method(resolveUnresponsive:))]
        pub fn resolve_unresponsive(&self, terminate: bool);
        #[unsafe(method(showAutofillSuggestions:))]
        pub fn show_autofill_suggestions(&self, passwords: bool) -> bool;
        #[unsafe(method(popupFeatures))]
        pub fn popup_features(&self) -> Option<Retained<NSDictionary<NSString, NSNumber>>>;
        #[unsafe(method(preferredSize))]
        pub fn preferred_size(&self) -> NSSize;

        // Category `Navigation`.
        #[unsafe(method(goBack))]
        pub fn go_back(&self);
        #[unsafe(method(goForward))]
        pub fn go_forward(&self);
        #[unsafe(method(reload))]
        pub fn reload(&self);
        #[unsafe(method(stop))]
        pub fn stop(&self);
        #[unsafe(method(loadURL:userInitiated:))]
        pub fn load_url(&self, url: &NSString, user_initiated: bool);
        #[unsafe(method(goToOffset:))]
        pub fn go_to_offset(&self, offset: c_int);
        #[unsafe(method(reloadIgnoringCache))]
        pub fn reload_ignoring_cache(&self);
        #[unsafe(method(loadIfNeeded))]
        pub fn load_if_needed(&self);
        #[unsafe(method(navigationEntries))]
        pub fn navigation_entries(&self) -> Retained<NSArray<Payload>>;
        #[unsafe(method(committedHTTPStatus))]
        pub fn committed_http_status(&self) -> c_int;
        #[unsafe(method(committedDocumentCount))]
        pub fn committed_document_count(&self) -> c_int;

        // Category `Page`.
        #[unsafe(method(executeJavaScript:frame:))]
        pub fn execute_javascript(&self, code: &NSString, frame_id: Option<&NSString>);
        #[unsafe(method(evaluate:completion:))]
        fn evaluate_raw(&self, code: &NSString, completion: &DynBlock<dyn Fn(*mut AnyObject)>);
        #[unsafe(method(hasLiveFrame:))]
        pub fn has_live_frame(&self, frame_id: &NSString) -> bool;
        #[unsafe(method(callPage:json:))]
        pub fn call_page(&self, kind: &NSString, json: &NSString);
        #[unsafe(method(callFrame:kind:json:))]
        pub fn call_frame(&self, frame_id: &NSString, kind: &NSString, json: &NSString);
        #[unsafe(method(grantUserActivationToFrame:))]
        pub fn grant_user_activation_to_frame(&self, frame_id: &NSString);
        #[unsafe(method(pageBackgroundColor))]
        pub fn page_background_color(&self) -> Option<Retained<NSColor>>;
        #[unsafe(method(setPageBackgroundColor:))]
        pub fn set_page_background_color(&self, color: Option<&NSColor>);
        #[unsafe(method(pageColor))]
        pub fn page_color(&self) -> Option<Retained<NSString>>;
        #[unsafe(method(setPageColor:))]
        pub fn set_page_color(&self, color: Option<&NSString>);
        #[unsafe(method(contentsBackgroundColor))]
        pub fn contents_background_color(&self) -> Option<Retained<NSColor>>;
        #[unsafe(method(setContentsBackgroundColor:))]
        pub fn set_contents_background_color(&self, color: Option<&NSColor>);
        #[unsafe(method(zoomFactor))]
        pub fn zoom_factor(&self) -> f64;
        #[unsafe(method(setZoomFactor:))]
        pub fn set_zoom_factor(&self, factor: f64);
        #[unsafe(method(zoomStep:))]
        pub fn zoom_step(&self, direction: c_int);
        #[unsafe(method(pinchScale))]
        pub fn pinch_scale(&self) -> f64;

        // Category `Commands`.
        #[unsafe(method(executeChromeCommand:))]
        pub fn execute_chrome_command(&self, command: c_int) -> bool;
        #[unsafe(method(openBlockedPopup:always:))]
        pub fn open_blocked_popup(&self, popup_id: &NSString, always: bool);
        #[unsafe(method(showDevTools))]
        pub fn show_dev_tools(&self);
        #[unsafe(method(showDevToolsPanel:))]
        pub fn show_dev_tools_panel(&self, panel: &NSString);
        #[unsafe(method(closeDevTools))]
        pub fn close_dev_tools(&self);
        #[unsafe(method(devToolsView))]
        pub fn dev_tools_view(&self) -> Option<Retained<NSView>>;
        #[unsafe(method(devToolsLayoutForSize:devTools:page:))]
        pub unsafe fn dev_tools_layout_for_size(
            &self,
            size: NSSize,
            dev_tools_frame: *mut NSRect,
            page_frame: *mut NSRect,
        );
        #[unsafe(method(devToolsCall:params:timeout:completion:))]
        fn dev_tools_call_raw(
            &self,
            method: &NSString,
            params: Option<&NSDictionary>,
            timeout: NSTimeInterval,
            completion: &DynBlock<dyn Fn(*mut AnyObject, *mut AnyObject)>,
        );

        // Category `Media`.
        #[unsafe(method(mediaCaptureSourceId))]
        pub fn media_capture_source_id(&self) -> Option<Retained<NSString>>;
        #[unsafe(method(copyPicture:completion:))]
        pub fn copy_picture(
            &self,
            timeout: NSTimeInterval,
            completion: &DynBlock<dyn Fn(CGImageRef)>,
        );
        #[unsafe(method(stopCapture))]
        pub fn stop_capture(&self) -> bool;
        #[unsafe(method(noteShownByHost:))]
        pub fn note_shown_by_host(&self, shown: bool);
        #[unsafe(method(noteInShownSlot))]
        pub fn note_in_shown_slot(&self);
        #[unsafe(method(exitFullscreen))]
        pub fn exit_fullscreen(&self);
        #[unsafe(method(exitExclusiveAccess))]
        pub fn exit_exclusive_access(&self);
        #[unsafe(method(tracePictureInPicture:withInputs:))]
        pub fn trace_picture_in_picture(&self, line: &NSString, with_inputs: bool);
        #[unsafe(method(exitPictureInPicture))]
        pub fn exit_picture_in_picture(&self);
        #[unsafe(method(enterPictureInPictureForClickAt:completion:))]
        fn enter_picture_in_picture_for_click_at_raw(
            &self,
            click_time: NSTimeInterval,
            completion: &DynBlock<dyn Fn(*mut AnyObject, Bool)>,
        );
        #[unsafe(method(shareThisTabInsteadFor:))]
        pub fn share_this_tab_instead_for(&self, capturer: &Tab) -> bool;
        #[unsafe(method(showCastDialog))]
        pub fn show_cast_dialog(&self) -> bool;
        #[unsafe(method(devShowBluetoothChooser:))]
        pub fn dev_show_bluetooth_chooser(&self, unauthorized: bool) -> bool;

        // Category `Extensions`.
        #[unsafe(method(executeExtensionAction:))]
        pub fn execute_extension_action(&self, extension_id: &NSString) -> Retained<NSString>;
        #[unsafe(method(actionStatesForExtensions:))]
        pub fn action_states_for_extensions(
            &self,
            extension_ids: &NSArray<NSString>,
        ) -> Retained<NSDictionary<NSString, NSDictionary>>;
        #[unsafe(method(sidePanelURLForExtension:))]
        pub fn side_panel_url_for_extension(
            &self,
            extension_id: &NSString,
        ) -> Option<Retained<NSString>>;
    );
}

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    #[name = "ArdiaCorePageView"]
    pub struct PageView;
);

impl PageView {
    extern_methods!(
        #[unsafe(method(isPageView:))]
        pub fn is_page_view(view: &NSView) -> bool;
        /// The observer is weak; it must conform to `ArdiaCorePageScrollObserver`.
        #[unsafe(method(setScrollObserver:))]
        pub unsafe fn set_scroll_observer(observer: Option<&AnyObject>);
        #[unsafe(method(takeSwipes:))]
        pub fn take_swipes(view: &NSView) -> bool;
        #[unsafe(method(takesSwipes:))]
        pub fn takes_swipes(view: &NSView) -> bool;
        #[unsafe(method(isEditingText:))]
        pub fn is_editing_text(view: &NSView) -> bool;
    );
}

/// `NSEvent` is re-exported so that the callers of the delegate need no second import.
pub type Event = NSEvent;

// The block parameters above are raw pointers: they encode as `@`, and they keep the lifetimes out of the macro.
// Each wrapper below gives the closure owned values (`Retained`), so that no closure type has a higher-ranked lifetime.

impl Engine {
    /// `-loadProfile:completion:`. The completion may run before this call returns.
    pub fn load_profile(
        &self,
        directory_name: &NSString,
        completion: impl Fn(Option<Retained<Profile>>) + 'static,
    ) {
        self.load_profile_raw(
            directory_name,
            &RcBlock::new(move |p: *mut AnyObject| {
                completion(unsafe { Retained::retain(p.cast::<Profile>()) })
            }),
        );
    }

    /// `-offTheRecordProfileFor:completion:`: the completion gets the private profile or the error text.
    pub fn off_the_record_profile_for(
        &self,
        profile: &Profile,
        completion: impl Fn(Option<Retained<Profile>>, Option<String>) + 'static,
    ) {
        self.off_the_record_profile_for_raw(
            profile,
            &RcBlock::new(move |p: *mut AnyObject, e: *mut AnyObject| {
                completion(
                    unsafe { Retained::retain(p.cast::<Profile>()) },
                    unsafe { e.cast::<NSString>().as_ref() }.map(|e| e.to_string()),
                )
            }),
        );
    }

    /// `+endTracing:completion:`: the completion gets the path of the trace, or `None`.
    pub fn end_tracing(keep: bool, completion: impl Fn(Option<String>) + 'static) {
        Self::end_tracing_raw(
            keep,
            &RcBlock::new(move |p: *mut AnyObject| {
                completion(unsafe { p.cast::<NSString>().as_ref() }.map(|p| p.to_string()))
            }),
        );
    }
}

impl Profile {
    /// `-fetchIcon:maxBytes:completion:`: the completion gets the bytes, or `None`.
    pub fn fetch_icon(
        &self,
        url: &NSString,
        max_bytes: usize,
        completion: impl Fn(Option<Vec<u8>>) + 'static,
    ) {
        self.fetch_icon_raw(
            url,
            max_bytes,
            &RcBlock::new(move |d: *mut AnyObject| {
                completion(unsafe { d.cast::<NSData>().as_ref() }.map(NSData::to_vec))
            }),
        );
    }
}

impl Tab {
    /// `-evaluate:completion:`: the completion gets the JSON text of the result, or `None`.
    pub fn evaluate(&self, code: &NSString, completion: impl Fn(Option<String>) + 'static) {
        self.evaluate_raw(
            code,
            &RcBlock::new(move |j: *mut AnyObject| {
                completion(unsafe { j.cast::<NSString>().as_ref() }.map(|j| j.to_string()))
            }),
        );
    }

    /// `-devToolsCall:params:timeout:completion:`: the completion gets the result or the error text.
    pub fn dev_tools_call(
        &self,
        method: &NSString,
        params: Option<&NSDictionary>,
        timeout: NSTimeInterval,
        completion: impl Fn(Option<Retained<NSDictionary>>, Option<String>) + 'static,
    ) {
        self.dev_tools_call_raw(
            method,
            params,
            timeout,
            &RcBlock::new(move |r: *mut AnyObject, e: *mut AnyObject| {
                completion(
                    unsafe { Retained::retain(r.cast::<NSDictionary>()) },
                    unsafe { e.cast::<NSString>().as_ref() }.map(|e| e.to_string()),
                )
            }),
        );
    }

    /// `-enterPictureInPictureForClickAt:completion:`: the completion gets the route and whether the window opened.
    pub fn enter_picture_in_picture_for_click_at(
        &self,
        click_time: NSTimeInterval,
        completion: impl Fn(String, bool) + 'static,
    ) {
        self.enter_picture_in_picture_for_click_at_raw(
            click_time,
            &RcBlock::new(move |route: *mut AnyObject, opened: Bool| {
                completion(
                    unsafe { route.cast::<NSString>().as_ref() }
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    opened.as_bool(),
                )
            }),
        );
    }
}
