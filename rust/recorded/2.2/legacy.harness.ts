import * as legacy from "./legacy.ts";

export function fromLegacyUrl(url: string): string {
  return legacy.fromLegacyUrl(url);
}

// `fromLegacy` returns its argument when nothing changed: the flag records that identity.
export function fromLegacyValue(value: unknown): [unknown, boolean] {
  const out = legacy.fromLegacy(value);
  return [out, out === value];
}
