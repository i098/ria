import * as k from "./pluginRows.ts";
import type { Check } from "./pluginChecks.ts";

// Each check adds problems to its `Check`: the cases record the result and those problems. A
// settings case also records the keys of `storedRows` and `secretRows`; a row value case reads
// `value` with the row that `row` gives for `raw`.
type Out = { result: unknown; problems: string[] };
const run = (f: (c: Check) => unknown): Out => {
  const c: Check = { problems: [] };
  const result = f(c);
  return { result, problems: c.problems };
};

export const rowTypesCase = (): unknown => k.ROW_TYPES;
export const rowCase = (path: string, value: unknown): Out => run((c) => k.row(c, path, value));
export const settingsOfCase = (path: string, value: unknown): Out & { stored: string[]; secret: string[] } => {
  const out = run((c) => k.settingsOf(c, path, value));
  const s = out.result as k.SettingsSpec;
  return { ...out, stored: k.storedRows(s).map((r) => r.key), secret: k.secretRows(s).map((r) => r.key) };
};
export const rowValueCase = (raw: unknown, value: unknown): unknown => {
  const r = k.row({ problems: [] }, "row", raw);
  return r ? k.rowValue(r, value) : null;
};
