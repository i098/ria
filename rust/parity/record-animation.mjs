#!/usr/bin/env node
// Records one Ardia animation: its frames with times, then the measured duration, curve samples and end state.
//   ARDIA_DIR=<ardia checkout> METRO_PORT=<port> node record-animation.mjs <Debug Ardia.app> <out dir> [--only a,b] [--runs N]
//        [--grab <framegrab binary>] [--frames <dir for the frame PNGs>]
// Writes <out>/<animation>.json and, in --frames, <animation>/run-N/frame-*.png + frames.json. Build framegrab with
// `swiftc -O tools/framegrab.swift -o framegrab`.
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { measure, round, summary } from "./analyze.mjs";
import { RESET, W, controlSocket, frameGrab, mainWindow, sha256, sleep, start } from "./lib.mjs";

// setup runs from the RESET baseline and settles; trigger is the one action whose animation is recorded.
export const ANIMATIONS = [
  { name: "sidebar-open", setup: `st().activate("t1"); st().toggleSidebar(W);`, trigger: `st().toggleSidebar(W);` },
  { name: "sidebar-close", setup: `st().activate("t1");`, trigger: `st().toggleSidebar(W);` },
  { name: "find-bar-open", setup: `st().activate("t1");`, trigger: `st().setFind("t1", { open: true, replace: false, query: "" });` },
  { name: "downloads-open", setup: `st().activate("t1");`, trigger: `st().setDownloadsOpen(W, true);` },
];

async function main() {
  const args = process.argv.slice(2);
  const flags = {}, positional = [];
  for (let i = 0; i < args.length; i++) args[i].startsWith("--") ? (flags[args[i].slice(2)] = args[++i]) : positional.push(args[i]);
  const [app, outArg] = positional;
  if (!app || !outArg) throw new Error("usage: record-animation.mjs <Debug Ardia.app> <out dir> [--only a,b] [--runs N] [--grab bin] [--frames dir]");
  const out = resolve(outArg);
  const framesRoot = resolve(flags.frames ?? join(out, "frames"));
  const grabBin = resolve(flags.grab ?? "framegrab");
  const runs = Number(flags.runs ?? 3);
  const only = flags.only ? new Set(flags.only.split(",")) : null;
  const animations = ANIMATIONS.filter((a) => !only || only.has(a.name));
  if (!animations.length) throw new Error(`no such animation: ${flags.only}`);
  mkdirSync(out, { recursive: true });
  const data = "/tmp/halyard-parity-data";
  rmSync(data, { recursive: true, force: true });
  const { inst, run } = await start(app, data);
  try {
    const win = mainWindow(inst.pid);
    for (const anim of animations) {
      const measured = [];
      let endState = null;
      for (let n = 1; n <= runs; n++) {
        await run(RESET);
        await run(`return settle(600);`);
        await run(anim.setup);
        await sleep(2000);
        const dir = join(framesRoot, anim.name, `run-${n}`);
        rmSync(dir, { recursive: true, force: true });
        const grab = frameGrab(grabBin, win.id, dir, { max: 20 });
        await grab.ready();
        await sleep(600);
        grab.mark();
        await run(anim.trigger);
        await sleep(1800);
        const log = [];
        const socket = controlSocket(data, log);
        await socket.ready;
        const geometry = (await socket.call("windows.geometry", { window: W })).result;
        const settings = (await socket.call("settings.get", { keys: ["sidebarWidth", "sidebarSide", "tabLayout"] })).result;
        const tabs = (await socket.call("tabs.list", { window: W })).result;
        socket.close();
        await grab.stop();
        const report = JSON.parse(readFileSync(join(dir, "frames.json"), "utf8"));
        const m = measure(report.frames, report.marks[0]);
        measured.push({ run: n, ...m, frameCount: report.frames.length, frames: join(anim.name, `run-${n}`) });
        endState = {
          windowGeometry: geometry, settings, activeTab: Array.isArray(tabs) ? tabs.find((t) => t.active)?.id ?? null : null,
          lastFrame: { file: report.frames.at(-1).file, sha256: sha256(join(dir, report.frames.at(-1).file)) },
          lastFrameSignature16x10Rgb: report.lastSignature,
        };
        console.log(`ok   ${anim.name} run ${n}: ${m.durationMs} ms, latency ${m.latencyMs} ms, t50 ${m.t50Ms} ms, ${m.framesInAnimation} frames`);
      }
      const doc = {
        animation: anim.name, trigger: anim.trigger, setup: anim.setup,
        capture: { tool: "framegrab (ScreenCaptureKit window stream)", requestedFps: 60, windowPoints: [win.bounds.Width, win.bounds.Height], clock: "CLOCK_UPTIME_RAW ms" },
        summary: { runs, durationMs: summary(measured.map((r) => r.durationMs)), t50Ms: summary(measured.map((r) => r.t50Ms)), latencyMs: summary(measured.map((r) => r.latencyMs)) },
        // The recording of record for comparison is the median-duration run.
        reference: measured.slice().sort((a, b) => a.durationMs - b.durationMs)[Math.floor(measured.length / 2)].run,
        runs: measured, endState,
      };
      writeFileSync(join(out, `${anim.name}.json`), JSON.stringify(doc, null, 2) + "\n");
    }
  } finally {
    await inst.quit().catch(() => {});
    rmSync(data, { recursive: true, force: true });
  }
}

await main();
