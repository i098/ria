// Ria original, no Ardia source.
import { holdsBack, noteFileHealth, pullSettled } from "./fileHealth.ts";
import { emptyScope, type ScopeState } from "./scopeState.ts";

type Files = { read: string[]; unreadable: string[]; pending: string[]; present: string[] };
type Step = ["set", Partial<ScopeState>] | ["note", Files, number, number] | ["holdsBack", number, number] | ["pullSettled"];

// Replays steps on one scope state; each step gives its result and the file health after it.
export function replayFileHealth(input: string): unknown[] {
  const steps: Step[] = JSON.parse(input);
  const state = emptyScope();
  return steps.map((step) => {
    let result: unknown = null;
    if (step[0] === "set") Object.assign(state, structuredClone(step[1]));
    else if (step[0] === "note") noteFileHealth(state, { ...step[1], present: new Set(step[1].present) }, step[2], step[3]);
    else if (step[0] === "holdsBack") result = holdsBack(state, step[1], step[2]);
    else result = pullSettled(state);
    const { files, damaged, pendingSince, pending } = state;
    return structuredClone({ result, files, damaged, pendingSince: pendingSince ?? null, pending });
  });
}
