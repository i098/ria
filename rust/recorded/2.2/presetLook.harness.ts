import { trackPress } from "./presetLook.ts";

type Step = { op?: unknown; trackPixels?: unknown; pageX?: unknown; trackX?: unknown; snapTo?: unknown };

// Drives one `trackPress()`: "armed" gives `armed()`, "grant" calls `grant(trackPixels)` and gives null,
// "at" gives `at(pageX, trackX, trackPixels, snapTo)` (snapTo left out when not a number). Anything else gives "skip".
export function replayTrackPress(steps: Step[]): unknown[] {
  const press = trackPress();
  const n = (v: unknown) => (typeof v === "number" ? v : NaN);
  return (Array.isArray(steps) ? steps : []).map((step) => {
    if (step?.op === "armed") return press.armed();
    if (step?.op === "grant") return void press.grant(n(step.trackPixels)) ?? null;
    if (step?.op === "at") return press.at(n(step.pageX), n(step.trackX), n(step.trackPixels), typeof step.snapTo === "number" ? step.snapTo : undefined);
    return "skip";
  });
}
