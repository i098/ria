import { withDeadline } from "./deadline.ts";

// A promise cannot cross the bridge: `kind` builds one here. `ms` is capped so a "never" case ends fast.
export async function withDeadlineCase(kind: "resolve" | "reject" | "never", value: unknown, ms: number, fallback: unknown): Promise<unknown> {
  const promise = kind === "never" ? new Promise<never>(() => {}) : kind === "reject" ? Promise.reject(value) : Promise.resolve(value);
  try {
    return { value: await withDeadline(promise, Math.min(ms, 20), fallback) };
  } catch (reason) {
    return { rejected: reason };
  }
}
