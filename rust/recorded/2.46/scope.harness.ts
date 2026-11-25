// Ria original, no Ardia source.
import { canonical } from "./canon.ts";
import { Clock } from "./hlc.ts";
import { compact, compactionDue, flush, ingestSnapshot, parseSyncFile, pull, restoreJournal, syncScope, writtenSeq } from "./scope.ts";
import { emptyScope, type Adapter, type Listing, type LogFile, type ScopeOptions, type ScopeState, type SnapshotFile, type Transport } from "./scopeState.ts";

type Step = [string, ...unknown[]];
type Entry = { payload: string; hidden: string[]; partial: string[] };
type AdapterConfig = {
  prefix: string;
  store?: Record<string, unknown>;
  extract?: "null" | "throw";
  republish?: string[];
  ignore?: string[];
  json?: Record<string, string>;
  editedAt?: string;
  published?: boolean;
  probe?: string[];
  apply?: "store" | "sets" | "refuse" | "throw" | "void";
  land?: boolean;
  landed?: string[];
  checkHeld?: boolean;
  adoptRemoteOnJoin?: boolean;
  publishAfterPull?: boolean;
};
type OptionsConfig = Record<string, unknown> & { expired?: string[]; persist?: boolean; holdApply?: boolean[] };
type Device = {
  id: string;
  state: ScopeState;
  clock: Clock;
  offset: number;
  adapters: { config: AdapterConfig; store: Map<string, unknown> }[];
  options: OptionsConfig;
  holds: boolean[];
  journal: Map<number, LogFile>;
  fail: Set<string>;
  onRead: unknown[][] | null;
  saved: string;
  savedClock: string | null;
};
type World = { now: number; counter: number; folder: Map<string, Entry>; log: unknown[]; devices: Map<string, Device>; d: Device };

const NUMBERS = ["tombstoneTTL", "compactAfterFiles", "compactAfterLogs", "compactLogShare", "compactInterval", "pruneGrace", "damagedGiveUp"] as const;
const u = (v: unknown) => (v === undefined ? "<undefined>" : v);
const time = (w: World, d: Device) => () => w.now + d.offset;

function edit(d: Device, key: string, value: unknown, set: boolean) {
  const adapter = d.adapters.find((a) => key.startsWith(a.config.prefix));
  if (!adapter) return;
  if (set) adapter.store.set(key, value);
  else adapter.store.delete(key);
}

function configure(w: World, d: Device, config: { adapters?: AdapterConfig[]; options?: OptionsConfig; offset?: number }) {
  if (config.adapters) d.adapters = config.adapters.map((c) => ({ config: c, store: new Map(Object.entries(c.store ?? {})) }));
  if (config.options) {
    d.options = config.options;
    d.holds = [...(config.options.holdApply ?? [])];
  }
  if (typeof config.offset === "number") d.offset = config.offset;
}

function device(w: World, id: string): Device {
  const d: Device = { id, state: emptyScope(), clock: new Clock(id), offset: 0, adapters: [], options: {}, holds: [], journal: new Map(), fail: new Set(), onRead: null, saved: JSON.stringify(emptyScope()), savedClock: null };
  d.clock = new Clock(id, null, time(w, d));
  return d;
}

function transport(w: World, d: Device): Transport {
  return {
    write: async (scope, payload) => {
      w.log.push(["write", d.id, scope]);
      if (d.fail.delete("write")) throw new Error("write failed");
      const id = `f${++w.counter}`;
      w.folder.set(id, { payload, hidden: [], partial: [] });
      if (d.fail.delete("writeAfter")) throw new Error("write timed out");
      return id;
    },
    read: async (scope, known): Promise<Listing> => {
      w.log.push(["read", d.id, scope, known]);
      if (d.fail.delete("read")) throw new Error("read failed");
      const listing: Listing = { files: [], pending: [], damaged: [], present: [] };
      for (const [id, e] of w.folder) {
        listing.present.push(id);
        if (known.includes(id)) continue;
        if (e.hidden.includes(d.id)) listing.pending.push(id);
        else if (e.partial.includes(d.id)) listing.damaged.push({ id, age: 0 });
        else listing.files.push({ id, payload: e.payload });
      }
      for (const [key, ...value] of d.onRead ?? []) edit(d, key as string, value[0], value.length > 0);
      d.onRead = null;
      return listing;
    },
    remove: async (scope, ids) => {
      w.log.push(["remove", d.id, scope, ids]);
      if (d.fail.delete("remove")) throw new Error("remove failed");
      for (const id of ids) w.folder.delete(id);
    },
  };
}

function adapter(w: World, c: AdapterConfig, store: Map<string, unknown>): Adapter {
  return {
    prefix: c.prefix,
    adoptRemoteOnJoin: c.adoptRemoteOnJoin,
    publishAfterPull: c.publishAfterPull,
    editedAt:
      c.editedAt === undefined
        ? undefined
        : (value) => {
            const at = value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>)[c.editedAt!] : undefined;
            return typeof at === "number" ? at : undefined;
          },
    extract: async (base, queued, now, known, remote, stamp, baseJson) => {
      w.log.push(["extract", c.prefix, now]);
      for (const k of c.probe ?? []) w.log.push(["probe", k, u(base(k)), queued(k), known(k), u(remote(k)), u(stamp(k)), u(baseJson(k))]);
      if (c.extract === "null") return null;
      if (c.extract === "throw") throw new Error("extract failed");
      const { republish, ignore, json } = c;
      return {
        values: new Map(store),
        republish: republish && ((k) => republish.includes(k)),
        ignore: ignore && ((k) => ignore.includes(k)),
        json: json && ((k) => (Object.hasOwn(json, k) ? json[k] : undefined)),
        published: c.published ? (keys) => void w.log.push(["published", c.prefix, [...keys]]) : undefined,
      };
    },
    apply: async (visible, changed, base, records) => {
      const entry: unknown[] = ["apply", c.prefix, [...changed], records.joined, records.now, records.horizon, [...changed].map((k) => [records.known(k), u(base(k))])];
      if (c.checkHeld) entry.push(records.held());
      w.log.push(entry);
      if (c.apply === "throw") throw new Error("apply failed");
      for (const k of changed) {
        if (visible.has(k)) store.set(k, visible.get(k));
        else if (c.apply !== "sets") store.delete(k);
        if (c.land) records.landed(k);
      }
      for (const k of c.landed ?? []) records.landed(k);
      if (c.apply === "refuse") return false;
      if (c.apply === "void") return undefined;
      return true;
    },
  };
}

function options(w: World, d: Device): ScopeOptions {
  const o = d.options;
  const out: ScopeOptions = {};
  for (const k of NUMBERS) if (typeof o[k] === "number") out[k] = o[k] as number;
  const { expired } = o;
  if (expired) out.expired = (key) => expired.includes(key);
  if (o.persist) {
    out.persist = async (batches) => {
      w.log.push(["persist", d.id, batches.map((f) => f.seq)]);
      if (d.fail.delete("persist")) throw new Error("persist failed");
      for (const f of batches) d.journal.set(f.seq, JSON.parse(JSON.stringify(f)) as LogFile);
    };
  }
  if (o.holdApply) {
    out.holdApply = () => {
      const held = d.holds.shift() ?? false;
      w.log.push(["hold", held]);
      return held;
    };
  }
  return out;
}

function show(w: World) {
  const { d } = w;
  const out = {
    device: d.id,
    state: d.state,
    clock: d.clock.last,
    stores: d.adapters.map((a) => Object.fromEntries(a.store)),
    journal: [...d.journal.keys()],
    folder: [...w.folder].map(([id, e]) => [id, e.payload]),
    log: w.log,
  };
  w.log = [];
  return canonical(out);
}

function entry(w: World, id: string) {
  let e = w.folder.get(id);
  if (!e) w.folder.set(id, (e = { payload: "", hidden: [], partial: [] }));
  return e;
}

async function run(w: World, [name, a, b]: Step): Promise<unknown> {
  const d = w.d;
  const now = time(w, d);
  switch (name) {
    case "device": {
      let next = w.devices.get(a as string);
      if (!next) w.devices.set(a as string, (next = device(w, a as string)));
      w.d = next;
      configure(w, next, (b ?? {}) as Parameters<typeof configure>[2]);
      return null;
    }
    case "now":
      w.now = a as number;
      return null;
    case "tick":
      w.now += a as number;
      return null;
    case "state":
      d.state = { ...emptyScope(), ...(a as Partial<ScopeState>) };
      return null;
    case "relaunch":
      d.state = JSON.parse(JSON.stringify(d.state)) as ScopeState;
      d.clock = new Clock(d.id, d.clock.last, now);
      return null;
    case "clock":
      d.clock = new Clock(d.id, a as string | null, now);
      return d.clock.last;
    case "set":
      edit(d, a as string, b, true);
      return null;
    case "del":
      edit(d, a as string, undefined, false);
      return null;
    case "onRead":
      d.onRead = a as unknown[][];
      return null;
    case "fail":
      d.fail.add(a as string);
      return null;
    case "file":
      entry(w, a as string).payload = b as string;
      return null;
    case "drop":
      w.folder.delete(a as string);
      return null;
    case "hide":
    case "partial": {
      const e = entry(w, a as string);
      const list = name === "hide" ? e.hidden : e.partial;
      if (b === null) list.length = 0;
      else list.push(b as string);
      return null;
    }
    case "sync":
      return syncScope({ scope: "s", state: d.state, transport: transport(w, d), adapters: d.adapters.map((x) => adapter(w, x.config, x.store)), clock: d.clock, device: d.id, now, options: options(w, d) });
    case "flush":
      await flush(d.state, transport(w, d), "s", d.id, now(), options(w, d));
      return null;
    case "pull":
      return [...(await pull(d.state, transport(w, d), "s", d.clock, now(), options(w, d)))];
    case "compact":
      return compact(d.state, transport(w, d), "s", d.id, now(), options(w, d), a === true);
    case "due":
      return compactionDue(d.state, now(), options(w, d));
    case "restore":
      restoreJournal(d.state, (a ?? [...d.journal.values()]) as LogFile[], d.clock, b as number | undefined);
      return null;
    case "save":
      d.saved = JSON.stringify(d.state);
      d.savedClock = d.clock.last;
      for (const seq of d.journal.keys()) if (seq <= d.state.seq) d.journal.delete(seq);
      return null;
    case "crash":
      d.state = JSON.parse(d.saved) as ScopeState;
      d.clock = new Clock(d.id, d.savedClock, now);
      restoreJournal(d.state, [...d.journal.values()], d.clock);
      return null;
    case "parse":
      return canonical(parseSyncFile(a as string));
    case "ingest": {
      const changed = new Set<string>();
      ingestSnapshot(d.state, a as SnapshotFile, d.clock, changed);
      return [...changed];
    }
    case "written":
      return writtenSeq(d.state);
    case "show":
      return show(w);
  }
  return { error: "bad step" };
}

export async function replayScope(input: string): Promise<unknown[]> {
  let steps: unknown;
  try {
    steps = JSON.parse(input);
  } catch {
    return [{ error: "bad script" }];
  }
  if (!Array.isArray(steps)) return [{ error: "bad script" }];
  const w = { now: 1000, counter: 0, folder: new Map(), log: [], devices: new Map() } as unknown as World;
  w.d = device(w, "devA");
  w.devices.set("devA", w.d);
  const out: unknown[] = [];
  for (const step of steps) {
    if (!Array.isArray(step)) {
      out.push({ error: "bad step" });
      continue;
    }
    try {
      out.push(await run(w, step as Step));
    } catch (error) {
      out.push({ error: (error as Error).message });
    }
  }
  return out;
}
