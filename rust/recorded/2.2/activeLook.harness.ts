import { activeLook, iconName, setActiveLook, withIconNames } from "./activeLook.ts";
import { DEFAULT_LOOK, parseLookBlocks, resolveLook, type ThemeLook } from "./themeLook.ts";

// A look holds a Set and a Map, which the wire cannot carry: each case sets the look of a theme
// document `doc` (null or undefined: Ardia's own), makes the call, and sets Ardia's own back.
// `lookJson` gives the Set and the Map as arrays.
const lookOf = (doc: unknown): ThemeLook => (doc == null ? DEFAULT_LOOK : resolveLook(parseLookBlocks("themes/t.json", doc as Record<string, unknown>, [])));
const lookJson = (look: ThemeLook) => ({ ...look, hidden: [...look.hidden], icons: [...look.icons] });

function withLook<T>(doc: unknown, run: () => T): T {
  setActiveLook(lookOf(doc));
  try {
    return run();
  } finally {
    setActiveLook(DEFAULT_LOOK);
  }
}

export const activeLookCase = (doc: Record<string, unknown> | null): unknown => withLook(doc, () => lookJson(activeLook()));

export const iconNameCase = (doc: Record<string, unknown> | null, name: string): string => withLook(doc, () => iconName(name));

// `same`: Ardia gives the items themselves back when the look swaps no icon.
export function withIconNamesCase(doc: Record<string, unknown> | null, items: unknown[]): { items: unknown[]; same: boolean } {
  return withLook(doc, () => {
    const out = withIconNames(items as { symbol?: string }[]);
    return { items: out, same: out === items };
  });
}
