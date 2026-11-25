import * as k from "./pluginPageBar.ts";
import type { Check } from "./pluginChecks.ts";

// Reads the switches, then the bar with them and `rows`, into one `Check`, and words the consent
// for what it read.
export const constantsCase = (): unknown => ({ HOST_ACTIONS: k.HOST_ACTIONS, HOST_ACTION_NAMES: k.HOST_ACTION_NAMES, BUTTON_DIVIDER: k.BUTTON_DIVIDER, PAGE_BAR_LOOKS: k.PAGE_BAR_LOOKS, PAGE_BAR_LIMITS: k.PAGE_BAR_LIMITS });
export const isHostActionCase = (value: unknown): boolean => k.isHostAction(value);
// `rows` that are not a list of `{ key, type }` text (the recorder makes those) throw.
type Row = { key: string; type: string };
const isRow = (r: unknown): r is Row => typeof r === "object" && r !== null && typeof (r as Row).key === "string" && typeof (r as Row).type === "string";
export const pageBarCase = (switches: unknown, bar: unknown, rows: unknown): unknown => {
  if (!Array.isArray(rows) || !rows.every(isRow)) throw new TypeError("rows must be a list of { key, type } text");
  const c: Check = { problems: [] };
  const read = k.siteSwitchesOf(c, switches);
  const spec = k.pageBarOf(c, bar, read, rows);
  return { switches: read, bar: spec, problems: c.problems, consent: k.pageBarConsent(read, spec) };
};
export const switchOriginCase = (url: string): string | null => k.switchOrigin(url);
export const isLocalOriginCase = (origin: string): boolean => k.isLocalOrigin(origin);
