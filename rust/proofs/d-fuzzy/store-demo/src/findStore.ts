type Find = { open: boolean; query: string; count: number | null; active: number };
const store: Record<string, Find> = {};
const set = (id: string, p: Partial<Find>) => { store[id] = { ...(store[id] ?? { open: false, query: "", count: null, active: 0 }), ...p }; };
type Action = { type: "open" | "close" | "query" | "step"; tab: string; query?: string };
/** Replay actions on a fresh store; returns the whole state after each action. */
export function replay(actions: Action[]): Record<string, Find>[] {
  for (const k of Object.keys(store)) delete store[k];
  return actions.map((a) => {
    if (a.type === "open") set(a.tab, { open: true });
    else if (a.type === "close") set(a.tab, { open: false, count: null, active: 0 });
    else if (a.type === "query") set(a.tab, { query: a.query ?? "" });
    else set(a.tab, { active: (store[a.tab]?.active ?? 0) + 1 });
    return structuredClone(store);
  });
}
