// Ria original, no Ardia source.

import { loadArdia } from "./load-ardia.mjs";
import { nativeFake, replayDriver } from "./replay.mjs";

export async function findReplay(scratch) {
  const load = await loadArdia(scratch);
  const [find, { useBrowser }, { webviews }, { takeFindCounts }] = await Promise.all([
    load("state/find/actions.ts"),
    load("state/browser/store.ts"),
    load("platform/webviews.ts"),
    load("state/telemetry/findCounts.ts"),
  ]);
  const store = useBrowser.getState();
  store.hydrate({});
  const tabs = Object.fromEntries(["a", "b"].map((name) => {
    const window = store.createWindow({ url: "https://example.test/" });
    return [name, useBrowser.getState().windows[window].tabIds[0]];
  }));

  return replayDriver(() => {
    const mounted = new Set();
    useBrowser.setState({ find: {} });
    takeFindCounts();
    const platform = nativeFake();
    return {
      platform,
      state: () => Object.fromEntries(Object.entries(tabs).map(([name, tab]) => [name, useBrowser.getState().find[tab]])),
      apply(action) {
        const tab = Object.hasOwn(tabs, action.tab) ? tabs[action.tab] : action.tab;
        switch (action.type) {
          case "mount":
            mounted.add(tab);
            webviews.set(tab, {
              find: platform.method("find", action.match),
              stopFinding: platform.method("stopFinding"),
              focus: platform.method("focus"),
            });
            break;
          case "open": {
            const now = Date.now;
            Date.now = () => action.now;
            try {
              find.openFind(tab, action.options);
            } finally {
              Date.now = now;
            }
            break;
          }
          case "close":
            find.closeFind(tab);
            break;
          case "setMatches":
            store.setFind(tab, { count: action.count, active: action.active });
            break;
          case "step":
            find.findStep(tab, action.backwards);
            break;
          case "takeFocus":
            find.takeFindFocusRequest(tab);
            break;
          case "submitRepeatsKey":
            return find.submitRepeatsKey(action.keyAt, action.now);
          case "takeCounts":
            return takeFindCounts();
          default:
            throw new Error(`Unknown find action: ${action.type}`);
        }
      },
      cleanup() {
        for (const tab of mounted) webviews.delete(tab);
      },
    };
  });
}
