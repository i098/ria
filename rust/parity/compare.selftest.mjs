#!/usr/bin/env node
// Self-test of compare.mjs (slice 0.5): an equal pair passes, and a pair just above each threshold of check P fails.
//   node rust/parity/compare.selftest.mjs        (needs no Ardia and no Mac)
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, cpSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";
import { TOLERANCES, compareAnimation, compareStates } from "./compare.mjs";
import { compareTranscripts, normalize } from "./socket/compare.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const tmp = mkdtempSync(join(tmpdir(), "ria-compare-selftest-"));
const T = TOLERANCES;
let n = 0;
const check = (name, fn) => { fn(); console.log(`ok   ${name}`); n++; };

// 8-bit RGB PNG, 100x100; `pixel(i)` gives the grey level of pixel i.
const W = 100, H = 100;
function png(pixel) {
  const raw = Buffer.alloc((W * 3 + 1) * H);
  for (let y = 0; y < H; y++) for (let x = 0; x < W * 3; x++) raw[y * (W * 3 + 1) + 1 + x] = pixel(y * W + (x / 3 | 0));
  const crc = (buf) => { let c = ~0; for (const b of buf) { c ^= b; for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1)); } return ~c >>> 0; };
  const chunk = (type, data) => { const b = Buffer.concat([Buffer.from(type), data]), out = Buffer.alloc(b.length + 8); out.writeUInt32BE(data.length); b.copy(out, 4); out.writeUInt32BE(crc(b), b.length + 4); return out; };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(H, 4); ihdr[8] = 8; ihdr[9] = 2;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}
const geometry = (x) => ({ socketGeometry: { frame: [0, 0, 1200, 800] }, regionsInWindowPoints: { sidebar: [0, 0, 240 + x, 800] } });
function stateDir(root, { pixel = () => 100, geo = 0 } = {}) {
  const dir = join(root, "s");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "window.png"), png(pixel));
  writeFileSync(join(dir, "geometry.json"), JSON.stringify(geometry(geo)));
  return root;
}
const states = (cand) => { const c = stateDir(join(tmp, `c${n}`), cand); return compareStates(stateDir(join(tmp, "ref")), c)[0]; };

check("states: an equal pair passes", () => assert.equal(states({}).ok, true));
check("states: mean pixel delta above 2.0 fails", () => { const r = states({ pixel: () => 103 }); assert.ok(r.meanDelta > T.pixelMeanDelta); assert.equal(r.ok, false); });
check("states: differing pixels above 2 % fail", () => { const r = states({ pixel: (i) => (i % 100 < 3 ? 120 : 100) }); assert.ok(r.meanDelta < T.pixelMeanDelta && r.differingPixelShare > T.differingPixelShare); assert.equal(r.ok, false); });
check("states: geometry off by more than 1 pt fails", () => assert.equal(states({ geo: 1.5 }).ok, false));
check("states: geometry off by exactly 1 pt passes", () => assert.equal(states({ geo: 1 }).ok, true));
check("states: a state missing on the candidate side fails", () => {
  const empty = join(tmp, "empty"); mkdirSync(empty);
  assert.equal(compareStates(join(tmp, "ref"), empty)[0].ok, false);
});

// Animations: the real sidebar-open recording, then one changed number at a time.
const ref = JSON.parse(readFileSync(join(here, "captures/animations/sidebar-open.json"), "utf8"));
const variant = (edit) => { const d = structuredClone(ref); edit(d, d.runs.find((r) => r.run === d.reference)); return compareAnimation(ref, d); };
check("animation: an equal pair passes", () => assert.equal(compareAnimation(ref, structuredClone(ref)).ok, true));
check("animation: duration 34 ms off fails", () => { const r = variant((d, run) => { run.durationMs += T.durationMs + 0.6; }); assert.equal(r.ok, false); assert.ok(r.problems.some((p) => p.startsWith("duration"))); });
check("animation: duration 33 ms off passes that check", () => assert.ok(!variant((d, run) => { run.durationMs += T.durationMs - 0.4; }).problems.some((p) => p.startsWith("duration"))));
check("animation: t50 34 ms off fails", () => { const r = variant((d, run) => { run.t50Ms += T.t50Ms + 0.6; }); assert.equal(r.ok, false); assert.ok(r.problems.some((p) => p.startsWith("t50"))); });
check("animation: curve RMSE above 0.05 fails", () => {
  const r = variant((d, run) => { run.curve = run.curve.map((c) => ({ ...c, progress: Math.min(1, c.progress + 0.12) })); });
  assert.ok(r.curveRmse.value > T.curveRmse); assert.equal(r.ok, false);
});
check("animation: end signature delta above 4 fails", () => {
  const r = variant((d) => { d.endState.lastFrameSignature16x10Rgb = d.endState.lastFrameSignature16x10Rgb.map((v) => (v > 128 ? v - 6 : v + 6)); });
  assert.ok(r.endSignatureDelta.value > T.endSignatureDelta); assert.equal(r.ok, false);
});

// Socket transcript (check K): ids, times and data paths do not matter; any other change does.
const ex = (data, extra = {}) => normalize({ request: { method: "windows", params: {} }, response: { result: [{ id: "w1", at: 1, url: `file://${data}/p.html`, ...extra }] } }, data);
check("socket: ids, times and data-folder paths are ignored", () => assert.equal(compareTranscripts([ex("/tmp/a")], [ex("/tmp/b")]).ok, true));
check("socket: a changed value fails", () => assert.equal(compareTranscripts([ex("/tmp/a")], [ex("/tmp/b", { focused: true })]).ok, false));
check("socket: a missing exchange fails", () => assert.equal(compareTranscripts([ex("/tmp/a")], []).ok, false));

rmSync(tmp, { recursive: true });
console.log(`${n} checks passed`);
