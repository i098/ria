import {
  compareVersions,
  extensionApprovalKey,
  extensionIdOf,
  extensionKey,
  extensionsAdapter,
  isExtensionState,
  isRemoval,
  type ExtensionChange,
  type ExtensionIntent,
  type LocalExtensions,
} from "./extensions.ts";

type Args = {
  local?: LocalExtensions | null;
  intents?: Record<string, ExtensionIntent>;
  answers?: boolean[];
  base?: Record<string, unknown>;
  known?: string[];
  queued?: string[];
  probe?: string[];
  publish?: string[];
  visible?: [string, unknown][];
  changed?: string[];
  joined?: boolean;
  now?: number;
  held?: boolean[];
};

const pure: Record<string, (...args: never[]) => unknown> = { extensionKey, extensionApprovalKey, extensionIdOf, compareVersions, isRemoval, isExtensionState };

// Replays a script of steps against one scripted host: `host` sets what the host answers, `adapter`
// starts a new adapter, `extract`/`apply`/`reconcile` drive it and return the host calls made.
export async function replayExtensions(input: string): Promise<unknown[]> {
  let steps: unknown;
  try {
    steps = JSON.parse(input);
  } catch {
    steps = null;
  }
  // Fuzzed text is rarely a script: one answer for anything that is not an array of named steps.
  if (!Array.isArray(steps) || !steps.every((s) => Array.isArray(s) && typeof s[0] === "string")) return ["not a script"];
  let local: LocalExtensions | null = null;
  let intents: Record<string, ExtensionIntent> = {};
  let answers: boolean[] = [];
  let calls: unknown[] = [];
  const host = {
    read: async () => (calls.push(["read"]), structuredClone(local)),
    change: async (change: ExtensionChange) => (calls.push(["change", change]), answers.shift() ?? true),
    intents: () => (calls.push(["intents"]), structuredClone(intents)),
    settleIntents: (published: Record<string, number>) => void calls.push(["settle", published]),
    keepInstalled: (id: string) => void calls.push(["keep", id]),
  };
  const heldBy = (held: boolean[]) => () => (calls.push(["held"]), held.length > 1 ? held.shift()! : (held[0] ?? false));
  let adapter = extensionsAdapter(host);
  const out: unknown[] = [];
  for (const [name, ...args] of steps as [string, ...unknown[]][]) {
    if (Object.hasOwn(pure, name)) {
      out.push(pure[name]!(...(args as never[])));
      continue;
    }
    const a = (args[0] ?? {}) as Args;
    let result: unknown = null;
    if (name === "host") {
      if (a.local !== undefined) local = a.local;
      if (a.intents) intents = a.intents;
      if (a.answers) answers = a.answers;
      out.push(null);
      continue;
    } else if (name === "adapter") {
      adapter = extensionsAdapter(host);
      out.push(null);
      continue;
    } else if (name === "extract") {
      const base = a.base ?? {};
      const x = await adapter.extract(
        (key) => (Object.hasOwn(base, key) ? base[key] : undefined),
        (key) => (a.queued ?? []).includes(key),
        0,
        (key) => (a.known ?? []).includes(key),
        () => undefined,
        () => undefined,
        () => undefined,
      );
      if (x) {
        const probe = a.probe ?? [];
        result = { values: [...x.values], ignore: probe.map((k) => x.ignore!(k)), republish: probe.map((k) => x.republish!(k)) };
        if (a.publish) x.published!(new Set(a.publish));
      }
    } else if (name === "apply") {
      result = await adapter.apply(new Map(a.visible ?? []), new Set(a.changed ?? []), () => undefined, {
        known: () => false,
        now: 0,
        horizon: 0,
        joined: a.joined ?? true,
        held: heldBy(a.held ?? []),
        landed: () => {},
      });
    } else if (name === "reconcile") {
      await adapter.reconcile(new Map(a.visible ?? []), a.now ?? 0, a.held && heldBy(a.held));
    } else {
      out.push(["unknown step", name]);
      continue;
    }
    out.push({ result, calls });
    calls = [];
  }
  return out;
}
