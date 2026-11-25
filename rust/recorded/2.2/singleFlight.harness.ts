import { singleFlight } from "./singleFlight.ts";

type Pass = { join: boolean; repeat: boolean; fail: boolean };

// Runs flights until every pass is used. Pass `i` waits once, then optionally calls the flight
// again (joins), asks to repeat, and fails. `keepGoing` answers from `keep` (only `false` stops).
export async function replaySingleFlight(passes: Pass[], keep: boolean[]): Promise<unknown[]> {
  const list = Array.isArray(passes) ? passes : [];
  const answers = Array.isArray(keep) ? keep : [];
  const log: unknown[] = [];
  let next = 0;
  let asked = 0;
  let current: Promise<void> | null = null;
  const run = singleFlight(
    async (flight) => {
      const i = next++;
      const p = list[i];
      log.push(["pass", i, flight.again]);
      await null;
      if (p?.join === true) log.push(["join", run() === current]);
      if (p?.repeat === true) flight.repeat();
      log.push(["again", flight.again]);
      if (p?.fail === true) await Promise.reject(new Error(`pass ${i}`));
    },
    () => {
      const v = answers[asked++] !== false;
      log.push(["keepGoing", v]);
      return v;
    },
  );
  do {
    current = run();
    try {
      await current;
      log.push(["done"]);
    } catch (error) {
      log.push(["failed", (error as Error).message]);
    }
  } while (next < list.length);
  return log;
}
