import { tabCornersControl, validTabCorners, withTabCorners } from "./tabCorners.ts";
import { DEFAULT_LOOK, parseLookBlocks, resolveLook, type ThemeLook } from "./themeLook.ts";

// A look holds a Set and a Map, which the wire cannot carry: `withTabCornersCase` takes a theme
// document `doc` (null or undefined: Ardia's own look) and gives the Set and the Map of the result as
// arrays, and `same` when Ardia gives the look itself back.
const lookOf = (doc: unknown): ThemeLook => (doc == null ? DEFAULT_LOOK : resolveLook(parseLookBlocks("themes/t.json", doc as Record<string, unknown>, [])));
const lookJson = (look: ThemeLook) => ({ ...look, hidden: [...look.hidden], icons: [...look.icons] });

export const validTabCornersCase = (value: unknown): boolean => validTabCorners(value);

export function withTabCornersCase(doc: Record<string, unknown> | null, corners: number | null): { look: unknown; same: boolean } {
  const look = lookOf(doc);
  const out = withTabCorners(look, corners);
  return { look: lookJson(out), same: out === look };
}

export const tabCornersControlCase = (inForce: number, chosen: number | null) => tabCornersControl(inForce, chosen);
