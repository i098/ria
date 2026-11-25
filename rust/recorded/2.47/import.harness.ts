import { getEventListeners } from "node:events";
import { ArdiaError } from "@ardia/core";

type Reply = { text: string | null } | { value: unknown } | { error: { code?: string; message: string } };
type Action = { emit: Record<string, unknown> } | { abort: true };
type Script = { actions?: Action[]; reply: Reply };
type SignalMode = "none" | "live" | "aborted";
type Options = {
  historyLimit?: number;
  historySince?: number;
  spaceIds?: string[];
  chunkSize?: number;
  onProgress?: boolean;
  onHistoryChunk?: boolean;
  signal?: SignalMode;
};

let runs = 0;

// One replay is one fresh load of index.ts (its job counter starts at 0) over a fake native module.
// Each step is [export, ...arguments, script]; the script says what the fake does and answers.
export async function replayImport(input: string): Promise<unknown[]> {
  const steps: [string, ...unknown[]][] = JSON.parse(input);
  const listeners = new Set<(event: unknown) => void>();
  let calls: unknown[][] = [];
  let progress: unknown[] = [];
  let chunks: unknown[] = [];
  let controller: AbortController | undefined;
  let script: Script;

  const log = (name: string, args: unknown[]) => calls.push(JSON.parse(JSON.stringify([name, ...args])));
  const answer = (jobId?: string) => {
    for (const action of script.actions ?? []) {
      if ("abort" in action) controller!.abort();
      else {
        const event = { ...action.emit, ...(action.emit.jobId === "$job" ? { jobId } : {}) };
        for (const listener of [...listeners]) listener(event);
      }
    }
    const reply = script.reply;
    if ("error" in reply) {
      return Promise.reject(reply.error.code === undefined ? new Error(reply.error.message) : new ArdiaError(reply.error.code, reply.error.message));
    }
    return "text" in reply ? reply.text : reply.value;
  };
  const call = async (name: string, args: unknown[], jobId?: string) => {
    log(name, args);
    return answer(jobId);
  };
  const now = (name: string, args: unknown[]) => {
    log(name, args);
    return answer() as unknown;
  };

  (globalThis as { __riaNative?: unknown }).__riaNative = {
    addListener: (_name: string, listener: (event: unknown) => void) => {
      listeners.add(listener);
      return { remove: () => listeners.delete(listener) };
    },
    listBrowsers: () => call("listBrowsers", []),
    importData: (jobId: string, b: string, p: string, kinds: string[], o: unknown) => call("importData", [jobId, b, p, kinds, o], jobId),
    cancelImport: (jobId: string) => log("cancelImport", [jobId]),
    unlockBrowser: (b: string, password: string | null) => call("unlockBrowser", [b, password]),
    isBrowserUnlocked: (b: string) => now("isBrowserUnlocked", [b]),
    forgetUnlockedKeys: () => void now("forgetUnlockedKeys", []),
    writeImported: (t: string, p: string, kinds: string[]) => call("writeImported", [t, p, kinds]),
    discardImported: (t: string) => void now("discardImported", [t]),
    listExtensions: (b: string, p: string) => call("listExtensions", [b, p]),
    importSafariExport: (jobId: string, path: string) => call("importSafariExport", [jobId, path], jobId),
    safariHasFullDiskAccess: () => now("safariHasFullDiskAccess", []),
    openFullDiskAccessSettings: () => call("openFullDiskAccessSettings", []),
    importSafariDirect: (jobId: string) => call("importSafariDirect", [jobId], jobId),
    importBookmarksHTML: (path: string) => call("importBookmarksHTML", [path]),
    importPasswordsCSV: (path: string) => call("importPasswordsCSV", [path]),
    chooseImportFile: (kind: string) => call("chooseImportFile", [kind]),
    readDiaTabs: () => call("readDiaTabs", []),
  };
  const mod = await import(`./index.ts?run=${++runs}`);

  const signalFor = (mode: SignalMode | undefined) => {
    if (mode === undefined || mode === "none") return undefined;
    controller = new AbortController();
    if (mode === "aborted") controller.abort();
    return controller.signal;
  };
  const optionsFor = (o: Options) => ({
    historyLimit: o.historyLimit,
    historySince: o.historySince,
    spaceIds: o.spaceIds,
    chunkSize: o.chunkSize,
    onProgress: o.onProgress ? (p: unknown) => progress.push(JSON.parse(JSON.stringify(p))) : undefined,
    onHistoryChunk: o.onHistoryChunk ? (c: unknown) => chunks.push(c) : undefined,
    signal: signalFor(o.signal),
  });
  const run = (op: string, a: any[]): unknown => {
    switch (op) {
      case "listBrowsers": return mod.listBrowsers();
      case "importData": return mod.importData(a[0], a[1], a[2], optionsFor(a[3]));
      case "unlockBrowser": return mod.unlockBrowser(a[0], a[1] === null ? {} : { primaryPassword: a[1] });
      case "isBrowserUnlocked": return mod.isBrowserUnlocked(a[0]);
      case "forgetUnlockedKeys": return mod.forgetUnlockedKeys();
      case "writeImported": return mod.writeImported(a[0], a[1], a[2]);
      case "discardImported": return mod.discardImported(a[0]);
      case "listExtensions": return mod.listExtensions(a[0], a[1]);
      case "importSafariExport": return mod.importSafariExport(a[0], { signal: signalFor(a[1]) });
      case "safariHasFullDiskAccess": return mod.safariHasFullDiskAccess();
      case "openFullDiskAccessSettings": return mod.openFullDiskAccessSettings();
      case "importSafariDirect": return mod.importSafariDirect({ signal: signalFor(a[0]) });
      case "importBookmarksHTML": return mod.importBookmarksHTML(a[0]);
      case "importPasswordsCSV": return mod.importPasswordsCSV(a[0]);
      case "chooseImportFile": return mod.chooseImportFile(a[0]);
      case "readDiaTabs": return mod.readDiaTabs();
      default: return Promise.reject(new TypeError(`unknown export ${op}`));
    }
  };

  const out: unknown[] = [];
  for (const [op, ...args] of steps) {
    script = args.pop() as Script;
    calls = [];
    progress = [];
    chunks = [];
    controller = undefined;
    let result: unknown = null;
    let error: unknown = null;
    try {
      result = (await run(op, args)) ?? null;
    } catch (e) {
      const err = e as { name: string; code?: string; message: string };
      error = { name: err.name, code: err.code ?? null, message: err.message };
    }
    const abortListeners = controller ? getEventListeners((controller as AbortController).signal, "abort").length : 0;
    out.push({ result, error, calls, progress, chunks, listeners: listeners.size, abortListeners });
  }
  return out;
}
