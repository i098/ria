// Ported from ardia apps/browser/src/state/settings/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `settings` (`apps/browser/src/state/settings`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.7 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

mod reducer;
#[cfg(test)]
mod replay_tests;
pub mod specs;

pub use reducer::{OrderedPatch, Run};

use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "cliControl")]
    pub cli_control: bool,
    #[serde(rename = "warnBeforeQuitting")]
    pub warn_before_quitting: bool,
    #[serde(rename = "warnBeforeClosingWindow")]
    pub warn_before_closing_window: bool,
    #[serde(rename = "restoreSession")]
    pub restore_session: bool,
    #[serde(rename = "separateDownloadFolders")]
    pub separate_download_folders: bool,
    #[serde(rename = "searchEngine")]
    pub search_engine: String,
    #[serde(rename = "customSearchUrl")]
    pub custom_search_url: String,
    #[serde(rename = "customSearchEngines")]
    pub custom_search_engines: Vec<serde_json::Value>,
    #[serde(rename = "extensionSearchEngines")]
    pub extension_search_engines: Vec<serde_json::Value>,
    #[serde(rename = "searchSuggestions")]
    pub search_suggestions: bool,
    #[serde(rename = "commandBarHistory")]
    pub command_bar_history: bool,
    #[serde(rename = "showSelectionSearch")]
    pub show_selection_search: bool,
    #[serde(rename = "tabLayout")]
    pub tab_layout: SettingsTabLayout,
    #[serde(rename = "sidebarSide")]
    pub sidebar_side: SettingsSidebarSide,
    #[serde(rename = "autoHideTabs")]
    pub auto_hide_tabs: bool,
    #[serde(rename = "hideToolbarWhileScrolling")]
    pub hide_toolbar_while_scrolling: bool,
    #[serde(rename = "newTabPosition")]
    pub new_tab_position: SettingsNewTabPosition,
    #[serde(rename = "newTabButtonPosition")]
    pub new_tab_button_position: SettingsNewTabButtonPosition,
    #[serde(rename = "appLinksFirst")]
    pub app_links_first: bool,
    #[serde(rename = "newTabOpens")]
    pub new_tab_opens: SettingsNewTabOpens,
    #[serde(rename = "newTabUrl")]
    pub new_tab_url: String,
    #[serde(rename = "warnBeforeClosingLastTab")]
    pub warn_before_closing_last_tab: bool,
    #[serde(rename = "keepTopAppsRunning")]
    pub keep_top_apps_running: bool,
    #[serde(rename = "tabReorderHaptics")]
    pub tab_reorder_haptics: bool,
    #[serde(rename = "swipeBetweenTabs")]
    pub swipe_between_tabs: bool,
    #[serde(rename = "undoReopensClosedTab")]
    pub undo_reopens_closed_tab: bool,
    #[serde(rename = "warnBeforeMovingTabsToProfile")]
    pub warn_before_moving_tabs_to_profile: bool,
    #[serde(rename = "shareTabsAcrossWindows")]
    pub share_tabs_across_windows: bool,
    #[serde(rename = "extendWebsiteColor")]
    pub extend_website_color: bool,
    #[serde(rename = "cmdClickCreatesTabGroup")]
    pub cmd_click_creates_tab_group: bool,
    #[serde(rename = "optShiftClickOpensInGroup")]
    pub opt_shift_click_opens_in_group: bool,
    #[serde(rename = "peekModifier")]
    pub peek_modifier: SettingsPeekModifier,
    #[serde(rename = "peekLinksToOtherSites")]
    pub peek_links_to_other_sites: bool,
    #[serde(rename = "newTabLinksOpenIn")]
    pub new_tab_links_open_in: SettingsNewTabLinksOpenIn,
    #[serde(rename = "popupsOpenAsTabs")]
    pub popups_open_as_tabs: bool,
    #[serde(rename = "peekButtons")]
    pub peek_buttons: SettingsPeekButtons,
    #[serde(rename = "splitLinkModifier")]
    pub split_link_modifier: SettingsSplitLinkModifier,
    #[serde(rename = "autoGroupMeetingTabs")]
    pub auto_group_meeting_tabs: bool,
    #[serde(rename = "cleanUpInactiveTabsAfterMinutes")]
    pub clean_up_inactive_tabs_after_minutes: Option<Number>,
    #[serde(rename = "suggestCleanUp")]
    pub suggest_clean_up: bool,
    #[serde(rename = "mutedSites")]
    pub muted_sites: Vec<String>,
    #[serde(rename = "showUnreadCounts")]
    pub show_unread_counts: bool,
    #[serde(rename = "showTopAppBadges")]
    pub show_top_app_badges: bool,
    #[serde(rename = "showTabHoverMenu")]
    pub show_tab_hover_menu: bool,
    #[serde(rename = "showFullUrlInTabHoverMenu")]
    pub show_full_url_in_tab_hover_menu: bool,
    #[serde(rename = "closedTopAppTiles")]
    pub closed_top_app_tiles: bool,
    #[serde(rename = "topAppFolders")]
    pub top_app_folders: bool,
    #[serde(rename = "unreadHiddenSites")]
    pub unread_hidden_sites: Vec<String>,
    #[serde(rename = "neverTranslateSites")]
    pub never_translate_sites: std::collections::HashMap<String, Vec<String>>,
    #[serde(rename = "developerModeSites")]
    pub developer_mode_sites: std::collections::HashMap<String, bool>,
    #[serde(rename = "autoPictureInPicture")]
    pub auto_picture_in_picture: bool,
    #[serde(rename = "playingNowAcrossProfiles")]
    pub playing_now_across_profiles: bool,
    #[serde(rename = "sidebarMediaControls")]
    pub sidebar_media_controls: bool,
    #[serde(rename = "commandScrollZoom")]
    pub command_scroll_zoom: bool,
    #[serde(rename = "blockAutoplayEverywhere")]
    pub block_autoplay_everywhere: bool,
    #[serde(rename = "blockNotificationRequests")]
    pub block_notification_requests: bool,
    #[serde(rename = "videoUpscaling")]
    pub video_upscaling: bool,
    #[serde(rename = "sidebarWidth")]
    pub sidebar_width: Number,
    #[serde(rename = "showSidebarButton")]
    pub show_sidebar_button: bool,
    #[serde(rename = "showClearButton")]
    pub show_clear_button: bool,
    #[serde(rename = "tidyTabs")]
    pub tidy_tabs: bool,
    #[serde(rename = "balancedTopApps")]
    pub balanced_top_apps: bool,
    #[serde(rename = "selectedTabGlow")]
    pub selected_tab_glow: SettingsSelectedTabGlow,
    #[serde(rename = "interfaceScale")]
    pub interface_scale: Number,
    #[serde(rename = "sidebarTextSize")]
    pub sidebar_text_size: SettingsSidebarTextSize,
    #[serde(rename = "sidebarTextContrast")]
    pub sidebar_text_contrast: SettingsSidebarTextContrast,
    #[serde(rename = "selectedTopApp")]
    pub selected_top_app: SettingsSelectedTopApp,
    #[serde(rename = "elementOutlines")]
    pub element_outlines: bool,
    #[serde(rename = "interfaceStyle")]
    pub interface_style: SettingsInterfaceStyle,
    #[serde(rename = "windowButtons")]
    pub window_buttons: SettingsWindowButtons,
    #[serde(rename = "glassTintWithTheme")]
    pub glass_tint_with_theme: bool,
    #[serde(rename = "glassTintAmount")]
    pub glass_tint_amount: Number,
    #[serde(rename = "fadeUnloadedTabs")]
    pub fade_unloaded_tabs: bool,
    #[serde(rename = "strikeSleepingTabs")]
    pub strike_sleeping_tabs: bool,
    #[serde(rename = "warnBeforeClosingTabGroup")]
    pub warn_before_closing_tab_group: bool,
    #[serde(rename = "batterySaverWhen")]
    pub battery_saver_when: SettingsBatterySaverWhen,
    #[serde(rename = "unloadProfilesAfter")]
    pub unload_profiles_after: SettingsUnloadProfilesAfter,
    #[serde(rename = "memorySaver")]
    pub memory_saver: bool,
    #[serde(rename = "memorySaverMode")]
    pub memory_saver_mode: SettingsMemorySaverMode,
    #[serde(rename = "loadProfilesWhenOpened")]
    pub load_profiles_when_opened: bool,
    #[serde(rename = "keepActiveSites")]
    pub keep_active_sites: Vec<String>,
    #[serde(rename = "extensionSidePanelWidth")]
    pub extension_side_panel_width: Number,
    #[serde(rename = "showFullUrl")]
    pub show_full_url: bool,
    #[serde(rename = "allowJavaScriptFromAppleEvents")]
    pub allow_java_script_from_apple_events: bool,
    #[serde(rename = "compactCliPill")]
    pub compact_cli_pill: bool,
    #[serde(rename = "showBookmarks")]
    pub show_bookmarks: bool,
    #[serde(rename = "bookmarksBar")]
    pub bookmarks_bar: SettingsBookmarksBar,
    #[serde(rename = "bookmarksImportDismissed")]
    pub bookmarks_import_dismissed: bool,
    pub appearance: SettingsAppearance,
    #[serde(rename = "addressBar")]
    pub address_bar: SettingsAddressBar,
    #[serde(rename = "compactToolbar")]
    pub compact_toolbar: bool,
    #[serde(rename = "addressBarButtons")]
    pub address_bar_buttons: SettingsAddressBarButtons,
    #[serde(rename = "pageFrame")]
    pub page_frame: Number,
    #[serde(rename = "borderlessMode")]
    pub borderless_mode: bool,
    #[serde(rename = "tabCornerRadius")]
    pub tab_corner_radius: Option<Number>,
    #[serde(rename = "minimalismMode")]
    pub minimalism_mode: bool,
    #[serde(rename = "pageFrameZeroInFullScreen")]
    pub page_frame_zero_in_full_screen: bool,
    #[serde(rename = "pageFrameZeroWhenTabsHidden")]
    pub page_frame_zero_when_tabs_hidden: bool,
    #[serde(rename = "commandBarPlacement")]
    pub command_bar_placement: SettingsCommandBarPlacement,
    #[serde(rename = "commandBarStart")]
    pub command_bar_start: crate::omnibox::CommandBarStart,
    #[serde(rename = "pressMotion")]
    pub press_motion: SettingsPressMotion,
    #[serde(rename = "folderStyle")]
    pub folder_style: SettingsFolderStyle,
    #[serde(rename = "liveFolderIcon")]
    pub live_folder_icon: SettingsLiveFolderIcon,
    #[serde(rename = "spacesFollowAppearance")]
    pub spaces_follow_appearance: bool,
    #[serde(rename = "adaptTextToColor")]
    pub adapt_text_to_color: bool,
    #[serde(rename = "defaultProfileId")]
    pub default_profile_id: String,
    #[serde(rename = "showProfileDots")]
    pub show_profile_dots: bool,
    pub shortcuts: std::collections::HashMap<String, Vec<String>>,
    #[serde(rename = "mouseShortcuts")]
    pub mouse_shortcuts: std::collections::HashMap<String, Vec<String>>,
    #[serde(rename = "openLinksInLittleArdia")]
    pub open_links_in_little_ardia: bool,
    #[serde(rename = "littleArdiaOpensNewTabLinks")]
    pub little_ardia_opens_new_tab_links: bool,
    #[serde(rename = "littleArdiaShortcutFromAnyApp")]
    pub little_ardia_shortcut_from_any_app: bool,
    #[serde(rename = "littleArdiaSize")]
    pub little_ardia_size: (Number, Number),
    #[serde(rename = "engineMode")]
    pub engine_mode: SettingsEngineMode,
    #[serde(rename = "pageEngine")]
    pub page_engine: SettingsPageEngine,
    #[serde(rename = "websiteEngines")]
    pub website_engines: std::collections::HashMap<String, SettingsWebsiteEnginesValue>,
    #[serde(rename = "protectedVideoInWebKit")]
    pub protected_video_in_web_kit: bool,
    #[serde(rename = "offerWebKitForProtectedVideo")]
    pub offer_web_kit_for_protected_video: bool,
    #[serde(rename = "engineSwitchKeepsPage")]
    pub engine_switch_keeps_page: bool,
    #[serde(rename = "engineSwitchAsksAboutUnsavedWork")]
    pub engine_switch_asks_about_unsaved_work: bool,
    #[serde(rename = "engineSwitchCarriesSiteData")]
    pub engine_switch_carries_site_data: bool,
    #[serde(rename = "webKitFraudWarnings")]
    pub web_kit_fraud_warnings: bool,
    #[serde(rename = "launchActions")]
    pub launch_actions: Vec<String>,
    #[serde(rename = "omniboxProviders")]
    pub omnibox_providers: Vec<serde_json::Value>,
    #[serde(rename = "readerTextSize")]
    pub reader_text_size: SettingsReaderTextSize,
    #[serde(rename = "readerFont")]
    pub reader_font: SettingsReaderFont,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsTabLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsSidebarSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsNewTabPosition {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "nextToCurrent")]
    NextToCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsNewTabButtonPosition {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsNewTabOpens {
    #[default]
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "commandBar")]
    CommandBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsPeekModifier {
    #[default]
    #[serde(rename = "alt")]
    Alt,
    #[serde(rename = "shift")]
    Shift,
    #[serde(rename = "off")]
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsNewTabLinksOpenIn {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "littleArdia")]
    LittleArdia,
    #[serde(rename = "peek")]
    Peek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsPeekButtons {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsSplitLinkModifier {
    #[default]
    #[serde(rename = "alt")]
    Alt,
    #[serde(rename = "altShift")]
    AltShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsSelectedTabGlow {
    #[default]
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "soft")]
    Soft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsSidebarTextSize {
    #[default]
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "large")]
    Large,
    #[serde(rename = "larger")]
    Larger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsSidebarTextContrast {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "standard")]
    Standard,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsSelectedTopApp {
    #[default]
    #[serde(rename = "ring")]
    Ring,
    #[serde(rename = "fill")]
    Fill,
    #[serde(rename = "outline")]
    Outline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsInterfaceStyle {
    #[default]
    #[serde(rename = "glass")]
    Glass,
    #[serde(rename = "classic")]
    Classic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsWindowButtons {
    #[default]
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "trafficLights")]
    TrafficLights,
    #[serde(rename = "hidden")]
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsBatterySaverWhen {
    #[default]
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "lowBattery")]
    LowBattery,
    #[serde(rename = "unplugged")]
    Unplugged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsUnloadProfilesAfter {
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
pub enum SettingsMemorySaverMode {
    #[default]
    #[serde(rename = "moderate")]
    Moderate,
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "maximum")]
    Maximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsBookmarksBar {
    #[default]
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "always")]
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsAppearance {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsAddressBar {
    #[default]
    #[serde(rename = "toolbar")]
    Toolbar,
    #[serde(rename = "sidebar")]
    Sidebar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsAddressBarButtons {
    #[default]
    #[serde(rename = "hover")]
    Hover,
    #[serde(rename = "always")]
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsCommandBarPlacement {
    #[default]
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "centred")]
    Centred,
    #[serde(rename = "centredOnClick")]
    CentredOnClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsPressMotion {
    #[default]
    #[serde(rename = "still")]
    Still,
    #[serde(rename = "bounce")]
    Bounce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsFolderStyle {
    #[default]
    #[serde(rename = "neutral")]
    Neutral,
    #[serde(rename = "profile")]
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsLiveFolderIcon {
    #[default]
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "service")]
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsEngineMode {
    #[default]
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsPageEngine {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsWebsiteEnginesValue {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsReaderTextSize {
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
pub enum SettingsReaderFont {
    #[default]
    #[serde(rename = "serif")]
    Serif,
    #[serde(rename = "sans")]
    Sans,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SettingsMeta {
    pub chosen: Vec<String>,
    pub inferred: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SettingsPlace {
    pub pane: SettingsPlacePane,
    #[serde(rename = "profileId")]
    pub profile_id: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SettingsPlacePane {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "appearance")]
    Appearance,
    #[serde(rename = "shortcuts")]
    Shortcuts,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "automations")]
    Automations,
    #[serde(rename = "advanced")]
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateFlash {
    pub id: String,
    pub at: Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StatePane {
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "appearance")]
    Appearance,
    #[serde(rename = "shortcuts")]
    Shortcuts,
    #[default]
    #[serde(rename = "general")]
    General,
    #[serde(rename = "automations")]
    Automations,
    #[serde(rename = "advanced")]
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateReveal {
    pub id: String,
    pub at: Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ChooseAddressBarAddressBar {
    #[default]
    #[serde(rename = "toolbar")]
    Toolbar,
    #[serde(rename = "sidebar")]
    Sidebar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ChooseCommandBarPlacementCommandBarPlacement {
    #[default]
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "centred")]
    Centred,
    #[serde(rename = "centredOnClick")]
    CentredOnClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ChooseSidebarSideSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ChooseTabLayoutLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DropSettingsSearchWhy {
    #[default]
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "dropped")]
    Dropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GoToSettingsPanePane {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "appearance")]
    Appearance,
    #[serde(rename = "shortcuts")]
    Shortcuts,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "automations")]
    Automations,
    #[serde(rename = "advanced")]
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Target {
    pub id: String,
    pub pane: TargetPane,
    pub section: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TargetPane {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "appearance")]
    Appearance,
    #[serde(rename = "shortcuts")]
    Shortcuts,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "automations")]
    Automations,
    #[serde(rename = "advanced")]
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LoadR {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LoadArg2 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fresh: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LoadAllNeededItemItem0 {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpenSettingsPane {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "appearance")]
    Appearance,
    #[serde(rename = "shortcuts")]
    Shortcuts,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "automations")]
    Automations,
    #[serde(rename = "advanced")]
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PreparePanePane {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "appearance")]
    Appearance,
    #[serde(rename = "shortcuts")]
    Shortcuts,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "automations")]
    Automations,
    #[serde(rename = "advanced")]
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RevealCommandKeysVia {
    #[default]
    #[serde(rename = "keyboard")]
    Keyboard,
    #[serde(rename = "click")]
    Click,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RevealSettingVia {
    #[default]
    #[serde(rename = "keyboard")]
    Keyboard,
    #[serde(rename = "click")]
    Click,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetAutofillForKind {
    #[default]
    #[serde(rename = "addresses")]
    Addresses,
    #[serde(rename = "cards")]
    Cards,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetDownloadsForPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<serde_json::Value>,
    #[serde(
        rename = "askWhereToSave",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ask_where_to_save: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetUpdaterPrefsPatch {
    #[serde(
        rename = "automaticChecks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub automatic_checks: Option<bool>,
    #[serde(
        rename = "automaticDownloads",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub automatic_downloads: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StoreR {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateSettingsOptions {
    #[serde(rename = "fromSync", default, skip_serializing_if = "Option::is_none")]
    pub from_sync: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inferred: Option<bool>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub back: Vec<SettingsPlace>,
    pub detail: Option<String>,
    pub flash: Option<StateFlash>,
    #[serde(rename = "focusRequest")]
    pub focus_request: Number,
    pub forward: Vec<SettingsPlace>,
    #[serde(rename = "keyboardMove")]
    pub keyboard_move: Number,
    pub pane: StatePane,
    pub previewing: bool,
    #[serde(rename = "previewSection")]
    pub preview_section: Option<String>,
    #[serde(rename = "profileId")]
    pub profile_id: Option<String>,
    pub query: String,
    pub reached: Number,
    pub reveal: Option<StateReveal>,
    #[serde(rename = "revealedSections")]
    pub revealed_sections: Vec<String>,
    pub scale: Number,
    pub selected: Number,
    pub settings: Settings,
    #[serde(rename = "settingsMeta")]
    pub settings_meta: SettingsMeta,
    pub sheet: serde_json::Value,
    #[serde(rename = "shortcutsQuery")]
    pub shortcuts_query: String,
    pub source: Option<String>,
    /// Runtime fields of the module-private variables of the TypeScript files. They are not part of the store state, so
    /// they are not in its JSON.
    #[serde(skip)]
    pub run: Run,
}

/// The spec defaults (`settings.ts` `SETTING_SPECS`), not the zero value of each field.
impl Default for Settings {
    fn default() -> Self {
        serde_json::from_value(serde_json::Value::Object(specs::default_settings_map()))
            .expect("spec defaults fit Settings")
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            back: Vec::new(),
            detail: None,
            flash: None,
            focus_request: Number(0.0),
            forward: Vec::new(),
            keyboard_move: Number(0.0),
            pane: StatePane::default(),
            previewing: false,
            preview_section: None,
            profile_id: None,
            query: String::new(),
            reached: Number(0.0),
            reveal: None,
            revealed_sections: Vec::new(),
            scale: Number(1.0),
            selected: Number(0.0),
            settings: Settings::default(),
            // `store.ts` runs `withToolbarKeptWhileScrolling` on a new store, which marks the old default as applied.
            settings_meta: SettingsMeta {
                chosen: Vec::new(),
                inferred: vec!["hideToolbarWhileScrolling:offByDefault".to_owned()],
            },
            sheet: serde_json::Value::Null,
            shortcuts_query: String::new(),
            source: None,
            run: Run::default(),
        }
    }
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `addCustomEngine` (an exported function that writes a store).
    #[serde(rename = "addCustomEngine")]
    AddCustomEngine {
        input: serde_json::Value,
        #[serde(default)]
        now: Number,
    },
    /// `appliedInterfaceScale` (an exported function that writes a store).
    #[serde(rename = "appliedInterfaceScale")]
    AppliedInterfaceScale { scale: Number },
    /// `chooseAddressBar` (an exported function that writes a store).
    #[serde(rename = "chooseAddressBar")]
    ChooseAddressBar {
        #[serde(rename = "addressBar")]
        address_bar: ChooseAddressBarAddressBar,
    },
    /// `chooseAppIcon` (an exported function that writes a store).
    #[serde(rename = "chooseAppIcon")]
    ChooseAppIcon { id: String },
    /// `chooseCommandBarPlacement` (an exported function that writes a store).
    #[serde(rename = "chooseCommandBarPlacement")]
    ChooseCommandBarPlacement {
        #[serde(rename = "commandBarPlacement")]
        command_bar_placement: ChooseCommandBarPlacementCommandBarPlacement,
    },
    /// `chooseInterfaceScale` (an exported function that writes a store).
    #[serde(rename = "chooseInterfaceScale")]
    ChooseInterfaceScale { scale: Number },
    /// `chooseScrollToolbar` (an exported function that writes a store).
    #[serde(rename = "chooseScrollToolbar")]
    ChooseScrollToolbar {
        #[serde(rename = "hideToolbarWhileScrolling")]
        hide_toolbar_while_scrolling: bool,
    },
    /// `chooseSidebarSide` (an exported function that writes a store).
    #[serde(rename = "chooseSidebarSide")]
    ChooseSidebarSide { side: ChooseSidebarSideSide },
    /// `chooseTabLayout` (an exported function that writes a store).
    #[serde(rename = "chooseTabLayout")]
    ChooseTabLayout { layout: ChooseTabLayoutLayout },
    /// `clearSettingsReveal` (an exported function that writes a store).
    #[serde(rename = "clearSettingsReveal")]
    ClearSettingsReveal,
    /// `closeSettingsSheet` (an exported function that writes a store).
    #[serde(rename = "closeSettingsSheet")]
    CloseSettingsSheet,
    /// `dropSettingsSearch` (an exported function that writes a store).
    #[serde(rename = "dropSettingsSearch")]
    DropSettingsSearch {
        consent: Option<crate::telemetry::ConsentToken>,
        #[serde(default)]
        why: Option<DropSettingsSearchWhy>,
    },
    /// `flashSetting` (an exported function that writes a store).
    #[serde(rename = "flashSetting")]
    FlashSetting {
        id: String,
        #[serde(default)]
        now: Number,
    },
    /// `focusSettingsSearch` (an exported function that writes a store).
    #[serde(rename = "focusSettingsSearch")]
    FocusSettingsSearch {
        #[serde(default)]
        now: Number,
    },
    /// `followUpdater` (an exported function that writes a store). The callback parameters `settled` are not in the JSON.
    #[serde(rename = "followUpdater")]
    FollowUpdater,
    /// `goToSettingsPane` (an exported function that writes a store).
    #[serde(rename = "goToSettingsPane")]
    GoToSettingsPane {
        pane: GoToSettingsPanePane,
        #[serde(rename = "profileId", default)]
        profile_id: Option<String>,
        #[serde(default)]
        detail: Option<String>,
    },
    /// `jumpToSetting` (an exported function that writes a store).
    #[serde(rename = "jumpToSetting")]
    JumpToSetting {
        #[serde(rename = "target")]
        arg0: Target,
        #[serde(default)]
        now: Number,
    },
    /// `load` (an exported function that writes a store).
    #[serde(rename = "load")]
    Load {
        r: LoadR,
        args: serde_json::Value,
        #[serde(default)]
        arg2: Option<LoadArg2>,
    },
    /// `loadAll` (an exported function that writes a store).
    #[serde(rename = "loadAll")]
    LoadAll {
        needed: Vec<(LoadAllNeededItemItem0, Vec<serde_json::Value>)>,
    },
    /// `makeDefaultBrowser` (an exported function that writes a store).
    #[serde(rename = "makeDefaultBrowser")]
    MakeDefaultBrowser,
    /// `moveSettingsSelection` (an exported function that writes a store).
    #[serde(rename = "moveSettingsSelection")]
    MoveSettingsSelection { step: Number, count: Number },
    /// `noteSettingsScroll` (an exported function that writes a store).
    #[serde(rename = "noteSettingsScroll")]
    NoteSettingsScroll {
        offset: Number,
        viewport: Number,
        content: Number,
    },
    /// `openImport` (an exported function that writes a store).
    #[serde(rename = "openImport")]
    OpenImport {
        #[serde(rename = "browserId", default)]
        browser_id: Option<String>,
    },
    /// `openSettings` (an exported function that writes a store).
    #[serde(rename = "openSettings")]
    OpenSettings {
        #[serde(default)]
        pane: Option<OpenSettingsPane>,
        #[serde(rename = "profileId", default)]
        profile_id: Option<String>,
        #[serde(default)]
        detail: Option<String>,
    },
    /// `openSiteSettings` (an exported function that writes a store).
    #[serde(rename = "openSiteSettings")]
    OpenSiteSettings {
        origin: String,
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `openUrlFromSettings` (an exported function that writes a store).
    #[serde(rename = "openUrlFromSettings")]
    OpenUrlFromSettings { url: String },
    /// `preparePane` (an exported function that writes a store).
    #[serde(rename = "preparePane")]
    PreparePane {
        pane: PreparePanePane,
        #[serde(rename = "profileId")]
        profile_id: Option<String>,
    },
    /// `previewCommandKeys` (an exported function that writes a store).
    #[serde(rename = "previewCommandKeys")]
    PreviewCommandKeys {
        keys: String,
        #[serde(default)]
        now: Number,
    },
    /// `previewSetting` (an exported function that writes a store).
    #[serde(rename = "previewSetting")]
    PreviewSetting {
        #[serde(rename = "target")]
        arg0: Target,
        #[serde(default)]
        now: Number,
    },
    /// `removeCustomEngine` (an exported function that writes a store).
    #[serde(rename = "removeCustomEngine")]
    RemoveCustomEngine { id: String },
    /// `replaceSettingsPlace` (an exported function that writes a store).
    #[serde(rename = "replaceSettingsPlace")]
    ReplaceSettingsPlace {
        #[serde(rename = "profileId")]
        profile_id: Option<String>,
        #[serde(default)]
        detail: Option<String>,
    },
    /// `revealCommandKeys` (an exported function that writes a store).
    #[serde(rename = "revealCommandKeys")]
    RevealCommandKeys {
        keys: String,
        via: RevealCommandKeysVia,
        rank: Number,
        #[serde(default)]
        now: Number,
    },
    /// `revealSetting` (an exported function that writes a store).
    #[serde(rename = "revealSetting")]
    RevealSetting {
        target: Target,
        via: RevealSettingVia,
        rank: Number,
        #[serde(default)]
        unshown: Option<bool>,
        #[serde(default)]
        now: Number,
    },
    /// `selectSettingsResult` (an exported function that writes a store).
    #[serde(rename = "selectSettingsResult")]
    SelectSettingsResult { index: Number },
    /// `setAutofillFor` (an exported function that writes a store).
    #[serde(rename = "setAutofillFor")]
    SetAutofillFor {
        profile: String,
        kind: SetAutofillForKind,
        on: bool,
    },
    /// `setDownloadsFor` (an exported function that writes a store).
    #[serde(rename = "setDownloadsFor")]
    SetDownloadsFor {
        patch: SetDownloadsForPatch,
        profiles: Vec<String>,
    },
    /// `setMouseShortcut` (an exported function that writes a store).
    #[serde(rename = "setMouseShortcut")]
    SetMouseShortcut {
        id: String,
        button: Option<crate::commands::Shortcut>,
    },
    /// `setPasswordAutofillFor` (an exported function that writes a store).
    #[serde(rename = "setPasswordAutofillFor")]
    SetPasswordAutofillFor { profile: String, on: bool },
    /// `setPreloadPagesFor` (an exported function that writes a store).
    #[serde(rename = "setPreloadPagesFor")]
    SetPreloadPagesFor { profile: String, on: bool },
    /// `setSearchEngine` (an exported function that writes a store).
    #[serde(rename = "setSearchEngine")]
    SetSearchEngine { id: String },
    /// `setSettingsQuery` (an exported function that writes a store).
    #[serde(rename = "setSettingsQuery")]
    SetSettingsQuery { query: String },
    /// `setShortcutOverride` (an exported function that writes a store).
    #[serde(rename = "setShortcutOverride")]
    SetShortcutOverride {
        id: String,
        combo: Option<crate::commands::Shortcut>,
    },
    /// `setShortcutsQuery` (an exported function that writes a store).
    #[serde(rename = "setShortcutsQuery")]
    SetShortcutsQuery {
        #[serde(rename = "shortcutsQuery")]
        shortcuts_query: String,
    },
    /// `settingsBack` (an exported function that writes a store).
    #[serde(rename = "settingsBack")]
    SettingsBack,
    /// `settingsForward` (an exported function that writes a store).
    #[serde(rename = "settingsForward")]
    SettingsForward,
    /// `settingsSearchHost` (an exported function that writes a store).
    #[serde(rename = "settingsSearchHost")]
    SettingsSearchHost,
    /// `setUpdaterPrefs` (an exported function that writes a store).
    #[serde(rename = "setUpdaterPrefs")]
    SetUpdaterPrefs { patch: SetUpdaterPrefsPatch },
    /// `showSettingsSheet` (an exported function that writes a store).
    #[serde(rename = "showSettingsSheet")]
    ShowSettingsSheet { sheet: serde_json::Value },
    /// `startInterfaceScale` (an exported function that writes a store).
    #[serde(rename = "startInterfaceScale")]
    StartInterfaceScale,
    /// `startNativeSettings` (an exported function that writes a store).
    #[serde(rename = "startNativeSettings")]
    StartNativeSettings,
    /// `store` (an exported function that writes a store).
    #[serde(rename = "store")]
    Store {
        r: StoreR,
        args: serde_json::Value,
        value: serde_json::Value,
    },
    /// `switchTabLayout` (an exported function that writes a store).
    #[serde(rename = "switchTabLayout")]
    SwitchTabLayout {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `takePreferredImportSource` (an exported function that writes a store).
    #[serde(rename = "takePreferredImportSource")]
    TakePreferredImportSource,
    /// `toggleSidebarSide` (an exported function that writes a store).
    #[serde(rename = "toggleSidebarSide")]
    ToggleSidebarSide,
    /// `toggleTabLayout` (an exported function that writes a store).
    #[serde(rename = "toggleTabLayout")]
    ToggleTabLayout {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `updateCustomEngine` (an exported function that writes a store).
    #[serde(rename = "updateCustomEngine")]
    UpdateCustomEngine {
        id: String,
        input: serde_json::Value,
    },
    /// `updateSettings` (an exported function that writes a store).
    #[serde(rename = "updateSettings")]
    UpdateSettings {
        patch: OrderedPatch,
        #[serde(default)]
        options: Option<UpdateSettingsOptions>,
    },
}

/// Applies `action` to `app`.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    reducer::apply(app, action, fx);
}
