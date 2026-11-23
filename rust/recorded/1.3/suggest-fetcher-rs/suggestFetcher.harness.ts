// Recording harness for suggestFetcher.ts: the fetcher runs on timers and promises and answers
// through a callback, so `runFetcher` plays a script with a fake timer and a fake fetch and
// returns what happened, in order: fetches, aborts and delivered results.
import type { SearchEngine } from "./engines.ts";
import { createSuggestFetcher } from "./suggestFetcher.ts";

type Step =
  | { op: "request"; engine: SearchEngine; query: string }
  | { op: "cancel" }
  // run the pending timer, if any
  | { op: "fire" }
  // answer the oldest open fetch: a response, a response whose body fails to read, or a failure
  | { op: "reply"; ok: boolean; body: string }
  | { op: "unreadable" }
  | { op: "fail" };

type Reply = { ok: boolean; text(): Promise<string> };
type Open = { resolve: (res: Reply) => void; reject: (e: Error) => void };

export async function runFetcher(steps: Step[], cacheSize?: number): Promise<unknown[]> {
  const log: unknown[] = [];
  const realSet = globalThis.setTimeout;
  const realClear = globalThis.clearTimeout;
  const timers = new Map<number, () => void>();
  let nextTimer = 1;
  globalThis.setTimeout = ((fn: () => void) => {
    timers.set(nextTimer, fn);
    return nextTimer++;
  }) as unknown as typeof setTimeout;
  globalThis.clearTimeout = ((id: number) => void timers.delete(id)) as unknown as typeof clearTimeout;
  const open: Open[] = [];
  try {
    const fetcher = createSuggestFetcher({
      fetch: (url, init) => {
        const { promise, resolve, reject } = Promise.withResolvers<Reply>();
        log.push({ fetch: url, engine: init.engine?.id ?? null });
        init.signal.addEventListener("abort", () => log.push({ abort: url }));
        open.push({ resolve, reject });
        return promise;
      },
      cacheSize,
    });
    for (const step of steps) {
      if (step.op === "request") fetcher.request(step.engine, step.query, (queries, sites) => log.push({ result: { queries, sites } }));
      else if (step.op === "cancel") fetcher.cancel();
      else if (step.op === "fire") {
        for (const [id, fn] of [...timers]) {
          timers.delete(id);
          fn();
        }
      } else {
        const o = open.shift();
        if (o && step.op === "fail") o.reject(new Error("offline"));
        else if (o && step.op === "unreadable") o.resolve({ ok: true, text: () => Promise.reject(new Error("unreadable")) });
        else if (o && step.op === "reply") o.resolve({ ok: step.ok, text: async () => step.body });
      }
      const { promise, resolve } = Promise.withResolvers<void>();
      realSet(resolve, 0);
      await promise;
    }
  } finally {
    globalThis.setTimeout = realSet;
    globalThis.clearTimeout = realClear;
  }
  return log;
}
