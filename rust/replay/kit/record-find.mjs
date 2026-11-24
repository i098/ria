// Ria original, no Ardia source.

import { once } from "node:events";
import { findReplay } from "./find.mjs";

const [scratch] = process.argv.slice(2);
if (!scratch) throw new Error("Usage: node record-find.mjs ARDIA_SCRATCH");
const replay = await findReplay(scratch);

for (const query of ["", "a", " a ", "café", "日本語", "🧪"]) {
  for (const replace of [undefined, false, true]) {
    for (const backwards of [false, true]) {
      for (const mounted of [false, true]) {
        for (const tab of ["a", "b", "missing"]) {
          const other = tab === "b" ? "a" : "b";
          const match = { count: query.length, active: Number(query !== "") };
          const actions = [
            ...(mounted ? [{ type: "mount", tab, match }] : []),
            { type: "step", tab, backwards },
            { type: "open", tab, now: 1000, options: { query, replace } },
            { type: "open", tab: other, now: 1001, options: { query: "other" } },
            { type: "open", tab, now: 1002 },
            { type: "takeFocus", tab },
            { type: "setMatches", tab, ...match, count: query === "" ? null : match.count },
            { type: "step", tab, backwards },
            { type: "close", tab },
            { type: "step", tab, backwards: !backwards },
            { type: "takeCounts" },
            { type: "takeCounts" },
            { type: "submitRepeatsKey", keyAt: 1000, now: backwards ? 1250 : 1249 },
          ];
          const line = JSON.stringify({ fn: "replay", args: [actions], ok: await replay(actions) });
          if (!process.stdout.write(`${line}\n`)) await once(process.stdout, "drain");
        }
      }
    }
  }
}
