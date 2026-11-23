#!/usr/bin/env node
// Captures reference states of Ardia: window PNG, geometry JSON and control-socket transcript for each.
//   ARDIA_DIR=<ardia checkout> METRO_PORT=<port> node capture-states.mjs <Debug Ardia.app> <out dir> [--only a,b] [--data DIR]
// Writes <out>/<state>/{window.png, geometry.json, socket.jsonl}, <out>/states.json and <out>/SHA256SUMS.
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { RESET, W, controlSocket, mainWindow, sha256, sleep, start } from "./lib.mjs";

export const STATES = [
  { name: "ntp-dark", body: `` },
  { name: "ntp-light", body: `st().updateSettings({ appearance: "light" });` },
  { name: "tab-dark", body: `st().activate("t1");` },
  { name: "tab-light", body: `st().updateSettings({ appearance: "light" }); st().activate("t1");` },
  { name: "pinned-active-dark", body: `st().activate("p2");` },
  { name: "group-collapsed-dark", body: `st().updateGroup("g1", { collapsed: true });` },
  { name: "sidebar-closed-dark", body: `st().activate("t1"); st().toggleSidebar(W);` },
  { name: "command-bar-dark", body: `st().activate("t1"); st().openPanel(W, "");` },
  { name: "downloads-dark", body: `st().activate("t1"); st().setDownloadsOpen(W, true);` },
  { name: "split-dark", body: `st().activate("t1"); st().createSplit(["t1", "t2"]);` },
];

// Frames of the sidebar and tab regions as the app measures them, in window points.
const REGIONS = `
const names = [...(globalThis.sidebarDrag ? globalThis.sidebarDrag.regions.keys() : [])];
return Promise.all(names.map((name) => new Promise((r) => {
  try { globalThis.sidebarDrag.regions.get(name).measureInWindow((x, y, w, h) => r([name, [x, y, w, h]])); } catch (e) { r([name, null]); }
}))).then((pairs) => Object.fromEntries(pairs));`;

async function main() {
  const args = process.argv.slice(2);
  const flags = {}, positional = [];
  for (let i = 0; i < args.length; i++) args[i].startsWith("--") ? (flags[args[i].slice(2)] = args[++i]) : positional.push(args[i]);
  const [app, outArg] = positional;
  if (!app || !outArg) throw new Error("usage: capture-states.mjs <Debug Ardia.app> <out dir> [--only a,b] [--data DIR]");
  const out = resolve(outArg);
  const only = flags.only ? new Set(flags.only.split(",")) : null;
  const states = STATES.filter((s) => !only || only.has(s.name));
  if (!states.length) throw new Error(`no such state: ${flags.only}`);
  const data = resolve(flags.data ?? "/tmp/ria-parity-data"); // a fixed path: it is inside the page URLs
  rmSync(data, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  const { inst, run } = await start(app, data);
  const results = [];
  try {
    for (const state of states) {
      const dir = join(out, state.name);
      rmSync(dir, { recursive: true, force: true });
      mkdirSync(dir, { recursive: true });
      await run(RESET);
      await run(`return settle(400);`);
      const setup = await run(state.body || "return null;");
      await sleep(1500);
      const log = [];
      const socket = controlSocket(data, log);
      await socket.ready;
      await socket.call("hello", {});
      const windows = await socket.call("windows", {});
      const geometry = await socket.call("windows.geometry", { window: W });
      await socket.call("workspace", {});
      await socket.call("tabs.list", { window: W });
      await socket.call("settings.get", { keys: ["appearance", "tabLayout", "addressBar", "bookmarksBar", "sidebarWidth", "sidebarSide"] });
      socket.close();
      const win = mainWindow(inst.pid);
      const png = join(dir, "window.png");
      execFileSync("screencapture", ["-l", String(win.id), "-o", "-x", png], { timeout: 20000 });
      const regions = await run(REGIONS).catch((e) => ({ error: String(e.message ?? e) }));
      writeFileSync(join(dir, "geometry.json"), JSON.stringify({
        state: state.name, cgWindow: { id: win.id, bounds: win.bounds }, socketGeometry: geometry.result ?? geometry.error,
        socketWindows: windows.result ?? windows.error, regionsInWindowPoints: regions,
      }, null, 2) + "\n");
      writeFileSync(join(dir, "socket.jsonl"), log.map((l) => JSON.stringify(l)).join("\n") + "\n");
      results.push({ name: state.name, png: sha256(png), setup: setup ?? null });
      console.log(`ok   ${state.name} ${statSync(png).size} bytes`);
    }
  } finally {
    await inst.quit().catch(() => {});
    rmSync(data, { recursive: true, force: true });
  }
  writeFileSync(join(out, "states.json"), JSON.stringify(results, null, 2) + "\n");
  const sums = [];
  for (const state of readdirSync(out, { withFileTypes: true }).filter((d) => d.isDirectory()))
    for (const f of readdirSync(join(out, state.name))) sums.push(`${sha256(join(out, state.name, f))}  ${state.name}/${f}`);
  writeFileSync(join(out, "SHA256SUMS"), sums.sort((a, b) => a.split("  ")[1].localeCompare(b.split("  ")[1])).join("\n") + "\n");
}

await main();
