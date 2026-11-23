// Recording harness for suggestionRows.ts: `Output` is a class whose rows are read after pushes,
// so `outputItems` pushes each batch and returns the rows kept; the other wrappers call through.
import { actionRow, createRow, dedupeSuggestions, Output, pageSuggestion, suggestionKey, type PageRow } from "./suggestionRows.ts";
import type { CommandAction, Suggestion } from "./suggestion.ts";

/** `pageSuggestion`; an undefined `tabId` takes the row's own. */
export function pageSuggestionOf(c: PageRow, tabId?: string | null): Suggestion {
  return pageSuggestion(c, tabId);
}

export function suggestionKeyOf(s: Suggestion): string {
  return suggestionKey(s);
}

export function outputItems(limit: number, batches: Suggestion[][]): Suggestion[] {
  const out = new Output(limit);
  for (const rows of batches) out.push(...rows);
  return out.items;
}

export function createRowOf(c: { id: string; title: string; url: string; site: string }): Suggestion {
  return createRow(c);
}

export function actionRowOf(a: CommandAction): Suggestion {
  return actionRow(a);
}

export function dedupeSuggestionsOf(remote: string[], query: string): string[] {
  return dedupeSuggestions(remote, query);
}
