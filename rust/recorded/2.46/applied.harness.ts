// Ria original, no Ardia source.
import { covered, isApplied, markApplied, raiseApplied } from "./applied.ts";
import { emptyScope } from "./scopeState.ts";

type Step = ["set", { vv?: Record<string, number>; extra?: Record<string, number[]> }] | ["covered", Record<string, number>, [string, number]] | ["isApplied" | "markApplied" | "raiseApplied", string, number];

// Replays steps on one scope state; each step gives its result and the `vv` and `extra` after it.
export function replayApplied(input: string): unknown[] {
  const steps: Step[] = JSON.parse(input);
  const state = emptyScope();
  return steps.map((step) => {
    let result: unknown = null;
    if (step[0] === "set") Object.assign(state, structuredClone(step[1]));
    else if (step[0] === "covered") result = covered(step[1], step[2]);
    else if (step[0] === "isApplied") result = isApplied(state, [step[1], step[2]]);
    else if (step[0] === "markApplied") markApplied(state, step[1], step[2]);
    else raiseApplied(state, step[1], step[2]);
    return { result, vv: structuredClone(state.vv), extra: structuredClone(state.extra) };
  });
}
