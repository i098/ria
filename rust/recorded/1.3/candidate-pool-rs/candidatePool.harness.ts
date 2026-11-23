// Recording harness for candidatePool.ts: a pool looks pages up through functions and the tab
// view holds a Map and a Set, so these wrappers return the lists and the lookups of given keys.
// Calls within one wrapper share the module's caches, as the app's calls do.
import { NO_BOOKMARKS, poolFor, prepareSuggestions, savedBookmark, savedVisit, tabsOver, type Pool } from "./candidatePool.ts";
import type { SuggestionSource } from "./suggestion.ts";

type History = SuggestionSource["history"];
type Bookmarks = NonNullable<SuggestionSource["bookmarks"]>;

const view = (pool: Pool, keys: readonly string[]) => ({ list: pool.list, byKey: keys.map((k) => pool.byKey.get(k) ?? null), has: keys.map((k) => pool.byKey.has(k)) });
const byText = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

/** `poolFor` of each history list with each bookmark list (null: none), in that order. */
export function poolGrid(histories: History[], bookmarkLists: (Bookmarks | null)[], keys: string[]) {
  return histories.flatMap((h) => bookmarkLists.map((b) => view(poolFor(h, b ?? NO_BOOKMARKS), keys)));
}

/** `prepareSuggestions` until it says the pool is ready (deadline 0 stops every 64 rows), then the pool. */
export function preparedPool(source: SuggestionSource, deadline: number, keys: string[]) {
  while (!prepareSuggestions(source, deadline));
  prepareSuggestions(source, deadline);
  return view(poolFor(source.history, source.bookmarks ?? NO_BOOKMARKS), keys);
}

/** `tabsOver` twice (the second answer is cached); page keys sorted by UTF-16 units. */
export function tabsOverView(source: SuggestionSource, currentTabId: string | null) {
  const pool = poolFor(source.history, source.bookmarks ?? NO_BOOKMARKS);
  tabsOver(pool, source.tabs, currentTabId ?? undefined);
  const t = tabsOver(pool, source.tabs, currentTabId ?? undefined);
  return { tabIds: [...t.tabIds].sort(([a], [b]) => byText(a, b)), extra: t.extra, renamed: [...t.renamed].sort(byText) };
}

export function savedVisits(history: History, urls: string[]) {
  return urls.map((u) => savedVisit(history, u) ?? null);
}

export function savedBookmarks(bookmarks: Bookmarks, urls: string[]) {
  return urls.map((u) => savedBookmark(bookmarks, u) ?? null);
}
