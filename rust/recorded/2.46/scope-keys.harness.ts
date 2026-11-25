// Ria original, no Ardia source.
import { canonical } from "./canon.ts";
import { baseKeysWith, dropBase, dropRecord, putBase, putRecord, recordsWith, trackTouchedKeys, visible, withBaseIndex, type TouchedKeys } from "./scopeKeys.ts";
import { emptyScope, type ScopeState, type SyncRecord } from "./scopeState.ts";

type Step = [string, ...unknown[]];
type World = { state: ScopeState; trackers: { touched: TouchedKeys; stop: () => void }[] };

async function run(w: World, step: Step): Promise<unknown> {
  const [name, a, b] = step;
  switch (name) {
    case "state":
      w.state = { ...emptyScope(), ...(a as Partial<ScopeState>) };
      return null;
    case "visible":
      return [...visible(w.state, a as string)];
    case "recordsWith":
      return recordsWith(w.state, a as string);
    case "track":
      w.trackers.push(trackTouchedKeys(w.state));
      return w.trackers.length - 1;
    case "stop":
      w.trackers[a as number]!.stop();
      return null;
    case "touched": {
      const { records, base } = w.trackers[a as number]!.touched;
      return { records: [...records], base: [...base] };
    }
    case "putRecord":
      putRecord(w.state, a as string, b as SyncRecord);
      return null;
    case "dropRecord":
      dropRecord(w.state, a as string);
      return null;
    case "putBase":
      putBase(w.state, a as string, b as string);
      return null;
    case "dropBase":
      dropBase(w.state, a as string);
      return null;
    case "baseKeysWith":
      return [...baseKeysWith(w.state, a as string)];
    case "withBaseIndex":
      return withBaseIndex(w.state, () => replaySteps(w, a as Step[]));
    case "show":
      return canonical(w.state);
  }
  return { error: "bad step" };
}

async function replaySteps(w: World, steps: Step[]): Promise<unknown[]> {
  const out: unknown[] = [];
  for (const step of steps) out.push(Array.isArray(step) ? await run(w, step) : { error: "bad step" });
  return out;
}

export async function replayScopeKeys(input: string): Promise<unknown[]> {
  let steps: unknown;
  try {
    steps = JSON.parse(input);
  } catch {
    return [{ error: "bad script" }];
  }
  if (!Array.isArray(steps)) return [{ error: "bad script" }];
  return replaySteps({ state: emptyScope(), trackers: [] }, steps as Step[]);
}
