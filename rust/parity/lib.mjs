// Shared pieces of the parity rig. Runs on the Mac, against the Ardia checkout named by ARDIA_DIR
// (its scripts/lib/instance.mjs launches and drives the hidden test instance).
import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import net from "node:net";
import { join } from "node:path";

export const ARDIA_DIR = process.env.ARDIA_DIR;
if (!ARDIA_DIR) throw new Error("set ARDIA_DIR to the Ardia checkout (the frozen commit, built per README.md)");
export const { launch, session, sleep } = await import(join(ARDIA_DIR, "scripts/lib/instance.mjs"));

export const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");

const PAGE_TITLES = {
  mail: "Inbox", calendar: "Calendar", docs: "Design doc", notes: "Meeting notes", reading: "Reading list",
  research1: "Swift concurrency", research2: "Metal shaders", work1: "Roadmap", work2: "Quarterly plan",
};
export const W = "w1";

// Same fixed seeded session as Ardia's scripts/agent/ui-snapshots, with static local pages: nothing reaches the network.
export function fixture(pagesDir) {
  mkdirSync(pagesDir, { recursive: true });
  for (const [name, title] of Object.entries(PAGE_TITLES))
    writeFileSync(join(pagesDir, `${name}.html`),
      `<!doctype html><html><head><meta charset="utf-8"><title>${title}</title></head><body style="font:16px -apple-system;margin:40px"><h1>${title}</h1><p>Find me: needle haystack needle.</p></body></html>`);
  const page = (name) => `file://${pagesDir}/${name}.html`;
  const tab = (id, name, extra = {}) => ({ id, url: page(name), title: PAGE_TITLES[name], lastActiveAt: 1, ...extra });
  return session({
    profiles: [{ id: "default", name: "Personal", color: "plum" }, { id: "work", name: "Work", color: "blue" }],
    settings: { searchSuggestions: false, appearance: "dark", restoreSession: true, cliControl: true },
    windows: [{
      id: W, frame: [80, 80, 1280, 800], createdAt: 1,
      tabIds: ["p1", "p2", "p3", "r1", "r2", "t1", "t2", "t3", "k1", "k2"],
      activeTabIds: { default: "t3", work: "k1" },
    }],
    tabs: [
      tab("p1", "mail", { pinned: true, pinnedUrl: page("mail") }),
      tab("p2", "calendar", { pinned: true, pinnedUrl: page("calendar") }),
      tab("p3", "docs", { pinned: true, pinnedUrl: page("docs") }),
      tab("r1", "research1"), tab("r2", "research2"), tab("t1", "notes"), tab("t2", "reading"),
      { id: "t3", url: "", title: "", lastActiveAt: 2 },
      tab("k1", "work1", { profileId: "work" }), tab("k2", "work2", { profileId: "work" }),
    ],
    groups: [{ id: "g1", windowId: W, profileId: "default", name: "Research", icon: null, color: "green", collapsed: false, pinned: false, tabIds: ["r1", "r2"], createdAt: 1 }],
  });
}

export const HELPERS = `
const st = () => ac.store.getState();
const W = ${JSON.stringify(W)};
const settle = (ms) => new Promise((r) => setTimeout(r, ms));
`;

// Puts the window back to the baseline every state and animation starts from.
export const RESET = `
const s = st();
const keep = ["p1", "p2", "p3", "r1", "r2", "t1", "t2", "t3", "k1", "k2"];
const extra = Object.keys(s.tabs).filter((id) => !keep.includes(id));
if (extra.length) s.closeTabs(extra);
if (s.windows[W].profileId !== "default") s.switchProfile(W, "default");
s.closePanel(W);
s.setDownloadsOpen(W, false);
for (const id of Object.keys(s.find)) s.setFind(id, { open: false });
s.setSelection(W, []);
s.updateGroup("g1", { collapsed: false });
for (const sp of Object.values(s.splits)) s.separateSplit(sp.id);
s.updateSettings({ appearance: "dark", tabLayout: "sidebar", addressBar: "toolbar", bookmarksBar: "never" });
if (st().windows[W].autoHideTabs ?? st().settings.autoHideTabs) st().toggleSidebar(W);
s.activate("t3");
s.updateProfile("default", { pinnedCollapsed: false });
return ac.shell.devSetPointer(W, 0, 0, false);
`;

export async function start(app, data, extra = {}) {
  const pagesDir = join(data, "pages");
  const inst = await launch(app, {
    data, onboarded: true, session: fixture(pagesDir), metroPort: process.env.METRO_PORT ?? 8081,
    switches: ["--proxy-server=http://127.0.0.1:9"],
    files: { "telemetry.json": { version: 1, sharing: false, decidedAt: 1, askDoneAt: 1, installId: null, crashCursor: 0, lastVersion: null, sessionOpen: false } },
    ...extra,
  });
  const run = (body, timeout = 20000) => inst.eval(`${HELPERS}\n${body}`, { timeout });
  await run(`return settle(1500).then(() => !!st().windows[W]);`);
  await run(`return settle(2500);`);
  return { inst, run };
}

// The main window's CGWindow id and bounds: the largest of the app's windows.
export function mainWindow(pid) {
  const list = JSON.parse(execFileSync("osascript", ["-l", "JavaScript", "-e",
    `ObjC.import("CoreGraphics"); JSON.stringify(ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionAll, 0)))
       .filter((w) => w.kCGWindowOwnerPID === ${pid} && w.kCGWindowLayer <= 0).map((w) => ({ id: w.kCGWindowNumber, layer: w.kCGWindowLayer, bounds: w.kCGWindowBounds })))`],
    { timeout: 20000 }).toString());
  list.sort((a, b) => b.bounds.Width * b.bounds.Height - a.bounds.Width * a.bounds.Height);
  if (!list.length) throw new Error(`no window for pid ${pid}`);
  return list[0];
}

// One newline-delimited JSON request per call on <data>/ArdiaControl.sock; every request and reply is appended to `log`.
export function controlSocket(data, log) {
  const sock = net.connect(join(data, "ArdiaControl.sock"));
  const waiting = new Map();
  let buf = "", next = 1;
  sock.on("data", (d) => {
    buf += d;
    for (let nl; (nl = buf.indexOf("\n")) >= 0; ) {
      const line = buf.slice(0, nl);
      buf = buf.slice(nl + 1);
      const msg = JSON.parse(line);
      log.push({ dir: "recv", at: new Date().toISOString(), msg });
      waiting.get(msg.id)?.(msg);
    }
  });
  const ready = new Promise((resolve, reject) => { sock.once("connect", resolve); sock.once("error", reject); });
  return {
    ready,
    call(method, params = {}) {
      const id = next++;
      const msg = { id, method, params };
      log.push({ dir: "send", at: new Date().toISOString(), msg });
      sock.write(JSON.stringify(msg) + "\n");
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error(`${method}: no reply in 10 s`)), 10000);
        waiting.set(id, (m) => { clearTimeout(timer); resolve(m); });
      });
    },
    close: () => sock.destroy(),
  };
}

// framegrab (tools/framegrab.swift): records a window's frames; "mark" stamps the trigger, "stop" writes frames.json.
export function frameGrab(bin, windowId, outDir, { fps = 60, max = 30 } = {}) {
  const child = spawn(bin, [String(windowId), outDir, "--fps", String(fps), "--max", String(max)], { stdio: ["pipe", "pipe", "inherit"] });
  let out = "";
  child.stdout.on("data", (d) => { out += d; });
  const exited = new Promise((resolve) => child.on("exit", resolve));
  return {
    async ready(ms = 15000) {
      for (const end = Date.now() + ms; Date.now() < end; await sleep(20)) if (out.includes("ready")) return;
      throw new Error(`framegrab not ready in ${ms} ms (output: ${out.trim()})`);
    },
    mark: () => child.stdin.write("mark\n"),
    async stop() {
      child.stdin.write("stop\n");
      const code = await exited;
      if (code !== 0) throw new Error(`framegrab exited ${code}: ${out.trim()}`);
    },
  };
}
