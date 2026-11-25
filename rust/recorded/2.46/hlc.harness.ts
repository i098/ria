// Ria original, no Ardia source.
import { Clock, hlcWall } from "./hlc.ts";

type Step = ["now", number] | ["tick"] | ["at", number] | ["observe", string] | ["last"] | ["hlcWall", string];
type Session = { device: string; last?: string | null; now: number; steps: Step[] };

// Drives one clock per session with a fake `now`; each step gives the clock's answer.
export function replayClock(input: string): unknown[] {
  const sessions: Session[] = JSON.parse(input);
  return sessions.map(({ device, last, now, steps }) => {
    let time = now;
    const clock = new Clock(device, last, () => time);
    const out: unknown[] = [clock.device, clock.last];
    for (const step of steps) {
      if (step[0] === "now") time = step[1];
      else if (step[0] === "tick") out.push(clock.tick());
      else if (step[0] === "at") out.push(clock.at(step[1]));
      else if (step[0] === "observe") clock.observe(step[1]);
      else if (step[0] === "last") out.push(clock.last);
      else out.push(hlcWall(step[1]));
    }
    return out;
  });
}
