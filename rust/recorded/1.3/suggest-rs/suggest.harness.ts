// Recording harness for suggest.ts: a BangTable is an object with a `get` method, which JSON cannot
// carry, so `bangs` is a slim list here. The texts are typed in turn on one source, as keystrokes
// are, so later texts narrow from the pages the earlier ones matched.
import { bangTable } from "./bangs.ts";
import { buildSuggestions, type SuggestionResult, type SuggestionSource, type SuggestOptions } from "./suggest.ts";

export type HarnessOptions = Omit<SuggestOptions, "bangs"> & { bangs?: string | null };

/** `buildSuggestions` of each text in order, with the same source and options. */
export function suggestTyped(raws: string[], source: SuggestionSource, options: HarnessOptions): SuggestionResult[] {
  const bangs = options.bangs == null ? options.bangs : bangTable(options.bangs);
  const opts: SuggestOptions = { ...options, bangs };
  return raws.map((raw) => buildSuggestions(raw, source, opts));
}
