import { PRESS_MOTIONS, pressMotionNow, setPressMotion, subscribePressMotion } from "./pressMotion.ts";

// Steps: "still" | "bounce" (set; gives the listeners it called), "now", "motions", "subscribe" (a new listener;
// gives its index), "subscribe:N" (listener N again), "unsubscribe:N" (subscription N). Anything else gives null.
// The module state is shared, so the replay ends with no listeners and "still".
export function replayPressMotion(steps: string[]): unknown[] {
  const listeners: (() => void)[] = [];
  const unsubscribes: (() => boolean)[] = [];
  let called: number[] = [];
  const subscribe = (listener: () => void) => unsubscribes.push(subscribePressMotion(listener)) - 1;
  const index = (step: string, prefix: string) => (step.startsWith(prefix) ? Number(step.slice(prefix.length)) : NaN);
  try {
    return steps.map((step) => {
      if (step === "still" || step === "bounce") {
        called = [];
        setPressMotion(step);
        return called;
      }
      if (step === "now") return pressMotionNow();
      if (step === "motions") return [...PRESS_MOTIONS];
      if (step === "subscribe") {
        const id = listeners.length;
        listeners.push(() => void called.push(id));
        return subscribe(listeners[id]!);
      }
      const again = listeners[index(step, "subscribe:")];
      if (again) return subscribe(again);
      const unsubscribe = unsubscribes[index(step, "unsubscribe:")];
      if (unsubscribe) return unsubscribe();
      return null;
    });
  } finally {
    for (const unsubscribe of unsubscribes) unsubscribe();
    setPressMotion("still");
  }
}
