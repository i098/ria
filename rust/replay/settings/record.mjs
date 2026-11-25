// Ria original, no Ardia source.

import { once } from "node:events";
import { settingsReplay } from "./settings.mjs";

const [scratch] = process.argv.slice(2);
if (!scratch) throw new Error("Usage: node record.mjs ARDIA_SCRATCH");

const T = 1700000000000;
const engineId = (now, n) => `engine-${now.toString(36)}-${n.toString(36)}`;
const U = (patch, options) => ({ type: "updateSettings", patch, ...(options ? { options } : {}) });
const add = (input, now) => ({ type: "addCustomEngine", input, now });
const input = (name, keyword, url) => ({ name, keyword, url });

const NAMES = [
  ["Café Suche", "café", "https://café.example/?q=%s"],
  ["日本語検索", "日本", "https://例え.テスト/search?q=%s"],
  ["🧪 Lab", "🧪", "https://lab.example/?q=%s&e=🧪"],
  ["ÜBER Suche", "ÜBER", "https://über.example/s?q=%s"],
  ["Σίσυφος", "ΣΊΣΥΦΟΣ", "https://ελληνικά.example/?q=%s"],
  ["İstanbul", "İST", "https://istanbul.example/?q=%s"],
  ["Поиск", "ПОИСК", "https://поиск.рф/?q=%s"],
  ["بحث", "بحث", "https://بحث.example/?q=%s"],
  ["חיפוש", "חפש", "https://example.co.il/?q=%s"],
  ["  한국어  ", "  한국  ", "  https://한국.example/?q=%s  "],
  ["\uFEFFBOM name\uFEFF", "\uFEFFbom", "https://bom.example/?q=%s"],
  ["e\u0301 combining", "E\u0301", "http://combining.example/?q=%s"],
];

const PANES = ["general", "profiles", "tabs", "appearance", "automations", "plugins", "shortcuts", "advanced"];
const target = (id, pane, section) => ({ id, pane, section });

const cases = [];

// Patches: valid, wrong types, unknown keys, out of range, nested objects; each as chosen, inferred, and synced.
const PATCHES = [
  { warnBeforeQuitting: true },
  { warnBeforeQuitting: "yes" },
  { notASetting: 1 },
  { notASetting: 1, restoreSession: false },
  { restoreSession: { nested: true } },
  { searchEngine: null },
  { searchEngine: "bing", customSearchUrl: "https://ex.test/?q=%s" },
  { interfaceScale: 1.25 },
  { interfaceScale: 1.23 },
  { interfaceScale: 2 },
  { glassTintAmount: 0.5 },
  { glassTintAmount: 1.5 },
  { sidebarWidth: 0 },
  { sidebarWidth: 280.5 },
  { pageFrame: 0 },
  { pageFrame: 12 },
  { pageFrame: 13 },
  { pageFrame: 2.5 },
  { tabCornerRadius: null },
  { tabCornerRadius: 8 },
  { tabCornerRadius: 100 },
  { shortcuts: { "tabs.new": ["t", "cmd"] } },
  { shortcuts: { "tabs.new": "t" } },
  { shortcuts: { "tabs.ü": ["ü", "⌘"] } },
  { mouseShortcuts: { back: ["button4"] }, shortcuts: [] },
  { neverTranslateSites: { "https://例え.jp": ["ja", "en"] } },
  { neverTranslateSites: { "https://a.test": "ja" } },
  { developerModeSites: { "https://a.test": true } },
  { developerModeSites: { "https://a.test": "true" } },
  { littleArdiaSize: [1024, 768] },
  { littleArdiaSize: [0, 1] },
  { littleArdiaSize: [900] },
  { launchActions: Array.from({ length: 20 }, (_, i) => `action-${i}`) },
  { launchActions: Array.from({ length: 21 }, (_, i) => `action-${i}`) },
  { customSearchEngines: [{ id: "mine-1", name: "Mine", keyword: "mine", url: "https://mine.example/?q=%s" }, { id: "bad", name: "Bad", keyword: "bad", url: "nope" }] },
  { customSearchEngines: [{ id: "mine-1", name: "Mine" }] },
  { websiteEngines: { "a.test": "webkit", "b.test": "chromium" } },
  { websiteEngines: { "a.test": "gecko" } },
  { mutedSites: ["ünïcode.test", "日本.jp"] },
  { mutedSites: ["a.test", 1] },
  { newTabUrl: "https://новости.рф/", defaultProfileId: "профиль" },
  { cleanUpInactiveTabsAfterMinutes: null },
  { cleanUpInactiveTabsAfterMinutes: 30 },
  { cleanUpInactiveTabsAfterMinutes: 0 },
  { commandBarStart: {} },
  { commandBarStart: { other: true } },
  { omniboxProviders: [] },
  { readerFont: "comic" },
  { appearance: "dark", pressMotion: "bounce" },
  { autoHideTabs: true },
  { minimalismMode: true },
  { pageFrameZeroInFullScreen: true },
  { pageFrameZeroInFullScreen: true, pageFrameZeroWhenTabsHidden: false },
  { peekModifier: "alt", splitLinkModifier: "alt" },
  { splitLinkModifier: "alt" },
  { tabLayout: "top", sidebarSide: "right", hideToolbarWhileScrolling: true },
];
for (const patch of PATCHES) {
  for (const options of [undefined, { inferred: true }, { fromSync: true }]) {
    cases.push([U(patch, options), U(patch, { inferred: true }), U(patch)]);
  }
}

// settingsMeta: chosen and inferred paths.
cases.push(
  [U({ restoreSession: false }, { inferred: true }), U({ tidyTabs: true }, { inferred: true }), U({ restoreSession: false }), U({ tidyTabs: true }, { inferred: false, fromSync: false })],
  [U({ restoreSession: false }, { inferred: true, fromSync: true }), U({ restoreSession: true }, { inferred: true }), U({ restoreSession: true }, { fromSync: true }), U({ restoreSession: true })],
  [U({ tidyTabs: "on", unknown: 1 }, { inferred: true }), U({ tidyTabs: true, unknown: 1 }, { inferred: true }), U({})],
  [U({}, { inferred: true }), U({}), U({ tidyTabs: false }), U({ tidyTabs: false }, { inferred: true }), U({ tidyTabs: false })],
  [U({ appearance: "light", compactToolbar: true }), U({ appearance: "dark" }, { inferred: true }), U({ compactToolbar: false }, { fromSync: true }), U({ compactToolbar: false })],
  [U({ hideToolbarWhileScrolling: true }, { inferred: true }), U({ hideToolbarWhileScrolling: false }), U({ hideToolbarWhileScrolling: true }, { inferred: true })],
);

// Link keys: peekModifier and splitLinkModifier stay apart.
cases.push(
  [U({ peekModifier: "alt" }), U({ splitLinkModifier: "alt" })],
  [U({ splitLinkModifier: "alt" }), U({ peekModifier: "alt" })],
  [U({ splitLinkModifier: "alt" }), U({ peekModifier: "off" }), U({ peekModifier: "alt" }), U({ splitLinkModifier: "alt" })],
  [U({ peekModifier: "alt" }, { fromSync: true }), U({ splitLinkModifier: "alt" }, { fromSync: true }), U({ peekModifier: "shift" })],
  [U({ peekModifier: "alt" }, { inferred: true }), U({ splitLinkModifier: "alt" }, { inferred: true })],
  [U({ peekModifier: "alt", splitLinkModifier: "alt" }, { inferred: true }), U({ peekModifier: "shift", splitLinkModifier: "alt" })],
  [U({ splitLinkModifier: "alt", peekModifier: "bogus" }), U({ peekModifier: "alt", splitLinkModifier: "bogus" })],
);

// minimalismMode, pageFrameZero rules, autoHideTabs.
cases.push(
  [U({ minimalismMode: true }), U({ newTabOpens: "page" }), U({ minimalismMode: false }), U({ minimalismMode: false })],
  [U({ minimalismMode: true }, { fromSync: true }), U({ minimalismMode: "yes" }), U({ minimalismMode: false }, { inferred: true })],
  [U({ pageFrameZeroInFullScreen: true }, { fromSync: true }), U({ pageFrameZeroWhenTabsHidden: false }), U({ pageFrameZeroInFullScreen: false }), U({ pageFrameZeroInFullScreen: true })],
  [U({ pageFrameZeroWhenTabsHidden: true }), U({ pageFrameZeroInFullScreen: true }), U({ pageFrameZeroWhenTabsHidden: false, pageFrameZeroInFullScreen: true })],
  [U({ pageFrameZeroInFullScreen: true, pageFrameZeroWhenTabsHidden: "no" }), U({ pageFrame: 0, borderlessMode: true })],
  [U({ autoHideTabs: true }), U({ autoHideTabs: true }), U({ autoHideTabs: false }, { fromSync: true }), U({ autoHideTabs: "true" })],
  [U({ autoHideTabs: true }, { inferred: true }), U({ autoHideTabs: false }, { inferred: true }), U({ autoHideTabs: false })],
);

// Custom search engines: add, update, remove, duplicate, invalid.
NAMES.forEach(([name, keyword, url], i) => {
  const now = T + 1000 * (i + 1);
  const first = engineId(now, 1);
  const second = engineId(now + 7, 2);
  cases.push([
    add(input(name, keyword, url), now),
    { type: "setSearchEngine", id: first },
    add(input(`${name} 2`, keyword.toUpperCase(), url), now + 5),
    add(input(`${name} 2`, `${keyword.trim()}2`, url), now + 7),
    { type: "updateCustomEngine", id: second, input: input("Zweite", keyword, url) },
    { type: "updateCustomEngine", id: second, input: input(` ${name} ✓ `, `${keyword.trim()}✓`, url) },
    { type: "updateCustomEngine", id: first, input: input(name, keyword, url) },
    { type: "removeCustomEngine", id: first },
    { type: "updateCustomEngine", id: first, input: input(name, keyword, url) },
  ]);
  cases.push([
    add(input("", keyword, url), now),
    add(input(name, "", url), now),
    add(input(name, `${keyword.trim()} x`, url), now),
    add(input(name, `${keyword.trim()}\u00a0x`, url), now),
    add(input(name, keyword, url.replace("https://", "ftp://").replace("http://", "ftp://")), now),
    add(input(name, keyword, url.replaceAll("%s", "")), now),
    add(input(name, "GOOGLE.COM", url), now),
    add(input(name, keyword, url), now + 1),
    { type: "setSearchEngine", id: "no-such-engine" },
    { type: "setSearchEngine", id: "bing" },
    { type: "removeCustomEngine", id: engineId(now + 1, 1) },
    { type: "removeCustomEngine", id: "no-such-engine" },
  ]);
});
cases.push(
  [U({ customSearchUrl: "https://legacy.example/?q=%s" }), { type: "setSearchEngine", id: "custom" }, add(input("Custom 2", "custom", "https://c.example/?q=%s"), T), U({ customSearchUrl: "" }), { type: "setSearchEngine", id: "custom" }],
  [U({ customSearchEngines: [{ id: "mine-1", name: "Mine", keyword: "mine", url: "https://mine.example/?q=%s" }] }), { type: "setSearchEngine", id: "mine-1" }, add(input("Dup", "MINE", "https://d.example/?q=%s"), T), { type: "updateCustomEngine", id: "mine-1", input: input("Mine ✓", "mine", "https://mine.example/?q=%s") }, { type: "removeCustomEngine", id: "mine-1" }],
  [add(input("A", "a", "https://a.example/?q=%s"), T), add(input("B", "b", "https://b.example/?q=%s"), T), { type: "updateCustomEngine", id: engineId(T, 2), input: input("B", "A", "https://b.example/?q=%s") }, { type: "setSearchEngine", id: engineId(T, 2) }, { type: "removeCustomEngine", id: engineId(T, 1) }, { type: "removeCustomEngine", id: engineId(T, 2) }],
);

// Shortcuts and mouse shortcuts.
const combo = (key, ...modifiers) => ({ key, modifiers });
cases.push(
  [{ type: "setShortcutOverride", id: "tabs.new", combo: combo("t", "cmd") }, { type: "setShortcutOverride", id: "tabs.new", combo: combo("n", "cmd", "shift") }, { type: "setShortcutOverride", id: "tabs.new", combo: null }],
  [{ type: "setShortcutOverride", id: "tabs.ü", combo: combo("ü", "cmd", "alt") }, { type: "setShortcutOverride", id: "見る", combo: combo("見") }, { type: "setShortcutOverride", id: "missing", combo: null }],
  [{ type: "setMouseShortcut", id: "back", button: combo("button4") }, { type: "setMouseShortcut", id: "forward", button: combo("button5", "shift") }, { type: "setMouseShortcut", id: "back", button: null }],
  [U({ shortcuts: { "tabs.close": ["w", "cmd"] } }, { inferred: true }), { type: "setShortcutOverride", id: "tabs.close", combo: combo("W", "cmd", "shift") }, { type: "setMouseShortcut", id: "🖱", button: combo("🖱", "ctrl") }],
);

// Choosers.
cases.push(
  [{ type: "chooseAddressBar", addressBar: "sidebar" }, { type: "chooseAddressBar", addressBar: "bottom" }, { type: "chooseAddressBar", addressBar: "toolbar" }],
  [{ type: "chooseScrollToolbar", hideToolbarWhileScrolling: true }, { type: "chooseScrollToolbar", hideToolbarWhileScrolling: false }],
  [{ type: "chooseCommandBarPlacement", commandBarPlacement: "centred" }, { type: "chooseCommandBarPlacement", commandBarPlacement: "centredOnClick" }, { type: "chooseCommandBarPlacement", commandBarPlacement: "nowhere" }, { type: "chooseCommandBarPlacement", commandBarPlacement: "field" }],
  [{ type: "chooseSidebarSide", side: "right" }, { type: "toggleSidebarSide" }, { type: "toggleSidebarSide" }, { type: "chooseSidebarSide", side: "middle" }, { type: "toggleSidebarSide" }],
  [{ type: "chooseTabLayout", layout: "top" }, { type: "chooseTabLayout", layout: "top" }, { type: "chooseTabLayout", layout: "sidebar" }],
  [0, 0.5, 0.8, 0.82, 0.825, 1, 1.024, 1.12, 1.125, 1.475, 1.5, 2, -1].map((scale) => ({ type: "chooseInterfaceScale", scale })),
);

// Navigation history, including more than 30 back entries.
const many = Array.from({ length: 36 }, (_, i) => ({ type: "goToSettingsPane", pane: PANES[i % PANES.length], ...(i % 3 ? {} : { profileId: `p${i}`, detail: i % 2 ? "sites" : null }) }));
cases.push(
  [...many, { type: "settingsBack" }, { type: "settingsBack" }, { type: "settingsBack" }, { type: "settingsForward" }, { type: "goToSettingsPane", pane: "advanced" }, { type: "settingsForward" }],
  [{ type: "settingsBack" }, { type: "settingsForward" }, { type: "goToSettingsPane", pane: "general" }, { type: "goToSettingsPane", pane: "general", profileId: null, detail: null }],
  [{ type: "goToSettingsPane", pane: "profiles", profileId: "default" }, { type: "goToSettingsPane", pane: "profiles", profileId: "default", detail: "site:https://bücher.de" }, { type: "replaceSettingsPlace", profileId: "default" }, { type: "settingsBack" }, { type: "settingsForward" }],
  [{ type: "goToSettingsPane", pane: "profiles", profileId: "a" }, { type: "replaceSettingsPlace", profileId: "b", detail: "passwords" }, { type: "replaceSettingsPlace", profileId: "b" }, { type: "settingsBack" }],
  [{ type: "goToSettingsPane", pane: "tabs" }, { type: "goToSettingsPane", pane: "appearance" }, { type: "settingsBack" }, { type: "settingsBack" }, { type: "settingsBack" }, { type: "settingsForward" }, { type: "settingsForward" }, { type: "settingsForward" }],
  [{ type: "openSettings" }, { type: "settle" }, { type: "openSettings", pane: "shortcuts" }, { type: "openSettings", pane: "profiles", profileId: "default", detail: "extension:äbc", consent: null }],
  [{ type: "setSettingsQuery", query: "tab" }, { type: "openSettings", pane: "appearance" }, { type: "setSettingsQuery", query: "größe" }, { type: "openSettings" }, { type: "settle" }],
  [{ type: "openSiteSettings", origin: "https://bücher.de", profileId: "default" }, { type: "openSiteSettings", origin: "https://例え.jp", profileId: "профиль" }, { type: "settingsBack" }, { type: "settle" }],
  [{ type: "openImport", browserId: "chrome" }, { type: "takePreferredImportSource" }, { type: "takePreferredImportSource" }, { type: "openImport" }, { type: "takePreferredImportSource" }],
  [{ type: "openImport", browserId: "safari" }, { type: "openImport", browserId: "brave ✓" }, { type: "settle" }, { type: "takePreferredImportSource" }],
);

// Settings search: query, selection, reveal, preview, flash, command keys.
const QUERIES = ["tab", "  ", "größe", "日本語", "🧪", "ΣΊΣΥΦΟΣ", "e\u0301"];
for (const query of QUERIES) {
  cases.push([
    { type: "setSettingsQuery", query },
    { type: "moveSettingsSelection", step: 1, count: 3 },
    { type: "moveSettingsSelection", step: 5, count: 3 },
    { type: "moveSettingsSelection", step: -10, count: 3 },
    { type: "selectSettingsResult", index: 2 },
    { type: "setSettingsQuery", query: `${query}x` },
    { type: "previewSetting", target: target(`id.${query}`, "appearance", `Abschnitt ${query}`), now: T + 10 },
    { type: "previewSetting", target: target(`id.${query}`, "appearance", `Abschnitt ${query}`), now: T + 11 },
    { type: "previewSetting", target: target("tabs.layout", "tabs", "Layout"), now: T + 12 },
    { type: "jumpToSetting", target: target("tabs.layout", "tabs", "Layout"), now: T + 13 },
    { type: "dropSettingsSearch" },
  ]);
}
cases.push(
  [{ type: "moveSettingsSelection", step: 1, count: 0 }, { type: "moveSettingsSelection", step: -1, count: 0 }, { type: "selectSettingsResult", index: -3 }, { type: "setSettingsQuery", query: "" }],
  [{ type: "setSettingsQuery", query: "a" }, { type: "dropSettingsSearch", consent: null, why: "closed" }, { type: "setSettingsQuery", query: "b" }, { type: "dropSettingsSearch", why: "dropped" }, { type: "dropSettingsSearch" }],
  [{ type: "setSettingsQuery", query: "café" }, { type: "previewSetting", target: target("x", "general", "Allgemein"), now: T + 1 }, { type: "settingsSearchHost" }, { type: "settingsSearchHost" }],
  [{ type: "previewSetting", target: target("x", "general", "General"), now: T + 1 }, { type: "setSettingsQuery", query: "  " }, { type: "previewSetting", target: target("x", "general", "General"), now: T + 2 }],
  [{ type: "focusSettingsSearch", now: T + 50 }, { type: "focusSettingsSearch", now: T + 60 }, { type: "flashSetting", id: "appearance.theme", now: T + 70 }, { type: "jumpToSetting", target: target("tabs.x", "tabs", "Tabs"), now: T + 80 }, { type: "flashSetting", id: "tabs.✓", now: T + 90 }, { type: "clearSettingsReveal" }],
  [{ type: "jumpToSetting", target: target("a", "tabs", "One"), now: T }, { type: "jumpToSetting", target: target("b", "tabs", "One"), now: T + 1 }, { type: "jumpToSetting", target: target("c", "appearance", "Two"), now: T + 2 }, { type: "settingsBack" }],
  [{ type: "goToSettingsPane", pane: "profiles", profileId: "default", detail: "sites" }, { type: "jumpToSetting", target: target("p", "profiles", "Sites"), now: T }, { type: "goToSettingsPane", pane: "profiles", profileId: "ghost" }, { type: "jumpToSetting", target: target("p", "profiles", "Sites"), now: T + 1 }],
  [{ type: "goToSettingsPane", pane: "profiles", profileId: "default", detail: "sites" }, { type: "setSettingsQuery", query: "sites" }, { type: "previewSetting", target: target("p", "profiles", "Sites"), now: T }, { type: "previewSetting", target: target("q", "general", "Q"), now: T + 1 }, { type: "previewSetting", target: target("p", "profiles", "Sites"), now: T + 2 }, { type: "jumpToSetting", target: target("p", "profiles", "Sites"), now: T + 3 }],
  [{ type: "revealSetting", target: target("r", "advanced", "Ä"), via: "keyboard", rank: 0, now: T }, { type: "revealSetting", target: target("r", "advanced", "Ä"), via: "click", rank: 3, unshown: true, now: T + 1 }],
  [{ type: "setSettingsQuery", query: "keys" }, { type: "previewSetting", target: target("s", "shortcuts", "S"), now: T }, { type: "revealSetting", target: target("s", "shortcuts", "S"), via: "click", rank: 1, unshown: false, now: T + 1 }],
  [{ type: "revealCommandKeys", keys: "⌘K", via: "keyboard", rank: 2, now: T }, { type: "setShortcutsQuery", shortcutsQuery: "" }, { type: "revealCommandKeys", keys: "⌥⇧N", via: "click", rank: 0, now: T + 1 }],
  [{ type: "previewCommandKeys", keys: "⌘T", now: T }, { type: "setSettingsQuery", query: "⌘" }, { type: "previewCommandKeys", keys: "⌘T", now: T + 1 }, { type: "previewCommandKeys", keys: "⌘W", now: T + 2 }, { type: "jumpToSetting", target: target("shortcuts.menuCommands", "shortcuts", "Keyboard"), now: T + 3 }],
  [{ type: "setShortcutsQuery", shortcutsQuery: "新しいタブ" }, { type: "setShortcutsQuery", shortcutsQuery: "  " }, { type: "dropSettingsSearch" }],
);

// Scroll ends.
const scroll = (offset, viewport, content) => ({ type: "noteSettingsScroll", offset, viewport, content });
cases.push(
  [scroll(0, 500, 3000), scroll(1900, 500, 3000), scroll(2500, 500, 3000), scroll(0, 500, 3000), scroll(2000, 500, 3000)],
  [scroll(0, 500, 500), scroll(0, 500, 500), scroll(0, 500, 4000), scroll(3500, 500, 4000), scroll(3500, 500, 4000)],
  [scroll(1899, 500, 3000), scroll(1900, 500, 3000), scroll(1901, 500, 3000), scroll(1901, 500, 3001)],
  [scroll(0, 0, 0), scroll(0, 0, 0), scroll(-700, 0, 0), scroll(0, 0, 0)],
  [scroll(100.5, 300.25, 1000.75), scroll(0, 0, 10000), scroll(9400, 0, 10000)],
);

// Interface scale: native push, settle, applied scale, native settings mirror.
cases.push(
  [U({ interfaceScale: 1.25 }), { type: "startInterfaceScale" }, { type: "settle" }, { type: "chooseInterfaceScale", scale: 1.1 }, { type: "settle" }, { type: "appliedInterfaceScale", scale: 0.9 }],
  [{ type: "startInterfaceScale" }, { type: "settle" }, { type: "chooseInterfaceScale", scale: 1 }, { type: "chooseInterfaceScale", scale: 1.3 }, { type: "chooseInterfaceScale", scale: 1.4 }, { type: "settle" }],
  [{ type: "startInterfaceScale" }, { type: "chooseInterfaceScale", scale: 1.2 }, { type: "chooseInterfaceScale", scale: 1.2 }, { type: "settle" }, { type: "appliedInterfaceScale", scale: 1.2 }],
  [{ type: "appliedInterfaceScale", scale: 1 }, { type: "appliedInterfaceScale", scale: 1.35 }, { type: "appliedInterfaceScale", scale: 1.35 }],
  [U({ interfaceScale: 0.9 }, { fromSync: true }), { type: "startInterfaceScale" }, { type: "startInterfaceScale" }, { type: "settle" }, U({ interfaceScale: 1 }, { fromSync: true }), { type: "settle" }],
  [{ type: "startInterfaceScale" }, U({ interfaceScale: 1.15 }, { inferred: true }), U({ glassTintAmount: 0.3 }), { type: "settle" }],
  [{ type: "startNativeSettings" }, { type: "settle" }, U({ appearance: "dark" }), U({ appearance: "dark" }), U({ videoUpscaling: false, popupsOpenAsTabs: true }), { type: "settle" }],
  [U({ appearance: "light", autoPictureInPicture: false, webKitFraudWarnings: true }), { type: "startNativeSettings" }, U({ blockAutoplayEverywhere: true, commandScrollZoom: false }, { fromSync: true }), U({ pressMotion: "bounce" }), { type: "settle" }],
  [U({ interfaceScale: 1.5 }), { type: "startNativeSettings" }, { type: "startNativeSettings" }, { type: "settle" }, { type: "chooseInterfaceScale", scale: 0.8 }, U({ tidyTabs: true }), { type: "settle" }],
  [{ type: "startNativeSettings" }, { type: "startInterfaceScale" }, U({ appearance: "bogus" }), { type: "settle" }, { type: "settle" }],
);

// Settings sheet.
cases.push(
  [{ type: "showSettingsSheet", sheet: "confirm-reset" }, { type: "closeSettingsSheet" }, { type: "closeSettingsSheet" }],
  [{ type: "showSettingsSheet", sheet: "Löschen bestätigen ✓" }, { type: "showSettingsSheet", sheet: "日本語シート" }, { type: "showSettingsSheet", sheet: null }],
  [{ type: "showSettingsSheet", sheet: "" }, { type: "openSettings", pane: "general" }, { type: "closeSettingsSheet" }],
);

const TYPES = [
  "updateSettings", "setSearchEngine", "addCustomEngine", "updateCustomEngine", "removeCustomEngine", "setShortcutOverride", "setMouseShortcut",
  "chooseAddressBar", "chooseScrollToolbar", "chooseCommandBarPlacement", "chooseSidebarSide", "toggleSidebarSide", "chooseInterfaceScale", "chooseTabLayout",
  "goToSettingsPane", "replaceSettingsPlace", "settingsBack", "settingsForward", "openSettings", "openSiteSettings", "openImport", "takePreferredImportSource",
  "setSettingsQuery", "dropSettingsSearch", "settingsSearchHost", "moveSettingsSelection", "selectSettingsResult", "clearSettingsReveal", "setShortcutsQuery",
  "focusSettingsSearch", "flashSetting", "jumpToSetting", "previewSetting", "revealSetting", "revealCommandKeys", "previewCommandKeys",
  "noteSettingsScroll", "showSettingsSheet", "closeSettingsSheet", "appliedInterfaceScale", "startInterfaceScale", "startNativeSettings", "settle",
];
const seen = new Set(cases.flat().map((a) => a.type));
const missing = TYPES.filter((t) => !seen.has(t));
if (missing.length || seen.size !== TYPES.length) throw new Error(`Action types: missing ${missing}, extra ${[...seen].filter((t) => !TYPES.includes(t))}`);
const nonAscii = cases.filter((c) => /[^\x00-\x7f]/.test(JSON.stringify(c))).length;
if (cases.length < 200 || nonAscii < 40) throw new Error(`Only ${cases.length} cases, ${nonAscii} with non-ASCII strings`);

const replay = await settingsReplay(scratch);
for (const actions of cases) {
  const line = JSON.stringify({ fn: "replay", args: [actions], ok: await replay(actions) });
  if (!process.stdout.write(`${line}\n`)) await once(process.stdout, "drain");
}
