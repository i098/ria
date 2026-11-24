// Ported from ardia apps/browser/src/state/onboarding/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `onboarding` (`apps/browser/src/state/onboarding`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.28 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiveBaseline {
    #[serde(
        rename = "cliControl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cli_control: Option<bool>,
    #[serde(
        rename = "warnBeforeQuitting",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_quitting: Option<bool>,
    #[serde(
        rename = "warnBeforeClosingWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_closing_window: Option<bool>,
    #[serde(
        rename = "restoreSession",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_session: Option<bool>,
    #[serde(
        rename = "separateDownloadFolders",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub separate_download_folders: Option<bool>,
    #[serde(
        rename = "searchEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub search_engine: Option<String>,
    #[serde(
        rename = "customSearchUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_search_url: Option<String>,
    #[serde(
        rename = "customSearchEngines",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_search_engines: Option<Vec<serde_json::Value>>,
    #[serde(
        rename = "extensionSearchEngines",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension_search_engines: Option<Vec<serde_json::Value>>,
    #[serde(
        rename = "searchSuggestions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub search_suggestions: Option<bool>,
    #[serde(
        rename = "commandBarHistory",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_bar_history: Option<bool>,
    #[serde(
        rename = "showSelectionSearch",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_selection_search: Option<bool>,
    #[serde(rename = "tabLayout", default, skip_serializing_if = "Option::is_none")]
    pub tab_layout: Option<LiveBaselineTabLayout>,
    #[serde(
        rename = "sidebarSide",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_side: Option<LiveBaselineSidebarSide>,
    #[serde(
        rename = "autoHideTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_hide_tabs: Option<bool>,
    #[serde(
        rename = "hideToolbarWhileScrolling",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_toolbar_while_scrolling: Option<bool>,
    #[serde(
        rename = "newTabPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_position: Option<LiveBaselineNewTabPosition>,
    #[serde(
        rename = "newTabButtonPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_button_position: Option<LiveBaselineNewTabButtonPosition>,
    #[serde(
        rename = "appLinksFirst",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub app_links_first: Option<bool>,
    #[serde(
        rename = "newTabOpens",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_opens: Option<LiveBaselineNewTabOpens>,
    #[serde(rename = "newTabUrl", default, skip_serializing_if = "Option::is_none")]
    pub new_tab_url: Option<String>,
    #[serde(
        rename = "warnBeforeClosingLastTab",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_closing_last_tab: Option<bool>,
    #[serde(
        rename = "keepTopAppsRunning",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keep_top_apps_running: Option<bool>,
    #[serde(
        rename = "tabReorderHaptics",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tab_reorder_haptics: Option<bool>,
    #[serde(
        rename = "swipeBetweenTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub swipe_between_tabs: Option<bool>,
    #[serde(
        rename = "undoReopensClosedTab",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub undo_reopens_closed_tab: Option<bool>,
    #[serde(
        rename = "warnBeforeMovingTabsToProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_moving_tabs_to_profile: Option<bool>,
    #[serde(
        rename = "shareTabsAcrossWindows",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub share_tabs_across_windows: Option<bool>,
    #[serde(
        rename = "extendWebsiteColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extend_website_color: Option<bool>,
    #[serde(
        rename = "cmdClickCreatesTabGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cmd_click_creates_tab_group: Option<bool>,
    #[serde(
        rename = "optShiftClickOpensInGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub opt_shift_click_opens_in_group: Option<bool>,
    #[serde(
        rename = "peekModifier",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub peek_modifier: Option<LiveBaselinePeekModifier>,
    #[serde(
        rename = "peekLinksToOtherSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub peek_links_to_other_sites: Option<bool>,
    #[serde(
        rename = "newTabLinksOpenIn",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_links_open_in: Option<LiveBaselineNewTabLinksOpenIn>,
    #[serde(
        rename = "popupsOpenAsTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub popups_open_as_tabs: Option<bool>,
    #[serde(
        rename = "peekButtons",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub peek_buttons: Option<LiveBaselinePeekButtons>,
    #[serde(
        rename = "splitLinkModifier",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub split_link_modifier: Option<LiveBaselineSplitLinkModifier>,
    #[serde(
        rename = "autoGroupMeetingTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_group_meeting_tabs: Option<bool>,
    #[serde(
        rename = "cleanUpInactiveTabsAfterMinutes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub clean_up_inactive_tabs_after_minutes: Option<Number>,
    #[serde(
        rename = "suggestCleanUp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub suggest_clean_up: Option<bool>,
    #[serde(
        rename = "mutedSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub muted_sites: Option<Vec<String>>,
    #[serde(
        rename = "showUnreadCounts",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_unread_counts: Option<bool>,
    #[serde(
        rename = "showTopAppBadges",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_top_app_badges: Option<bool>,
    #[serde(
        rename = "showTabHoverMenu",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_tab_hover_menu: Option<bool>,
    #[serde(
        rename = "showFullUrlInTabHoverMenu",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_full_url_in_tab_hover_menu: Option<bool>,
    #[serde(
        rename = "closedTopAppTiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub closed_top_app_tiles: Option<bool>,
    #[serde(
        rename = "topAppFolders",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub top_app_folders: Option<bool>,
    #[serde(
        rename = "unreadHiddenSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub unread_hidden_sites: Option<Vec<String>>,
    #[serde(
        rename = "neverTranslateSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub never_translate_sites: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(
        rename = "developerModeSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub developer_mode_sites: Option<std::collections::HashMap<String, bool>>,
    #[serde(
        rename = "autoPictureInPicture",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_picture_in_picture: Option<bool>,
    #[serde(
        rename = "playingNowAcrossProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub playing_now_across_profiles: Option<bool>,
    #[serde(
        rename = "sidebarMediaControls",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_media_controls: Option<bool>,
    #[serde(
        rename = "commandScrollZoom",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_scroll_zoom: Option<bool>,
    #[serde(
        rename = "blockAutoplayEverywhere",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub block_autoplay_everywhere: Option<bool>,
    #[serde(
        rename = "blockNotificationRequests",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub block_notification_requests: Option<bool>,
    #[serde(
        rename = "videoUpscaling",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub video_upscaling: Option<bool>,
    #[serde(
        rename = "sidebarWidth",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_width: Option<Number>,
    #[serde(
        rename = "showSidebarButton",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_sidebar_button: Option<bool>,
    #[serde(
        rename = "showClearButton",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_clear_button: Option<bool>,
    #[serde(rename = "tidyTabs", default, skip_serializing_if = "Option::is_none")]
    pub tidy_tabs: Option<bool>,
    #[serde(
        rename = "balancedTopApps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub balanced_top_apps: Option<bool>,
    #[serde(
        rename = "selectedTabGlow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub selected_tab_glow: Option<LiveBaselineSelectedTabGlow>,
    #[serde(
        rename = "interfaceScale",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub interface_scale: Option<Number>,
    #[serde(
        rename = "sidebarTextSize",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_text_size: Option<LiveBaselineSidebarTextSize>,
    #[serde(
        rename = "sidebarTextContrast",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_text_contrast: Option<LiveBaselineSidebarTextContrast>,
    #[serde(
        rename = "selectedTopApp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub selected_top_app: Option<LiveBaselineSelectedTopApp>,
    #[serde(
        rename = "elementOutlines",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub element_outlines: Option<bool>,
    #[serde(
        rename = "interfaceStyle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub interface_style: Option<LiveBaselineInterfaceStyle>,
    #[serde(
        rename = "windowButtons",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub window_buttons: Option<LiveBaselineWindowButtons>,
    #[serde(
        rename = "glassTintWithTheme",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub glass_tint_with_theme: Option<bool>,
    #[serde(
        rename = "glassTintAmount",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub glass_tint_amount: Option<Number>,
    #[serde(
        rename = "fadeUnloadedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fade_unloaded_tabs: Option<bool>,
    #[serde(
        rename = "strikeSleepingTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub strike_sleeping_tabs: Option<bool>,
    #[serde(
        rename = "warnBeforeClosingTabGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_closing_tab_group: Option<bool>,
    #[serde(
        rename = "batterySaverWhen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub battery_saver_when: Option<LiveBaselineBatterySaverWhen>,
    #[serde(
        rename = "unloadProfilesAfter",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub unload_profiles_after: Option<LiveBaselineUnloadProfilesAfter>,
    #[serde(
        rename = "memorySaver",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub memory_saver: Option<bool>,
    #[serde(
        rename = "memorySaverMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub memory_saver_mode: Option<LiveBaselineMemorySaverMode>,
    #[serde(
        rename = "loadProfilesWhenOpened",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub load_profiles_when_opened: Option<bool>,
    #[serde(
        rename = "keepActiveSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keep_active_sites: Option<Vec<String>>,
    #[serde(
        rename = "extensionSidePanelWidth",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension_side_panel_width: Option<Number>,
    #[serde(
        rename = "showFullUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_full_url: Option<bool>,
    #[serde(
        rename = "allowJavaScriptFromAppleEvents",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_java_script_from_apple_events: Option<bool>,
    #[serde(
        rename = "compactCliPill",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub compact_cli_pill: Option<bool>,
    #[serde(
        rename = "showBookmarks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_bookmarks: Option<bool>,
    #[serde(
        rename = "bookmarksBar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bookmarks_bar: Option<LiveBaselineBookmarksBar>,
    #[serde(
        rename = "bookmarksImportDismissed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bookmarks_import_dismissed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<LiveBaselineAppearance>,
    #[serde(
        rename = "addressBar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub address_bar: Option<LiveBaselineAddressBar>,
    #[serde(
        rename = "compactToolbar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub compact_toolbar: Option<bool>,
    #[serde(
        rename = "addressBarButtons",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub address_bar_buttons: Option<LiveBaselineAddressBarButtons>,
    #[serde(rename = "pageFrame", default, skip_serializing_if = "Option::is_none")]
    pub page_frame: Option<Number>,
    #[serde(
        rename = "borderlessMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub borderless_mode: Option<bool>,
    #[serde(
        rename = "tabCornerRadius",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tab_corner_radius: Option<Number>,
    #[serde(
        rename = "minimalismMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub minimalism_mode: Option<bool>,
    #[serde(
        rename = "pageFrameZeroInFullScreen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub page_frame_zero_in_full_screen: Option<bool>,
    #[serde(
        rename = "pageFrameZeroWhenTabsHidden",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub page_frame_zero_when_tabs_hidden: Option<bool>,
    #[serde(
        rename = "commandBarPlacement",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_bar_placement: Option<LiveBaselineCommandBarPlacement>,
    #[serde(
        rename = "commandBarStart",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_bar_start: Option<crate::omnibox::CommandBarStart>,
    #[serde(
        rename = "pressMotion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub press_motion: Option<LiveBaselinePressMotion>,
    #[serde(
        rename = "folderStyle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub folder_style: Option<LiveBaselineFolderStyle>,
    #[serde(
        rename = "liveFolderIcon",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub live_folder_icon: Option<LiveBaselineLiveFolderIcon>,
    #[serde(
        rename = "spacesFollowAppearance",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub spaces_follow_appearance: Option<bool>,
    #[serde(
        rename = "adaptTextToColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub adapt_text_to_color: Option<bool>,
    #[serde(
        rename = "defaultProfileId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_profile_id: Option<String>,
    #[serde(
        rename = "showProfileDots",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_profile_dots: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcuts: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(
        rename = "mouseShortcuts",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub mouse_shortcuts: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(
        rename = "openLinksInLittleArdia",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub open_links_in_little_ardia: Option<bool>,
    #[serde(
        rename = "littleArdiaOpensNewTabLinks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub little_ardia_opens_new_tab_links: Option<bool>,
    #[serde(
        rename = "littleArdiaShortcutFromAnyApp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub little_ardia_shortcut_from_any_app: Option<bool>,
    #[serde(
        rename = "littleArdiaSize",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub little_ardia_size: Option<(Number, Number)>,
    #[serde(
        rename = "engineMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_mode: Option<LiveBaselineEngineMode>,
    #[serde(
        rename = "pageEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub page_engine: Option<LiveBaselinePageEngine>,
    #[serde(
        rename = "websiteEngines",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub website_engines: Option<std::collections::HashMap<String, LiveBaselineWebsiteEnginesValue>>,
    #[serde(
        rename = "protectedVideoInWebKit",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub protected_video_in_web_kit: Option<bool>,
    #[serde(
        rename = "offerWebKitForProtectedVideo",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub offer_web_kit_for_protected_video: Option<bool>,
    #[serde(
        rename = "engineSwitchKeepsPage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_switch_keeps_page: Option<bool>,
    #[serde(
        rename = "engineSwitchAsksAboutUnsavedWork",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_switch_asks_about_unsaved_work: Option<bool>,
    #[serde(
        rename = "engineSwitchCarriesSiteData",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_switch_carries_site_data: Option<bool>,
    #[serde(
        rename = "webKitFraudWarnings",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub web_kit_fraud_warnings: Option<bool>,
    #[serde(
        rename = "launchActions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub launch_actions: Option<Vec<String>>,
    #[serde(
        rename = "omniboxProviders",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub omnibox_providers: Option<Vec<serde_json::Value>>,
    #[serde(
        rename = "readerTextSize",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reader_text_size: Option<LiveBaselineReaderTextSize>,
    #[serde(
        rename = "readerFont",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reader_font: Option<LiveBaselineReaderFont>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<LiveBaselineColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub looks: Option<std::collections::HashMap<String, ProfileLookBaseline>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineTabLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineSidebarSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineNewTabPosition {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "nextToCurrent")]
    NextToCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineNewTabButtonPosition {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineNewTabOpens {
    #[default]
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "commandBar")]
    CommandBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselinePeekModifier {
    #[default]
    #[serde(rename = "alt")]
    Alt,
    #[serde(rename = "shift")]
    Shift,
    #[serde(rename = "off")]
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineNewTabLinksOpenIn {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "littleArdia")]
    LittleArdia,
    #[serde(rename = "peek")]
    Peek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselinePeekButtons {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineSplitLinkModifier {
    #[default]
    #[serde(rename = "alt")]
    Alt,
    #[serde(rename = "altShift")]
    AltShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineSelectedTabGlow {
    #[default]
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "soft")]
    Soft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineSidebarTextSize {
    #[default]
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "large")]
    Large,
    #[serde(rename = "larger")]
    Larger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineSidebarTextContrast {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "standard")]
    Standard,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineSelectedTopApp {
    #[default]
    #[serde(rename = "ring")]
    Ring,
    #[serde(rename = "fill")]
    Fill,
    #[serde(rename = "outline")]
    Outline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineInterfaceStyle {
    #[default]
    #[serde(rename = "glass")]
    Glass,
    #[serde(rename = "classic")]
    Classic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineWindowButtons {
    #[default]
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "trafficLights")]
    TrafficLights,
    #[serde(rename = "hidden")]
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineBatterySaverWhen {
    #[default]
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "lowBattery")]
    LowBattery,
    #[serde(rename = "unplugged")]
    Unplugged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineUnloadProfilesAfter {
    #[default]
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "5m")]
    V5m,
    #[serde(rename = "15m")]
    V15m,
    #[serde(rename = "1h")]
    V1h,
    #[serde(rename = "2h")]
    V2h,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineMemorySaverMode {
    #[default]
    #[serde(rename = "moderate")]
    Moderate,
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "maximum")]
    Maximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineBookmarksBar {
    #[default]
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "always")]
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineAppearance {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineAddressBar {
    #[default]
    #[serde(rename = "toolbar")]
    Toolbar,
    #[serde(rename = "sidebar")]
    Sidebar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineAddressBarButtons {
    #[default]
    #[serde(rename = "hover")]
    Hover,
    #[serde(rename = "always")]
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineCommandBarPlacement {
    #[default]
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "centred")]
    Centred,
    #[serde(rename = "centredOnClick")]
    CentredOnClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselinePressMotion {
    #[default]
    #[serde(rename = "still")]
    Still,
    #[serde(rename = "bounce")]
    Bounce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineFolderStyle {
    #[default]
    #[serde(rename = "neutral")]
    Neutral,
    #[serde(rename = "profile")]
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineLiveFolderIcon {
    #[default]
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "service")]
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineEngineMode {
    #[default]
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselinePageEngine {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineWebsiteEnginesValue {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineReaderTextSize {
    #[default]
    #[serde(rename = "large")]
    Large,
    #[serde(rename = "larger")]
    Larger,
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "largest")]
    Largest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineReaderFont {
    #[default]
    #[serde(rename = "serif")]
    Serif,
    #[serde(rename = "sans")]
    Sans,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveBaselineColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProfileLookBaseline {
    pub color: ProfileLookBaselineColor,
    pub theme: serde_json::Value,
    #[serde(rename = "presetThemes")]
    pub preset_themes: crate::browser::ProfilePresetThemes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileLookBaselineColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateDraft {
    #[serde(
        rename = "cliControl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cli_control: Option<bool>,
    #[serde(
        rename = "warnBeforeQuitting",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_quitting: Option<bool>,
    #[serde(
        rename = "warnBeforeClosingWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_closing_window: Option<bool>,
    #[serde(
        rename = "restoreSession",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_session: Option<bool>,
    #[serde(
        rename = "separateDownloadFolders",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub separate_download_folders: Option<bool>,
    #[serde(
        rename = "searchEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub search_engine: Option<String>,
    #[serde(
        rename = "customSearchUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_search_url: Option<String>,
    #[serde(
        rename = "customSearchEngines",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_search_engines: Option<Vec<serde_json::Value>>,
    #[serde(
        rename = "extensionSearchEngines",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension_search_engines: Option<Vec<serde_json::Value>>,
    #[serde(
        rename = "searchSuggestions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub search_suggestions: Option<bool>,
    #[serde(
        rename = "commandBarHistory",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_bar_history: Option<bool>,
    #[serde(
        rename = "showSelectionSearch",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_selection_search: Option<bool>,
    #[serde(rename = "tabLayout", default, skip_serializing_if = "Option::is_none")]
    pub tab_layout: Option<StateDraftTabLayout>,
    #[serde(
        rename = "sidebarSide",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_side: Option<StateDraftSidebarSide>,
    #[serde(
        rename = "autoHideTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_hide_tabs: Option<bool>,
    #[serde(
        rename = "hideToolbarWhileScrolling",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_toolbar_while_scrolling: Option<bool>,
    #[serde(
        rename = "newTabPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_position: Option<StateDraftNewTabPosition>,
    #[serde(
        rename = "newTabButtonPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_button_position: Option<StateDraftNewTabButtonPosition>,
    #[serde(
        rename = "appLinksFirst",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub app_links_first: Option<bool>,
    #[serde(
        rename = "newTabOpens",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_opens: Option<StateDraftNewTabOpens>,
    #[serde(rename = "newTabUrl", default, skip_serializing_if = "Option::is_none")]
    pub new_tab_url: Option<String>,
    #[serde(
        rename = "warnBeforeClosingLastTab",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_closing_last_tab: Option<bool>,
    #[serde(
        rename = "keepTopAppsRunning",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keep_top_apps_running: Option<bool>,
    #[serde(
        rename = "tabReorderHaptics",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tab_reorder_haptics: Option<bool>,
    #[serde(
        rename = "swipeBetweenTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub swipe_between_tabs: Option<bool>,
    #[serde(
        rename = "undoReopensClosedTab",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub undo_reopens_closed_tab: Option<bool>,
    #[serde(
        rename = "warnBeforeMovingTabsToProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_moving_tabs_to_profile: Option<bool>,
    #[serde(
        rename = "shareTabsAcrossWindows",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub share_tabs_across_windows: Option<bool>,
    #[serde(
        rename = "extendWebsiteColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extend_website_color: Option<bool>,
    #[serde(
        rename = "cmdClickCreatesTabGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cmd_click_creates_tab_group: Option<bool>,
    #[serde(
        rename = "optShiftClickOpensInGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub opt_shift_click_opens_in_group: Option<bool>,
    #[serde(
        rename = "peekModifier",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub peek_modifier: Option<StateDraftPeekModifier>,
    #[serde(
        rename = "peekLinksToOtherSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub peek_links_to_other_sites: Option<bool>,
    #[serde(
        rename = "newTabLinksOpenIn",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_links_open_in: Option<StateDraftNewTabLinksOpenIn>,
    #[serde(
        rename = "popupsOpenAsTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub popups_open_as_tabs: Option<bool>,
    #[serde(
        rename = "peekButtons",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub peek_buttons: Option<StateDraftPeekButtons>,
    #[serde(
        rename = "splitLinkModifier",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub split_link_modifier: Option<StateDraftSplitLinkModifier>,
    #[serde(
        rename = "autoGroupMeetingTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_group_meeting_tabs: Option<bool>,
    #[serde(
        rename = "cleanUpInactiveTabsAfterMinutes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub clean_up_inactive_tabs_after_minutes: Option<Number>,
    #[serde(
        rename = "suggestCleanUp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub suggest_clean_up: Option<bool>,
    #[serde(
        rename = "mutedSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub muted_sites: Option<Vec<String>>,
    #[serde(
        rename = "showUnreadCounts",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_unread_counts: Option<bool>,
    #[serde(
        rename = "showTopAppBadges",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_top_app_badges: Option<bool>,
    #[serde(
        rename = "showTabHoverMenu",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_tab_hover_menu: Option<bool>,
    #[serde(
        rename = "showFullUrlInTabHoverMenu",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_full_url_in_tab_hover_menu: Option<bool>,
    #[serde(
        rename = "closedTopAppTiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub closed_top_app_tiles: Option<bool>,
    #[serde(
        rename = "topAppFolders",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub top_app_folders: Option<bool>,
    #[serde(
        rename = "unreadHiddenSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub unread_hidden_sites: Option<Vec<String>>,
    #[serde(
        rename = "neverTranslateSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub never_translate_sites: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(
        rename = "developerModeSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub developer_mode_sites: Option<std::collections::HashMap<String, bool>>,
    #[serde(
        rename = "autoPictureInPicture",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_picture_in_picture: Option<bool>,
    #[serde(
        rename = "playingNowAcrossProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub playing_now_across_profiles: Option<bool>,
    #[serde(
        rename = "sidebarMediaControls",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_media_controls: Option<bool>,
    #[serde(
        rename = "commandScrollZoom",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_scroll_zoom: Option<bool>,
    #[serde(
        rename = "blockAutoplayEverywhere",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub block_autoplay_everywhere: Option<bool>,
    #[serde(
        rename = "blockNotificationRequests",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub block_notification_requests: Option<bool>,
    #[serde(
        rename = "videoUpscaling",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub video_upscaling: Option<bool>,
    #[serde(
        rename = "sidebarWidth",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_width: Option<Number>,
    #[serde(
        rename = "showSidebarButton",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_sidebar_button: Option<bool>,
    #[serde(
        rename = "showClearButton",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_clear_button: Option<bool>,
    #[serde(rename = "tidyTabs", default, skip_serializing_if = "Option::is_none")]
    pub tidy_tabs: Option<bool>,
    #[serde(
        rename = "balancedTopApps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub balanced_top_apps: Option<bool>,
    #[serde(
        rename = "selectedTabGlow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub selected_tab_glow: Option<StateDraftSelectedTabGlow>,
    #[serde(
        rename = "interfaceScale",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub interface_scale: Option<Number>,
    #[serde(
        rename = "sidebarTextSize",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_text_size: Option<StateDraftSidebarTextSize>,
    #[serde(
        rename = "sidebarTextContrast",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_text_contrast: Option<StateDraftSidebarTextContrast>,
    #[serde(
        rename = "selectedTopApp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub selected_top_app: Option<StateDraftSelectedTopApp>,
    #[serde(
        rename = "elementOutlines",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub element_outlines: Option<bool>,
    #[serde(
        rename = "interfaceStyle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub interface_style: Option<StateDraftInterfaceStyle>,
    #[serde(
        rename = "windowButtons",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub window_buttons: Option<StateDraftWindowButtons>,
    #[serde(
        rename = "glassTintWithTheme",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub glass_tint_with_theme: Option<bool>,
    #[serde(
        rename = "glassTintAmount",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub glass_tint_amount: Option<Number>,
    #[serde(
        rename = "fadeUnloadedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fade_unloaded_tabs: Option<bool>,
    #[serde(
        rename = "strikeSleepingTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub strike_sleeping_tabs: Option<bool>,
    #[serde(
        rename = "warnBeforeClosingTabGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_before_closing_tab_group: Option<bool>,
    #[serde(
        rename = "batterySaverWhen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub battery_saver_when: Option<StateDraftBatterySaverWhen>,
    #[serde(
        rename = "unloadProfilesAfter",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub unload_profiles_after: Option<StateDraftUnloadProfilesAfter>,
    #[serde(
        rename = "memorySaver",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub memory_saver: Option<bool>,
    #[serde(
        rename = "memorySaverMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub memory_saver_mode: Option<StateDraftMemorySaverMode>,
    #[serde(
        rename = "loadProfilesWhenOpened",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub load_profiles_when_opened: Option<bool>,
    #[serde(
        rename = "keepActiveSites",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keep_active_sites: Option<Vec<String>>,
    #[serde(
        rename = "extensionSidePanelWidth",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension_side_panel_width: Option<Number>,
    #[serde(
        rename = "showFullUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_full_url: Option<bool>,
    #[serde(
        rename = "allowJavaScriptFromAppleEvents",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_java_script_from_apple_events: Option<bool>,
    #[serde(
        rename = "compactCliPill",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub compact_cli_pill: Option<bool>,
    #[serde(
        rename = "showBookmarks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_bookmarks: Option<bool>,
    #[serde(
        rename = "bookmarksBar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bookmarks_bar: Option<StateDraftBookmarksBar>,
    #[serde(
        rename = "bookmarksImportDismissed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bookmarks_import_dismissed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<StateDraftAppearance>,
    #[serde(
        rename = "addressBar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub address_bar: Option<StateDraftAddressBar>,
    #[serde(
        rename = "compactToolbar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub compact_toolbar: Option<bool>,
    #[serde(
        rename = "addressBarButtons",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub address_bar_buttons: Option<StateDraftAddressBarButtons>,
    #[serde(rename = "pageFrame", default, skip_serializing_if = "Option::is_none")]
    pub page_frame: Option<Number>,
    #[serde(
        rename = "borderlessMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub borderless_mode: Option<bool>,
    #[serde(
        rename = "tabCornerRadius",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tab_corner_radius: Option<Number>,
    #[serde(
        rename = "minimalismMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub minimalism_mode: Option<bool>,
    #[serde(
        rename = "pageFrameZeroInFullScreen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub page_frame_zero_in_full_screen: Option<bool>,
    #[serde(
        rename = "pageFrameZeroWhenTabsHidden",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub page_frame_zero_when_tabs_hidden: Option<bool>,
    #[serde(
        rename = "commandBarPlacement",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_bar_placement: Option<StateDraftCommandBarPlacement>,
    #[serde(
        rename = "commandBarStart",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_bar_start: Option<crate::omnibox::CommandBarStart>,
    #[serde(
        rename = "pressMotion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub press_motion: Option<StateDraftPressMotion>,
    #[serde(
        rename = "folderStyle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub folder_style: Option<StateDraftFolderStyle>,
    #[serde(
        rename = "liveFolderIcon",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub live_folder_icon: Option<StateDraftLiveFolderIcon>,
    #[serde(
        rename = "spacesFollowAppearance",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub spaces_follow_appearance: Option<bool>,
    #[serde(
        rename = "adaptTextToColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub adapt_text_to_color: Option<bool>,
    #[serde(
        rename = "defaultProfileId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_profile_id: Option<String>,
    #[serde(
        rename = "showProfileDots",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_profile_dots: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcuts: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(
        rename = "mouseShortcuts",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub mouse_shortcuts: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(
        rename = "openLinksInLittleArdia",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub open_links_in_little_ardia: Option<bool>,
    #[serde(
        rename = "littleArdiaOpensNewTabLinks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub little_ardia_opens_new_tab_links: Option<bool>,
    #[serde(
        rename = "littleArdiaShortcutFromAnyApp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub little_ardia_shortcut_from_any_app: Option<bool>,
    #[serde(
        rename = "littleArdiaSize",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub little_ardia_size: Option<(Number, Number)>,
    #[serde(
        rename = "engineMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_mode: Option<StateDraftEngineMode>,
    #[serde(
        rename = "pageEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub page_engine: Option<StateDraftPageEngine>,
    #[serde(
        rename = "websiteEngines",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub website_engines: Option<std::collections::HashMap<String, StateDraftWebsiteEnginesValue>>,
    #[serde(
        rename = "protectedVideoInWebKit",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub protected_video_in_web_kit: Option<bool>,
    #[serde(
        rename = "offerWebKitForProtectedVideo",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub offer_web_kit_for_protected_video: Option<bool>,
    #[serde(
        rename = "engineSwitchKeepsPage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_switch_keeps_page: Option<bool>,
    #[serde(
        rename = "engineSwitchAsksAboutUnsavedWork",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_switch_asks_about_unsaved_work: Option<bool>,
    #[serde(
        rename = "engineSwitchCarriesSiteData",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_switch_carries_site_data: Option<bool>,
    #[serde(
        rename = "webKitFraudWarnings",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub web_kit_fraud_warnings: Option<bool>,
    #[serde(
        rename = "launchActions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub launch_actions: Option<Vec<String>>,
    #[serde(
        rename = "omniboxProviders",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub omnibox_providers: Option<Vec<serde_json::Value>>,
    #[serde(
        rename = "readerTextSize",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reader_text_size: Option<StateDraftReaderTextSize>,
    #[serde(
        rename = "readerFont",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reader_font: Option<StateDraftReaderFont>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftTabLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftSidebarSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftNewTabPosition {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "nextToCurrent")]
    NextToCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftNewTabButtonPosition {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftNewTabOpens {
    #[default]
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "commandBar")]
    CommandBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftPeekModifier {
    #[default]
    #[serde(rename = "alt")]
    Alt,
    #[serde(rename = "shift")]
    Shift,
    #[serde(rename = "off")]
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftNewTabLinksOpenIn {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "littleArdia")]
    LittleArdia,
    #[serde(rename = "peek")]
    Peek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftPeekButtons {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftSplitLinkModifier {
    #[default]
    #[serde(rename = "alt")]
    Alt,
    #[serde(rename = "altShift")]
    AltShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftSelectedTabGlow {
    #[default]
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "soft")]
    Soft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftSidebarTextSize {
    #[default]
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "large")]
    Large,
    #[serde(rename = "larger")]
    Larger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftSidebarTextContrast {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "standard")]
    Standard,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftSelectedTopApp {
    #[default]
    #[serde(rename = "ring")]
    Ring,
    #[serde(rename = "fill")]
    Fill,
    #[serde(rename = "outline")]
    Outline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftInterfaceStyle {
    #[default]
    #[serde(rename = "glass")]
    Glass,
    #[serde(rename = "classic")]
    Classic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftWindowButtons {
    #[default]
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "trafficLights")]
    TrafficLights,
    #[serde(rename = "hidden")]
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftBatterySaverWhen {
    #[default]
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "lowBattery")]
    LowBattery,
    #[serde(rename = "unplugged")]
    Unplugged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftUnloadProfilesAfter {
    #[default]
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "5m")]
    V5m,
    #[serde(rename = "15m")]
    V15m,
    #[serde(rename = "1h")]
    V1h,
    #[serde(rename = "2h")]
    V2h,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftMemorySaverMode {
    #[default]
    #[serde(rename = "moderate")]
    Moderate,
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "maximum")]
    Maximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftBookmarksBar {
    #[default]
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "always")]
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftAppearance {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftAddressBar {
    #[default]
    #[serde(rename = "toolbar")]
    Toolbar,
    #[serde(rename = "sidebar")]
    Sidebar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftAddressBarButtons {
    #[default]
    #[serde(rename = "hover")]
    Hover,
    #[serde(rename = "always")]
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftCommandBarPlacement {
    #[default]
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "centred")]
    Centred,
    #[serde(rename = "centredOnClick")]
    CentredOnClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftPressMotion {
    #[default]
    #[serde(rename = "still")]
    Still,
    #[serde(rename = "bounce")]
    Bounce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftFolderStyle {
    #[default]
    #[serde(rename = "neutral")]
    Neutral,
    #[serde(rename = "profile")]
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftLiveFolderIcon {
    #[default]
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "service")]
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftEngineMode {
    #[default]
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftPageEngine {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftWebsiteEnginesValue {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftReaderTextSize {
    #[default]
    #[serde(rename = "large")]
    Large,
    #[serde(rename = "larger")]
    Larger,
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "largest")]
    Largest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateDraftReaderFont {
    #[default]
    #[serde(rename = "serif")]
    Serif,
    #[serde(rename = "sans")]
    Sans,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateExtras {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<StateExtrasColor>,
    #[serde(rename = "appIcon", default, skip_serializing_if = "Option::is_none")]
    pub app_icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pins: Option<Vec<String>>,
    #[serde(
        rename = "setDefault",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_default: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trial: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dock: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateExtrasColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiveMock {
    pub settings: StateDraft,
    pub label: String,
    pub focus: Option<LiveMockFocus>,
    pub accent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveMockFocus {
    #[default]
    #[serde(rename = "tabs-sidebar")]
    TabsSidebar,
    #[serde(rename = "tabs-top")]
    TabsTop,
    #[serde(rename = "address-toolbar")]
    AddressToolbar,
    #[serde(rename = "address-sidebar")]
    AddressSidebar,
    #[serde(rename = "cmdl-field")]
    CmdlField,
    #[serde(rename = "cmdl-centred")]
    CmdlCentred,
    #[serde(rename = "newtab-page")]
    NewtabPage,
    #[serde(rename = "newtab-commandbar")]
    NewtabCommandbar,
    #[serde(rename = "newtabpos-bottom")]
    NewtabposBottom,
    #[serde(rename = "newtabpos-top")]
    NewtabposTop,
    #[serde(rename = "newtabpos-next")]
    NewtabposNext,
    #[serde(rename = "tidy-keep")]
    TidyKeep,
    #[serde(rename = "tidy-auto")]
    TidyAuto,
    #[serde(rename = "quit-restore")]
    QuitRestore,
    #[serde(rename = "quit-fresh")]
    QuitFresh,
    #[serde(rename = "windows-separate")]
    WindowsSeparate,
    #[serde(rename = "windows-shared")]
    WindowsShared,
    #[serde(rename = "appearance-auto")]
    AppearanceAuto,
    #[serde(rename = "appearance-light")]
    AppearanceLight,
    #[serde(rename = "appearance-dark")]
    AppearanceDark,
    #[serde(rename = "style-glass")]
    StyleGlass,
    #[serde(rename = "style-classic")]
    StyleClassic,
    #[serde(rename = "sitecolor-on")]
    SitecolorOn,
    #[serde(rename = "sitecolor-off")]
    SitecolorOff,
    #[serde(rename = "block-on")]
    BlockOn,
    #[serde(rename = "block-off")]
    BlockOff,
    #[serde(rename = "links-little")]
    LinksLittle,
    #[serde(rename = "links-tab")]
    LinksTab,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum GalleryPage {
    #[serde(rename = "frames")]
    Frames {
        index: Number,
        #[serde(
            rename = "addressInSidebar",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        address_in_sidebar: Option<bool>,
    },
    #[serde(rename = "mock")]
    Mock { index: Number },
    #[serde(rename = "mockLive")]
    MockLive,
}

impl Default for GalleryPage {
    fn default() -> Self {
        Self::Frames {
            index: Default::default(),
            address_in_sidebar: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PluginsStepPhase {
    #[default]
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "installed")]
    Installed,
    #[serde(rename = "offer")]
    Offer,
    #[serde(rename = "working")]
    Working,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StatePicked {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<Vec<String>>,
    #[serde(rename = "newTab", default, skip_serializing_if = "Option::is_none")]
    pub new_tab: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pins: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Vec<String>>,
    #[serde(
        rename = "newTabPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_position: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outro: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmdl: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tidy: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quit: Option<Vec<String>>,
    #[serde(rename = "tabsMore", default, skip_serializing_if = "Option::is_none")]
    pub tabs_more: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Vec<String>>,
    #[serde(rename = "siteColor", default, skip_serializing_if = "Option::is_none")]
    pub site_color: Option<Vec<String>>,
    #[serde(rename = "lookMore", default, skip_serializing_if = "Option::is_none")]
    pub look_more: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crash: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Prefill {
    #[default]
    #[serde(rename = "preset")]
    Preset,
    #[serde(rename = "current")]
    Current,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BrowserSourceId {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "arc")]
    Arc,
    #[serde(rename = "dia")]
    Dia,
    #[serde(rename = "chrome")]
    Chrome,
    #[serde(rename = "safari")]
    Safari,
    #[serde(rename = "firefox")]
    Firefox,
    #[serde(rename = "zen")]
    Zen,
    #[serde(rename = "brave")]
    Brave,
    #[serde(rename = "edge")]
    Edge,
    #[serde(rename = "helium")]
    Helium,
    #[serde(rename = "vivaldi")]
    Vivaldi,
    #[serde(rename = "opera")]
    Opera,
    #[serde(rename = "fresh")]
    Fresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateRedoSource {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "arc")]
    Arc,
    #[serde(rename = "dia")]
    Dia,
    #[serde(rename = "chrome")]
    Chrome,
    #[serde(rename = "safari")]
    Safari,
    #[serde(rename = "firefox")]
    Firefox,
    #[serde(rename = "zen")]
    Zen,
    #[serde(rename = "brave")]
    Brave,
    #[serde(rename = "edge")]
    Edge,
    #[serde(rename = "helium")]
    Helium,
    #[serde(rename = "vivaldi")]
    Vivaldi,
    #[serde(rename = "opera")]
    Opera,
    #[serde(rename = "fresh")]
    Fresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateSource {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "arc")]
    Arc,
    #[serde(rename = "dia")]
    Dia,
    #[serde(rename = "chrome")]
    Chrome,
    #[serde(rename = "safari")]
    Safari,
    #[serde(rename = "firefox")]
    Firefox,
    #[serde(rename = "zen")]
    Zen,
    #[serde(rename = "brave")]
    Brave,
    #[serde(rename = "edge")]
    Edge,
    #[serde(rename = "helium")]
    Helium,
    #[serde(rename = "vivaldi")]
    Vivaldi,
    #[serde(rename = "opera")]
    Opera,
    #[serde(rename = "fresh")]
    Fresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OnboardingStep {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "layout")]
    Layout,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "pins")]
    Pins,
    #[serde(rename = "intro")]
    Intro,
    #[serde(rename = "source")]
    Source,
    #[serde(rename = "look")]
    Look,
    #[serde(rename = "searchPrivacy")]
    SearchPrivacy,
    #[serde(rename = "home")]
    Home,
    #[serde(rename = "outro")]
    Outro,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TourStop {
    pub id: String,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<TourStopAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<TourStopPlacement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pad: Option<Number>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TourStopAnchor {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "commandBar")]
    CommandBar,
    #[serde(rename = "sidebarButton")]
    SidebarButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TourStopPlacement {
    #[default]
    #[serde(rename = "below")]
    Below,
    #[serde(rename = "besideSidebar")]
    BesideSidebar,
    #[serde(rename = "inside")]
    Inside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ChooseSourceSource {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "arc")]
    Arc,
    #[serde(rename = "dia")]
    Dia,
    #[serde(rename = "chrome")]
    Chrome,
    #[serde(rename = "safari")]
    Safari,
    #[serde(rename = "firefox")]
    Firefox,
    #[serde(rename = "zen")]
    Zen,
    #[serde(rename = "brave")]
    Brave,
    #[serde(rename = "edge")]
    Edge,
    #[serde(rename = "helium")]
    Helium,
    #[serde(rename = "vivaldi")]
    Vivaldi,
    #[serde(rename = "opera")]
    Opera,
    #[serde(rename = "fresh")]
    Fresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum QuestionId {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "color")]
    Color,
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "pins")]
    Pins,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "newTabPosition")]
    NewTabPosition,
    #[serde(rename = "appearance")]
    Appearance,
    #[serde(rename = "engine")]
    Engine,
    #[serde(rename = "source")]
    Source,
    #[serde(rename = "outro")]
    Outro,
    #[serde(rename = "address")]
    Address,
    #[serde(rename = "cmdl")]
    Cmdl,
    #[serde(rename = "tidy")]
    Tidy,
    #[serde(rename = "quit")]
    Quit,
    #[serde(rename = "tabsMore")]
    TabsMore,
    #[serde(rename = "style")]
    Style,
    #[serde(rename = "frame")]
    Frame,
    #[serde(rename = "siteColor")]
    SiteColor,
    #[serde(rename = "lookMore")]
    LookMore,
    #[serde(rename = "crash")]
    Crash,
    #[serde(rename = "links")]
    Links,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FinishOnboardingOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tour: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PreviewColorColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetGalleryMockPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<StateDraft>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<SetGalleryMockPatchFocus>,
    #[serde(
        rename = "settingsPatch",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub settings_patch: Option<StateDraft>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetGalleryMockPatchFocus {
    #[default]
    #[serde(rename = "tabs-sidebar")]
    TabsSidebar,
    #[serde(rename = "tabs-top")]
    TabsTop,
    #[serde(rename = "address-toolbar")]
    AddressToolbar,
    #[serde(rename = "address-sidebar")]
    AddressSidebar,
    #[serde(rename = "cmdl-field")]
    CmdlField,
    #[serde(rename = "cmdl-centred")]
    CmdlCentred,
    #[serde(rename = "newtab-page")]
    NewtabPage,
    #[serde(rename = "newtab-commandbar")]
    NewtabCommandbar,
    #[serde(rename = "newtabpos-bottom")]
    NewtabposBottom,
    #[serde(rename = "newtabpos-top")]
    NewtabposTop,
    #[serde(rename = "newtabpos-next")]
    NewtabposNext,
    #[serde(rename = "tidy-keep")]
    TidyKeep,
    #[serde(rename = "tidy-auto")]
    TidyAuto,
    #[serde(rename = "quit-restore")]
    QuitRestore,
    #[serde(rename = "quit-fresh")]
    QuitFresh,
    #[serde(rename = "windows-separate")]
    WindowsSeparate,
    #[serde(rename = "windows-shared")]
    WindowsShared,
    #[serde(rename = "appearance-auto")]
    AppearanceAuto,
    #[serde(rename = "appearance-light")]
    AppearanceLight,
    #[serde(rename = "appearance-dark")]
    AppearanceDark,
    #[serde(rename = "style-glass")]
    StyleGlass,
    #[serde(rename = "style-classic")]
    StyleClassic,
    #[serde(rename = "sitecolor-on")]
    SitecolorOn,
    #[serde(rename = "sitecolor-off")]
    SitecolorOff,
    #[serde(rename = "block-on")]
    BlockOn,
    #[serde(rename = "block-off")]
    BlockOff,
    #[serde(rename = "links-little")]
    LinksLittle,
    #[serde(rename = "links-tab")]
    LinksTab,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub baseline: LiveBaseline,
    pub current: Option<String>,
    #[serde(rename = "doneAt")]
    pub done_at: Option<Number>,
    pub draft: StateDraft,
    pub extras: StateExtras,
    pub failures: std::collections::HashMap<String, String>,
    pub index: Number,
    pub installed: Vec<String>,
    pub leaving: bool,
    pub mock: LiveMock,
    pub page: GalleryPage,
    pub phase: PluginsStepPhase,
    pub picked: StatePicked,
    pub picks: std::collections::HashMap<String, bool>,
    pub prefill: Prefill,
    pub preset: BrowserSourceId,
    pub redo: bool,
    #[serde(rename = "redoSource")]
    pub redo_source: Option<StateRedoSource>,
    pub run: Number,
    pub session: Number,
    pub shown: Vec<String>,
    pub source: Option<StateSource>,
    pub step: OnboardingStep,
    pub stops: Vec<TourStop>,
    pub sub: Number,
    #[serde(rename = "tourAfter")]
    pub tour_after: bool,
    #[serde(rename = "trialStartedAt")]
    pub trial_started_at: Option<Number>,
    #[serde(rename = "windowId")]
    pub window_id: Option<String>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `adoptSetupChoices` (an exported function that writes a store).
    #[serde(rename = "adoptSetupChoices")]
    AdoptSetupChoices {
        choices: crate::presets::SetupChoices,
        lines: Vec<String>,
    },
    /// `cancelOnboarding` (an exported function that writes a store).
    #[serde(rename = "cancelOnboarding")]
    CancelOnboarding,
    /// `cancelPluginConsent` (an exported function that writes a store).
    #[serde(rename = "cancelPluginConsent")]
    CancelPluginConsent { id: Number },
    /// `chooseSource` (an exported function that writes a store).
    #[serde(rename = "chooseSource")]
    ChooseSource { source: Option<ChooseSourceSource> },
    /// `confirmAnswer` (an exported function that writes a store).
    #[serde(rename = "confirmAnswer")]
    ConfirmAnswer {
        question: QuestionId,
        patch: StateDraft,
        picked: Vec<String>,
    },
    /// `confirmDefaultBrowserAnswer` (an exported function that writes a store).
    #[serde(rename = "confirmDefaultBrowserAnswer")]
    ConfirmDefaultBrowserAnswer { picked: Vec<String> },
    /// `confirmPluginConsent` (an exported function that writes a store).
    #[serde(rename = "confirmPluginConsent")]
    ConfirmPluginConsent { id: Number },
    /// `dismissDefaultBrowserCheckIn` (an exported function that writes a store).
    #[serde(rename = "dismissDefaultBrowserCheckIn")]
    DismissDefaultBrowserCheckIn,
    /// `dismissOnboarding` (an exported function that writes a store).
    #[serde(rename = "dismissOnboarding")]
    DismissOnboarding { session: Number },
    /// `endToolTour` (an exported function that writes a store).
    #[serde(rename = "endToolTour")]
    EndToolTour,
    /// `finishOnboarding` (an exported function that writes a store).
    #[serde(rename = "finishOnboarding")]
    FinishOnboarding {
        #[serde(default)]
        options: Option<FinishOnboardingOptions>,
    },
    /// `finishPluginsStep` (an exported function that writes a store).
    #[serde(rename = "finishPluginsStep")]
    FinishPluginsStep {
        plugins: Vec<crate::plugins::OfficialPlugin>,
    },
    /// `goToStep` (an exported function that writes a store).
    #[serde(rename = "goToStep")]
    GoToStep {
        step: OnboardingStep,
        #[serde(default)]
        sub: Option<Number>,
    },
    /// `installPickedPlugins` (an exported function that writes a store).
    #[serde(rename = "installPickedPlugins")]
    InstallPickedPlugins {
        plugins: Vec<crate::plugins::OfficialPlugin>,
    },
    /// `installPluginAgain` (an exported function that writes a store).
    #[serde(rename = "installPluginAgain")]
    InstallPluginAgain { name: String },
    /// `leavePluginsStep` (an exported function that writes a store).
    #[serde(rename = "leavePluginsStep")]
    LeavePluginsStep,
    /// `maybeStartOnboarding` (an exported function that writes a store).
    #[serde(rename = "maybeStartOnboarding")]
    MaybeStartOnboarding,
    /// `nextStep` (an exported function that writes a store).
    #[serde(rename = "nextStep")]
    NextStep,
    /// `notePluginsShown` (an exported function that writes a store).
    #[serde(rename = "notePluginsShown")]
    NotePluginsShown { names: Vec<String> },
    /// `openVideoTour` (an exported function that writes a store).
    #[serde(rename = "openVideoTour")]
    OpenVideoTour {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `passEmptyPluginsStep` (an exported function that writes a store).
    #[serde(rename = "passEmptyPluginsStep")]
    PassEmptyPluginsStep,
    /// `pinSites` (an exported function that writes a store).
    #[serde(rename = "pinSites")]
    PinSites {
        #[serde(rename = "windowId")]
        window_id: String,
        ids: Vec<String>,
    },
    /// `previewColor` (an exported function that writes a store).
    #[serde(rename = "previewColor")]
    PreviewColor { color: PreviewColorColor },
    /// `previewLive` (an exported function that writes a store).
    #[serde(rename = "previewLive")]
    PreviewLive { patch: StateDraft },
    /// `previewTheme` (an exported function that writes a store).
    #[serde(rename = "previewTheme")]
    PreviewTheme { theme: serde_json::Value },
    /// `previousStep` (an exported function that writes a store).
    #[serde(rename = "previousStep")]
    PreviousStep,
    /// `redoOnboarding` (an exported function that writes a store).
    #[serde(rename = "redoOnboarding")]
    RedoOnboarding {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `redoSetupFromSettings` (an exported function that writes a store).
    #[serde(rename = "redoSetupFromSettings")]
    RedoSetupFromSettings,
    /// `resetOnboarding` (an exported function that writes a store).
    #[serde(rename = "resetOnboarding")]
    ResetOnboarding {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `resetPluginsStep` (an exported function that writes a store).
    #[serde(rename = "resetPluginsStep")]
    ResetPluginsStep,
    /// `setGalleryMock` (an exported function that writes a store).
    #[serde(rename = "setGalleryMock")]
    SetGalleryMock { patch: SetGalleryMockPatch },
    /// `setOnboardingDraft` (an exported function that writes a store).
    #[serde(rename = "setOnboardingDraft")]
    SetOnboardingDraft { patch: StateDraft },
    /// `setOnboardingExtras` (an exported function that writes a store).
    #[serde(rename = "setOnboardingExtras")]
    SetOnboardingExtras { patch: StateExtras },
    /// `showGalleryPage` (an exported function that writes a store).
    #[serde(rename = "showGalleryPage")]
    ShowGalleryPage { page: GalleryPage },
    /// `skipPluginsStep` (an exported function that writes a store).
    #[serde(rename = "skipPluginsStep")]
    SkipPluginsStep {
        plugins: Vec<crate::plugins::OfficialPlugin>,
    },
    /// `skipStep` (an exported function that writes a store).
    #[serde(rename = "skipStep")]
    SkipStep,
    /// `startOnboarding` (an exported function that writes a store).
    #[serde(rename = "startOnboarding")]
    StartOnboarding {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
        #[serde(default)]
        step: Option<OnboardingStep>,
    },
    /// `startToolTour` (an exported function that writes a store).
    #[serde(rename = "startToolTour")]
    StartToolTour {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `stepToolTour` (an exported function that writes a store).
    #[serde(rename = "stepToolTour")]
    StepToolTour { delta: Number },
    /// `togglePluginPick` (an exported function that writes a store).
    #[serde(rename = "togglePluginPick")]
    TogglePluginPick {
        plugin: crate::plugins::OfficialPlugin,
    },
}

/// Applies `action` to `app`. Slice 2.28 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.28")
}
