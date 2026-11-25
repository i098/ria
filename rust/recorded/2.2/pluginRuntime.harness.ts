import * as r from "./pluginRuntime.ts";

// Each check adds problems to its `Check`: the cases record the result and those problems.
const run = (f: (c: { problems: string[] }) => unknown) => {
  const c = { problems: [] as string[] };
  const result = f(c);
  return { result, problems: c.problems };
};

export const sidecarOfCase = (path: string, value: unknown) => run((c) => r.sidecarOf(c, path, value));
export const mainOfCase = (entry: unknown, memory: unknown) => run((c) => r.mainOf(c, entry, memory));
export const programCase = (path: string, value: unknown) => run((c) => r.program(c, path, value));
