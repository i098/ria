import * as k from "./pluginChecks.ts";

// Each check adds problems to its `Check`: the cases record the result and those problems.
// `matching` takes a pattern by its export name (NAME, KEY, VERSION, SYMBOL, KEYWORD, UNSAFE_TEXT).
type Out = { result: unknown; problems: string[] };
const run = (f: (c: k.Check) => unknown): Out => {
  const c: k.Check = { problems: [] };
  const result = f(c);
  return { result, problems: c.problems };
};
// `text` and `list` take a count limit by its `LIMITS` name: real callers pass only those.
const limitOf = (name: string): number => {
  const value: unknown = Object.hasOwn(k.LIMITS, name) ? (k.LIMITS as Record<string, unknown>)[name] : undefined;
  return typeof value === "number" ? value : k.LIMITS.name;
};
const PATTERNS: Record<string, RegExp> = { NAME: k.NAME, KEY: k.KEY, VERSION: k.VERSION, SYMBOL: k.SYMBOL, KEYWORD: k.KEYWORD, UNSAFE_TEXT: k.UNSAFE_TEXT };

export const constantsCase = (): unknown => ({ LIMITS: k.LIMITS, DEFAULT_SIDECAR_MB: k.DEFAULT_SIDECAR_MB, DEFAULT_MAIN_MB: k.DEFAULT_MAIN_MB, NAME_RULE: k.NAME_RULE, KEY_RULE: k.KEY_RULE, SYMBOL_RULE: k.SYMBOL_RULE, NAME_PATTERN: k.NAME_PATTERN, KEY_PATTERN: k.KEY_PATTERN, VERSION_PATTERN: k.VERSION_PATTERN, SYMBOL_PATTERN: k.SYMBOL_PATTERN, KEYWORD_PATTERN: k.KEYWORD_PATTERN });
export const patternCase = (name: string, value: string): boolean => (PATTERNS[name] ?? k.NAME).test(value);
export const atCase = (path: string, key: string | number): string => k.at(path, key);
export const onlyCase = (path: string, doc: Record<string, unknown>, keys: string[]): Out => run((c) => k.only(c, path, doc, keys));
export const textCase = (path: string, value: unknown, limit: string, required: boolean): Out => run((c) => k.text(c, path, value, limitOf(limit), required));
export const matchingCase = (path: string, value: unknown, pattern: string, rule: string, required: boolean): Out => run((c) => k.matching(c, path, value, PATTERNS[pattern] ?? k.NAME, rule, required));
export const listCase = (path: string, value: unknown, limit: string): Out => run((c) => k.list(c, path, value, limitOf(limit)));
export const objectCase = (path: string, value: unknown): Out => run((c) => k.object(c, path, value));
export const integerCase = (path: string, value: unknown, min: number, max: number, fallback: number): Out => run((c) => k.integer(c, path, value, [min, max], fallback));
export const flagCase = (path: string, value: unknown, fallback: boolean): Out => run((c) => k.flag(c, path, value, fallback));
export const uniqueCase = (path: string, names: (string | null)[], what: string): Out => run((c) => k.unique(c, path, names, what));
export const symbolOfCase = (path: string, value: unknown, fallback: string): Out => run((c) => k.symbolOf(c, path, value, fallback));
export const isPluginPathCase = (value: string): boolean => k.isPluginPath(value);
export const isArgumentCase = (arg: unknown): boolean => k.isArgument(arg);
export const pluginFileCase = (path: string, value: unknown, rule: string, extension: string | undefined): Out => run((c) => k.pluginFile(c, path, value, rule, extension));
