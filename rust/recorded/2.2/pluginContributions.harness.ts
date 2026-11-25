import * as k from "./pluginContributions.ts";
import type { Check } from "./pluginChecks.ts";

// A contributes case records what `contributesOf` read, its problems, the page panel and the
// view ids of the result.
export const constantsCase = (): unknown => ({ MENU_SLOTS: k.MENU_SLOTS, PANEL_MODES: k.PANEL_MODES, CONTRIBUTES_KEYS: k.CONTRIBUTES_KEYS, PAGE_PANEL_ID: k.PAGE_PANEL_ID, emptyMenus: k.emptyMenus() });
export const contributesCase = (value: unknown, hasMain: boolean): unknown => {
  const c: Check = { problems: [] };
  const result = k.contributesOf(c, value, hasMain);
  return { result, problems: c.problems, pagePanel: k.pagePanelOf(result.panels), viewIds: k.viewIdsOf(result) };
};
