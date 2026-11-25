// Ria original, no Ardia source.

import { registerHooks } from "node:module";
import { loadArdia } from "../kit/load-ardia.mjs";
import { nativeFake, replayDriver } from "../kit/replay.mjs";

const FROZEN_NOW = 1700000000000;

// Canned native results. A function gets the call args. Promise calls return a deferred that `settle` resolves.
const SYNC = {
  "ArdiaApp.interfaceScale": () => 1,
  "ArdiaApp.interfaceScaleRange": () => [0.8, 1.5],
};
const ASYNC = {
  "ArdiaApp.setInterfaceScale": (scale) => scale,
  "ArdiaShell.setAppearance": () => undefined,
  "ArdiaSystem.setThinScrollBars": () => undefined,
  "ArdiaAppWindows.openWindow": () => undefined,
  "ArdiaAppWindows.focusWindow": () => undefined,
};

const ENGINE_ID = /^engine-([0-9a-z]+)-([0-9a-z]+)$/;

export async function settingsReplay(scratch) {
  const load = await loadArdia(scratch);

  // useScrollEnds in growingRows.ts is module-private: export it under a test name so the real store is read.
  registerHooks({
    load(url, context, next) {
      const loaded = next(url, context);
      if (!url.endsWith("/state/settings/growingRows.ts")) return loaded;
      const source = typeof loaded.source === "string" ? loaded.source : new TextDecoder().decode(loaded.source);
      return { ...loaded, source: `${source}\nexport { useScrollEnds as __useScrollEnds };\n` };
    },
  });

  // Every native module goes through a proxy. Outside a case it uses Ardia's own fake; inside a case it records.
  const [{ shellNativeModules }, { ardiaCoreNativeModules }] = await Promise.all([load("test-native/shell.ts"), load("test-native/ardiacore.ts")]);
  let platform = null;
  let pending = [];
  const deferred = (value) =>
    new Promise((resolve) => {
      pending.push(() => resolve(value));
    });
  const proxies = {};
  for (const [moduleName, original] of Object.entries({ ...shellNativeModules, ...ardiaCoreNativeModules })) {
    const domain = moduleName in ardiaCoreNativeModules && typeof original.call === "function" ? `${moduleName[5].toLowerCase()}${moduleName.slice(6)}` : null;
    proxies[moduleName] = new Proxy(original, {
      get(target, prop, receiver) {
        const real = Reflect.get(target, prop, receiver);
        if (!platform || typeof prop !== "string" || typeof real !== "function") return real;
        if (domain && prop === "call") {
          return (verb, profile, json) => {
            const params = { ...JSON.parse(json), ...(profile ? { profile } : {}) };
            const name = `${domain}.${verb}`;
            platform.method(name, null)(...(Object.keys(params).length ? [params] : []));
            return deferred("null");
          };
        }
        const name = `${moduleName}.${prop}`;
        if (SYNC[name]) return (...args) => platform.method(name, SYNC[name](...args))(...args);
        if (ASYNC[name]) {
          return (...args) => {
            const result = ASYNC[name](...args);
            platform.method(name, result)(...args);
            return deferred(result);
          };
        }
        if (prop === "addListener" || prop === "removeListener") return real.bind(target);
        return () => {
          throw new Error(`Unfaked native call in settings replay: ${name}`);
        };
      },
    });
  }
  globalThis.acTestNativeModules = proxies;

  const [actions, nav, search, scaleMod, scaleSync, nativeSettings, sheetMod, rows, { useBrowser }] = await Promise.all([
    load("state/settings/actions.ts"),
    load("state/settings/settingsWindows.ts"),
    load("state/settings/searchState.ts"),
    load("state/settings/interfaceScale.ts"),
    load("state/settings/interfaceScaleSync.ts"),
    load("state/settings/nativeSettings.ts"),
    load("state/settings/sheet.ts"),
    load("state/settings/growingRows.ts"),
    load("state/browser/store.ts"),
  ]);
  const stores = [nav.useSettingsNav, search.useSettingsSearch, scaleMod.useInterfaceScale, sheetMod.useSettingsSheet, nav.useImportRequest, rows.__useScrollEnds];
  const initial = stores.map((store) => store.getState());

  // Generated engine ids are `engine-<now36>-<counter>` with a module counter; cases see the counter from 1.
  let engineAdds = 0;
  let engineBase = 0;
  const engineIdIn = (id) => {
    const m = typeof id === "string" && ENGINE_ID.exec(id);
    return m ? `engine-${m[1]}-${(parseInt(m[2], 36) + engineBase).toString(36)}` : id;
  };
  const engineIdOut = (value) => {
    if (typeof value === "string") {
      const m = ENGINE_ID.exec(value);
      const n = m && parseInt(m[2], 36);
      return m && n > engineBase ? `engine-${m[1]}-${(n - engineBase).toString(36)}` : value;
    }
    if (Array.isArray(value)) return value.map(engineIdOut);
    if (value && typeof value === "object") return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, engineIdOut(v)]));
    return value;
  };

  const store = () => useBrowser.getState();

  function run(action) {
    switch (action.type) {
      case "updateSettings":
        return void actions.updateSettings(action.patch, action.options);
      case "setSearchEngine":
        return void actions.setSearchEngine(engineIdIn(action.id));
      case "addCustomEngine": {
        const result = actions.addCustomEngine(action.input);
        if ("id" in result) engineAdds++;
        return result;
      }
      case "updateCustomEngine":
        return actions.updateCustomEngine(engineIdIn(action.id), action.input);
      case "removeCustomEngine":
        return void actions.removeCustomEngine(engineIdIn(action.id));
      case "setShortcutOverride":
        return void actions.setShortcutOverride(action.id, action.combo);
      case "setMouseShortcut":
        return void actions.setMouseShortcut(action.id, action.button);
      case "chooseAddressBar":
        return void actions.chooseAddressBar(action.addressBar);
      case "chooseScrollToolbar":
        return void actions.chooseScrollToolbar(action.hideToolbarWhileScrolling);
      case "chooseCommandBarPlacement":
        return void actions.chooseCommandBarPlacement(action.commandBarPlacement);
      case "chooseSidebarSide":
        return void actions.chooseSidebarSide(action.side);
      case "toggleSidebarSide":
        return void actions.toggleSidebarSide();
      case "chooseInterfaceScale":
        return void actions.chooseInterfaceScale(action.scale);
      case "chooseTabLayout":
        return void actions.chooseTabLayout(action.layout);
      case "goToSettingsPane":
        return void nav.goToSettingsPane(action.pane, action.profileId, action.detail);
      case "replaceSettingsPlace":
        return void nav.replaceSettingsPlace(action.profileId, action.detail);
      case "settingsBack":
        return void nav.settingsBack();
      case "settingsForward":
        return void nav.settingsForward();
      case "openSettings":
        // The TS function reads its consent token itself; `consent` only matters to the Rust signature.
        return void nav.openSettings(action.pane, action.profileId, action.detail);
      case "openSiteSettings":
        return void nav.openSiteSettings(action.origin, action.profileId);
      case "openImport":
        return void nav.openImport(action.browserId);
      case "takePreferredImportSource":
        return nav.takePreferredImportSource();
      case "setSettingsQuery":
        return void search.setSettingsQuery(action.query);
      case "dropSettingsSearch":
        return void search.dropSettingsSearch(action.consent ?? null, action.why);
      case "settingsSearchHost":
        return void search.settingsSearchHost()();
      case "moveSettingsSelection":
        return void search.moveSettingsSelection(action.step, action.count);
      case "selectSettingsResult":
        return void search.selectSettingsResult(action.index);
      case "clearSettingsReveal":
        return void search.clearSettingsReveal();
      case "setShortcutsQuery":
        return void search.setShortcutsQuery(action.shortcutsQuery);
      case "focusSettingsSearch":
        return void search.focusSettingsSearch();
      case "flashSetting":
        return void search.flashSetting(action.id);
      case "jumpToSetting":
        return void search.jumpToSetting(action.target);
      case "previewSetting":
        return void search.previewSetting(action.target);
      case "revealSetting":
        return void search.revealSetting(action.target, action.via, action.rank, action.unshown);
      case "revealCommandKeys":
        return void search.revealCommandKeys(action.keys, action.via, action.rank);
      case "previewCommandKeys":
        return void search.previewCommandKeys(action.keys);
      case "noteSettingsScroll":
        return void rows.noteSettingsScroll(action.offset, action.viewport, action.content);
      case "showSettingsSheet":
        return void sheetMod.showSettingsSheet(action.sheet);
      case "closeSettingsSheet":
        return void sheetMod.closeSettingsSheet();
      case "appliedInterfaceScale":
        return void scaleMod.appliedInterfaceScale(action.scale);
      default:
        throw new Error(`Unknown settings action: ${action.type}`);
    }
  }

  const replay = replayDriver(() => {
    platform = null;
    engineBase = engineAdds;
    store().hydrate({});
    // Re-arm growingRows' private scroll flags: a new content size with the end far away arms it again.
    rows.noteSettingsScroll(-1e9, 0, -1);
    stores.forEach((s, i) => s.setState(initial[i], true));
    pending = [];
    const fake = nativeFake();
    platform = fake;
    let stopScale = null;
    let stopNative = null;
    return {
      platform: fake,
      state: () => ({
        ...nav.useSettingsNav.getState(),
        ...search.useSettingsSearch.getState(),
        scale: scaleMod.useInterfaceScale.getState().scale,
        sheet: sheetMod.useSettingsSheet.getState().sheet,
        source: nav.useImportRequest.getState().source,
        reached: rows.__useScrollEnds.getState().reached,
        settings: store().settings,
        settingsMeta: store().settingsMeta,
      }),
      async apply(action) {
        const now = Date.now;
        Date.now = () => action.now ?? FROZEN_NOW;
        try {
          switch (action.type) {
            case "settle":
              for (let round = 0; ; round++) {
                await new Promise((resolve) => setImmediate(resolve));
                if (!pending.length) return;
                if (round > 100) throw new Error("settle: native promises keep coming");
                const resolving = pending;
                pending = [];
                for (const resolve of resolving) resolve();
              }
            case "startInterfaceScale":
              stopScale ??= scaleSync.startInterfaceScale();
              return;
            case "startNativeSettings":
              stopNative ??= nativeSettings.startNativeSettings();
              stopScale ??= scaleSync.startInterfaceScale();
              return;
            default:
              return run(action);
          }
        } finally {
          Date.now = now;
        }
      },
      cleanup() {
        platform = null;
        stopNative?.();
        stopScale?.();
        pending = [];
      },
    };
  });

  return async (actions) => engineIdOut(await replay(actions));
}
