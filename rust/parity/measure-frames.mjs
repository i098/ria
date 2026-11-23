#!/usr/bin/env node
// Measures one framegrab recording of any app (Halyard included) into the same JSON compare.mjs reads for Ardia.
//   node measure-frames.mjs <frames dir written by framegrab> <animation name> <out.json>
// The window's end state is taken from the last frame; the window geometry of the end state is not known here, so it is left out.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { measure, summary } from "./analyze.mjs";

const [dirArg, name, out] = process.argv.slice(2);
if (!dirArg || !name || !out) {
  console.error("usage: measure-frames.mjs <frames dir> <animation name> <out.json>");
  process.exit(64);
}
const dir = resolve(dirArg);
const report = JSON.parse(readFileSync(join(dir, "frames.json"), "utf8"));
if (!report.marks.length) throw new Error("no trigger mark in frames.json: send `mark` to framegrab right before the trigger");
const m = measure(report.frames, report.marks[0]);
const lastFile = report.frames.at(-1).file;
const doc = {
  animation: name,
  capture: { tool: "framegrab (ScreenCaptureKit window stream)", requestedFps: report.fpsRequested, windowPixels: [report.width, report.height], clock: report.clock },
  summary: { runs: 1, durationMs: summary([m.durationMs]), t50Ms: summary([m.t50Ms]), latencyMs: summary([m.latencyMs]) },
  reference: 1,
  runs: [{ run: 1, ...m, frameCount: report.frames.length, frames: dir }],
  endState: { lastFrame: { file: lastFile, sha256: createHash("sha256").update(readFileSync(join(dir, lastFile))).digest("hex") }, lastFrameSignature16x10Rgb: report.lastSignature },
};
writeFileSync(out, JSON.stringify(doc, null, 2) + "\n");
console.log(`${name}: duration ${m.durationMs} ms, t10 ${m.t10Ms}, t50 ${m.t50Ms}, t90 ${m.t90Ms}, latency ${m.latencyMs} ms, ${m.framesInAnimation} frames`);
