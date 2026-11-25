import { themeGesture } from "./themeGesture.ts";

type Step = { op?: unknown; target?: unknown; intensity?: unknown; settle?: unknown };

const look = (intensity: number) => ({ stops: [{ x: 0.5, y: 0.5, color: { r: 0.5, g: 0.4, b: 0.6, a: 1 } }], intensity, mode: "auto" as const, grain: 0, texture: "grain" as const, translucency: 1 });

// The rig of `themeGesture.test.mjs`, driven by steps. Ops: "begin" (target), "change" (intensity, a number
// or null for no theme; target; settle), "selectionChanged", "cancel", "dispose", "show" (the preset shown
// becomes target), "outside" (the look changes from outside), "tick" (runs the waiting frames). Each dep call
// is logged in order, and each step ends with ["frames", waiting frames]. A step it doesn't know logs ["skip"].
export function replayThemeGesture(showing: string, steps: Step[]): unknown[] {
  const log: unknown[] = [];
  const frames = new Map<number, () => void>();
  let next = 1;
  const state = { showing: String(showing), current: {} };
  const g = themeGesture({
    apply: (theme) => {
      log.push(["apply", state.showing, theme?.intensity ?? null]);
      state.current = {};
    },
    showing: () => state.showing,
    current: () => state.current,
    draft: (edit) => void log.push(["draft", edit === null ? null : [edit.theme?.intensity ?? null, edit.target]]),
    requestFrame: (run) => {
      log.push(["requestFrame", next]);
      frames.set(next, run);
      return next++;
    },
    cancelFrame: (handle) => {
      log.push(["cancelFrame", handle]);
      frames.delete(handle);
    },
  });
  for (const step of Array.isArray(steps) ? steps : []) {
    const target = typeof step?.target === "string" ? step.target : "";
    switch (step?.op) {
      case "begin":
        g.begin(target);
        break;
      case "change":
        g.change(typeof step.intensity === "number" ? look(step.intensity) : null, target, step.settle === true);
        break;
      case "selectionChanged":
        g.selectionChanged();
        break;
      case "cancel":
        g.cancel();
        break;
      case "dispose":
        g.dispose();
        break;
      case "show":
        state.showing = target;
        break;
      case "outside":
        state.current = {};
        break;
      case "tick": {
        const runs = [...frames.values()];
        frames.clear();
        runs.forEach((run) => run());
        break;
      }
      default:
        log.push(["skip"]);
    }
    log.push(["frames", frames.size]);
  }
  return log;
}
