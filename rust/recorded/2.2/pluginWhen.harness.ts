import * as w from "./pluginWhen.ts";

export const parseWhen = (source: string): unknown => w.parseWhen(source);
export const globMatches = (pattern: string, value: string): boolean => w.globMatches(pattern, value);
export const whenHolds = (source: string | undefined | null, context: w.WhenContext): boolean => w.whenHolds(source, context);

// `evaluateWhen` takes only a `WhenExpr` that `parseWhen` made: the case parses `source` first,
// and gives the parse problem when it does not parse.
export function evaluateWhenCase(source: string, context: w.WhenContext): boolean | string {
  const read = w.parseWhen(source);
  return read.ok ? w.evaluateWhen(read.expr, context) : read.problem;
}
