import { errorText } from "./errors.ts";

// An `Error` cannot cross the bridge: `asError` builds one from the value here.
export const errorTextCase = (value: unknown, asError: boolean): string => errorText(asError ? new Error(String(value)) : value);
