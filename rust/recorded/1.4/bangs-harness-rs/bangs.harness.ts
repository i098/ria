// Recording harness for bangs.ts: a BangTable is an object with a `get` method, which JSON
// cannot carry, so these wrappers take slim lists and triggers and return what the tables answer.
import { bangTable, checkedBangTable, findBang, joinBangTables, readBangDocument, type BangTable } from "./bangs.ts";

/** The table's size and `get` of each trigger in order; a repeated trigger reads the cache. */
const view = (table: BangTable, triggers: readonly string[]) => ({ size: table.size, bangs: triggers.map((t) => table.get(t) ?? null) });

export const bangTableView = (slim: string, triggers: readonly string[]) => view(bangTable(slim), triggers);

export function checkedBangTableView(slim: string, triggers: readonly string[]) {
  const table = checkedBangTable(slim);
  return table && view(table, triggers);
}

export function readBangDocumentView(doc: string | null | undefined, triggers: readonly string[]) {
  const read = readBangDocument(doc);
  return read && { etag: read.etag, ...view(read.table, triggers) };
}

export const joinBangTablesView = (slims: readonly string[], triggers: readonly string[]) => view(joinBangTables(slims.map(bangTable)), triggers);

/** `findBang` of each text against the slim lists joined, or against no table when `slims` is null. */
export const findBangIn = (texts: readonly string[], slims: readonly string[] | null) => {
  const table = slims && joinBangTables(slims.map(bangTable));
  return texts.map((text) => findBang(text, table));
};
