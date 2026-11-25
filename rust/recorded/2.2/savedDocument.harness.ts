import { parseSaved, unchanged, upgraded, versionedJson, type Migrations } from "./savedDocument.ts";

// A migration cannot cross the bridge: each step is data. "unchanged" is `unchanged`; `{ merge }` spreads `merge`
// over the document; `{ value }` returns `value`; anything else returns undefined. `calls` lists the steps run.
const migrationsOf = (steps: unknown, calls: string[]): Migrations =>
  Object.fromEntries(
    Object.entries(steps ?? {}).map(([key, step]) => [
      key,
      (doc: Record<string, unknown>) => {
        calls.push(key);
        if (step === "unchanged") return unchanged(doc);
        if (typeof step === "object" && step !== null && "merge" in step) return { ...doc, ...(step.merge as object) };
        return (step as { value?: unknown }).value;
      },
    ])
  );

export function upgradedCase(doc: unknown, version: number, steps: Record<string, unknown>, field?: string): unknown {
  const calls: string[] = [];
  return { result: upgraded(doc, version, migrationsOf(steps, calls), field), calls };
}

export function parseSavedCase(json: string | null, version: number, steps: Record<string, unknown>, field?: string): unknown {
  const calls: string[] = [];
  return { result: parseSaved(json, version, migrationsOf(steps, calls), field), calls };
}

export const versionedJsonCase = (data: object, version: number, field?: string, space?: number): string => versionedJson(data, version, field, space);

export const unchangedCase = (doc: Record<string, unknown>): unknown => unchanged(doc);
