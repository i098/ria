import { orWarn, passwordFailure } from "./fallback.ts";

const ACTIONS = ["browser import", "csv import", "settings edit", "settings delete", "sync delete"];

const thrown = (value: unknown, asError: boolean): unknown => (asError ? new Error(String(value)) : value);

export const passwordFailureCase = (value: unknown, asError: boolean): string => passwordFailure(thrown(value, asError));

// `orWarn` returns a function and writes to `console.warn`: this records both. An `action` outside the
// `PasswordAction` type (which TS rejects at compile time) records `{ error }`.
export function orWarnCase(
  fallback: unknown,
  action: "browser import" | "csv import" | "settings edit" | "settings delete" | "sync delete" | undefined,
  value: unknown,
  asError: boolean
): { result: unknown; warned: string[] } | { error: string } {
  if (action !== undefined && !ACTIONS.includes(action)) return { error: `not a PasswordAction: ${action}` };
  const warn = console.warn;
  const warned: string[] = [];
  console.warn = (...args: unknown[]) => void warned.push(args.join(" "));
  try {
    return { result: orWarn(fallback, action)(thrown(value, asError)), warned };
  } finally {
    console.warn = warn;
  }
}
