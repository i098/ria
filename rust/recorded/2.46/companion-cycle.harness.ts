// Ria original, no Ardia source.
import { Clock } from "./hlc.ts";
import { dayOf, historyExpired, isWebUrl, lastVisitOf } from "./companion/adapters.ts";
import {
  companionAcknowledge,
  companionAddBookmark,
  companionRemoveBookmark,
  companionRemovalsDone,
  companionSend,
  companionView,
  runCompanionCycle,
  type CycleInput,
} from "./companion/cycle.ts";
import { companionLoad, companionStateUnsaved, companionUnload, save, viewIsNew, type Host } from "./companion/state.ts";
import type { LogFile } from "./scopeState.ts";

type Step = [string, ...unknown[]];
type Entry = { payload: string; partial: boolean };
type World = {
  folder: Map<string, Map<string, Entry>>;
  counter: number;
  chain: string;
  fail: string[];
  journal: Record<string, LogFile[]>;
  stateJSON: string | null;
  log: unknown[];
  /// The ids that each read of a scope lists as pending (not yet downloaded).
  pending: Record<string, string[]>;
  host: Host;
};

// The companion reads its host from `globalThis.__host` (`companion/state.ts:38`).
const slot = globalThis as { __host?: Host };
const take = (w: World, what: string) => {
  const at = w.fail.indexOf(what);
  if (at >= 0) w.fail.splice(at, 1);
  return at >= 0;
};

function scopeOf(w: World, scope: string) {
  let s = w.folder.get(scope);
  if (!s) w.folder.set(scope, (s = new Map()));
  return s;
}

function add(w: World, scope: string, payload: string, partial = false) {
  const id = `f${String(++w.counter).padStart(4, "0")}`;
  scopeOf(w, scope).set(id, { payload, partial });
  return id;
}

function makeHost(w: World): Host {
  return {
    chainStatus: () => {
      w.log.push(["chain", w.chain]);
      if (take(w, "chain")) throw new Error("chain failed");
      return w.chain;
    },
    read: (scope, known) => {
      w.log.push(["read", scope, known]);
      if (take(w, "read")) throw new Error("read failed");
      const ids = JSON.parse(known) as string[];
      const listing = { files: [] as { id: string; payload: string }[], pending: [...(w.pending[scope] ?? [])], damaged: [] as { id: string; age: number }[], present: [] as string[] };
      for (const [id, e] of scopeOf(w, scope)) {
        listing.present.push(id);
        if (ids.includes(id)) continue;
        if (e.partial) listing.damaged.push({ id, age: 0 });
        else listing.files.push({ id, payload: e.payload });
      }
      return JSON.stringify(listing);
    },
    write: (scope, payload) => {
      if (take(w, "write")) {
        w.log.push(["write", scope, null]);
        throw new Error("disk full");
      }
      const id = add(w, scope, payload);
      w.log.push(["write", scope, id]);
      if (take(w, "writeAfter")) throw new Error("write timed out");
      return id;
    },
    remove: (scope, ids) => {
      w.log.push(["remove", scope, ids]);
      if (take(w, "remove")) throw new Error("remove failed");
      for (const id of JSON.parse(ids) as string[]) scopeOf(w, scope).delete(id);
      return "";
    },
    persistJournal: (scope, batches) => {
      w.log.push(["persist", scope, batches]);
      if (take(w, "persist")) throw new Error("journal disk full");
      if (take(w, "persistNo")) return "no";
      const list = (w.journal[scope] ??= []);
      for (const f of (JSON.parse(batches) as string[]).map((raw) => JSON.parse(raw) as LogFile)) if (!list.some((x) => x.seq === f.seq)) list.push(f);
      return "ok";
    },
    saveState: (json, seqs) => {
      w.log.push(["save", seqs]);
      if (take(w, "save")) throw new Error("state disk full");
      if (take(w, "saveNo")) return "no";
      w.stateJSON = json;
      const saved = JSON.parse(seqs) as Record<string, number>;
      for (const [scope, list] of Object.entries(w.journal)) {
        const left = list.filter((f) => f.seq > (saved[scope] ?? 0));
        if (left.length) w.journal[scope] = left;
        else delete w.journal[scope];
      }
      return "ok";
    },
  };
}

/// A log file of another device: `ops` are `[key, value, wall]`, stamped at `wall` with counter 0.
function peerLog(device: string, seq: number, at: number, ops: [string, unknown, number][]): LogFile {
  return { v: 1, kind: "log", device, seq, at, ops: ops.map(([k, v, wall]) => ({ k, h: new Clock(device, null, () => wall).tick(), v })) };
}

function loadInput(w: World, input: Record<string, unknown>) {
  const out = { ...input };
  if (out.state === "$saved") out.state = w.stateJSON === null ? null : JSON.parse(w.stateJSON);
  if (out.journal === "$journal") out.journal = JSON.parse(JSON.stringify(w.journal));
  return out as Parameters<typeof companionLoad>[0];
}

function show(w: World) {
  const out = {
    folder: [...w.folder].map(([scope, files]) => [scope, [...files].map(([id, e]) => [id, e.payload, e.partial])]),
    journal: w.journal,
    state: w.stateJSON,
    log: w.log,
  };
  w.log = [];
  return out;
}

async function run(w: World, [name, a, b, c, d, e]: Step): Promise<unknown> {
  switch (name) {
    case "host":
      slot.__host = a === false ? undefined : w.host;
      return null;
    case "chain":
      w.chain = a as string;
      return null;
    case "fail":
      w.fail.push(a as string);
      return null;
    case "pending":
      w.pending[a as string] = Array.isArray(b) ? b.filter((x) => typeof x === "string") : [];
      return null;
    case "file":
      return add(w, a as string, typeof b === "string" ? b : JSON.stringify(b), c === true);
    case "peer":
      return add(w, a as string, JSON.stringify(peerLog(b as string, c as number, d as number, e as [string, unknown, number][])));
    case "unlink":
      scopeOf(w, a as string).delete(b as string);
      return null;
    case "load":
      return companionLoad(loadInput(w, a as Record<string, unknown>));
    case "cycle":
      return runCompanionCycle(a as CycleInput);
    case "send":
      return companionSend(a as Parameters<typeof companionSend>[0]);
    case "ack":
      return companionAcknowledge(a as Parameters<typeof companionAcknowledge>[0]);
    case "removalsDone":
      return companionRemovalsDone(a as Parameters<typeof companionRemovalsDone>[0]);
    case "addBookmark":
      return companionAddBookmark(a as Parameters<typeof companionAddBookmark>[0]);
    case "removeBookmark":
      return companionRemoveBookmark(a as Parameters<typeof companionRemoveBookmark>[0]);
    case "view":
      return companionView();
    case "unload":
      return companionUnload();
    case "unsaved":
      return companionStateUnsaved();
    case "save":
      return save(a === true);
    case "viewIsNew":
      return viewIsNew(a as string);
    case "dayOf":
      return dayOf(a as number);
    case "lastVisitOf":
      return lastVisitOf(a as never);
    case "historyExpired":
      return historyExpired(a as string, b, c as number);
    case "isWebUrl":
      return isWebUrl(a as string);
    case "show":
      return show(w);
  }
  return { error: "bad step" };
}

export async function replayCompanion(input: string): Promise<unknown[]> {
  let steps: unknown;
  try {
    steps = JSON.parse(input);
  } catch {
    return [{ error: "bad script" }];
  }
  if (!Array.isArray(steps)) return [{ error: "bad script" }];
  const w = { folder: new Map(), counter: 0, chain: "ok", fail: [], journal: {}, stateJSON: null, log: [], pending: {} } as unknown as World;
  w.host = makeHost(w);
  companionUnload();
  slot.__host = w.host;
  const out: unknown[] = [];
  for (const step of steps) {
    if (!Array.isArray(step) || typeof step[0] !== "string") {
      out.push({ error: "bad step" });
      continue;
    }
    try {
      // A copy: later steps change the objects a result points to (journal, view).
      out.push(structuredClone(await run(w, step as Step)));
    } catch (error) {
      out.push({ error: (error as Error).message });
    }
  }
  return out;
}
