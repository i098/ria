import * as k from "./pluginMigrations.ts";

// A migration case runs the step of `MANIFEST_MIGRATIONS` for `version` on a copy of `doc`. A
// `doc` that is not a plain object (the recorder makes those) throws, as the TS type asks.
const docIn = (doc: unknown): Record<string, unknown> => {
  if (typeof doc === "object" && doc !== null && !Array.isArray(doc)) return doc as Record<string, unknown>;
  throw new TypeError("doc is not an object");
};
export const contributesFromV1Case = (doc: unknown): unknown => k.contributesFromV1(docIn(doc));
export const migrationCase = (version: number, doc: unknown): unknown => k.MANIFEST_MIGRATIONS[version]!({ ...docIn(doc) });
export const v1ProblemCase = (problem: unknown): string => {
  if (typeof problem !== "string") throw new TypeError("problem is not a string");
  return k.v1Problem(problem);
};
