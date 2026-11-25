import * as k from "./pluginCommands.ts";
import type { Check } from "./pluginChecks.ts";

// Each check adds problems to its `Check`: the cases record the result and those problems. A
// command case also records which `is…Action` predicates hold for its action.
type Out = { result: unknown; problems: string[] };
const run = (f: (c: Check) => unknown): Out => {
  const c: Check = { problems: [] };
  const result = f(c);
  return { result, problems: c.problems };
};
const kinds = (a: k.CommandAction) => [k.isToolAction(a), k.isOpenAction(a), k.isViewAction(a), k.isPanelAction(a), k.isHandlerAction(a)];

export const splitSidesCase = (): unknown => k.SPLIT_SIDES;
export const whenOfCase = (path: string, value: unknown): Out => run((c) => k.whenOf(c, path, value));
export const commandCase = (path: string, value: unknown, hasMain: boolean): Out & { kinds: boolean[] | null } => {
  const out = run((c) => k.command(c, path, value, hasMain));
  const spec = out.result as k.CommandSpec | null;
  return { ...out, kinds: spec ? kinds(spec.action) : null };
};
export const toolCase = (path: string, value: unknown): Out => run((c) => k.tool(c, path, value));
