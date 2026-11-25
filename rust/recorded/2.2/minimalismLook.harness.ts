import { minimalismLookOf } from "./minimalismLook.ts";
import { DEFAULT_LOOK, parseLookBlocks, resolveLook, type ThemeLook } from "./themeLook.ts";

// A look holds a Set and a Map, which the wire cannot carry: the case takes a theme document `doc`
// (null or undefined: Ardia's own look) and gives the Set and the Map of the result as arrays.
const lookOf = (doc: unknown): ThemeLook => (doc == null ? DEFAULT_LOOK : resolveLook(parseLookBlocks("themes/t.json", doc as Record<string, unknown>, [])));
const lookJson = (look: ThemeLook) => ({ ...look, hidden: [...look.hidden], icons: [...look.icons] });

export const minimalismLookCase = (doc: Record<string, unknown> | null): unknown => lookJson(minimalismLookOf(lookOf(doc)));
