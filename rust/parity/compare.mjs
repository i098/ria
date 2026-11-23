#!/usr/bin/env node
// Compares a later capture with the Ardia reference, with numbers. No Ardia needed: it reads the files the rig wrote.
//   node compare.mjs states <reference states dir> <candidate states dir> [--pixel-tolerance N] [--geometry-tolerance PT]
//   node compare.mjs animation <reference animation.json> <candidate animation.json> [--duration-tolerance MS] [--curve-tolerance P]
// Exit 1 when any number is outside its tolerance. Defaults are in TOLERANCES and in the README.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { inflateSync } from "node:zlib";

// Frame period at 60 fps is 16.7 ms; the recorder resolves time to one frame, so two frames is the least that is meaningful.
export const TOLERANCES = { pixelMeanDelta: 2.0, differingPixelShare: 0.02, geometryPoints: 1, durationMs: 33.4, t50Ms: 33.4, curveRmse: 0.05, endSignatureDelta: 4 };

export function readPng(path) {
  const buf = readFileSync(path);
  let width = 0, height = 0, channels = 4;
  const idat = [];
  for (let at = 8; at < buf.length; ) {
    const length = buf.readUInt32BE(at), type = buf.toString("ascii", at + 4, at + 8);
    if (type === "IHDR") {
      width = buf.readUInt32BE(at + 8);
      height = buf.readUInt32BE(at + 12);
      if (buf[at + 16] !== 8 || buf[at + 20] !== 0) throw new Error(`${path}: only 8-bit non-interlaced PNGs`);
      channels = { 0: 1, 2: 3, 4: 2, 6: 4 }[buf[at + 17]];
    } else if (type === "IDAT") idat.push(buf.subarray(at + 8, at + 8 + length));
    at += 12 + length;
  }
  const raw = inflateSync(Buffer.concat(idat)), stride = width * channels, out = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)], line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)), at = y * stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? out[at + x - channels] : 0, b = y ? out[at - stride + x] : 0, c = x >= channels && y ? out[at - stride + x - channels] : 0;
      let v = line[x];
      if (filter === 1) v += a;
      else if (filter === 2) v += b;
      else if (filter === 3) v += (a + b) >> 1;
      else if (filter === 4) { const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c); v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      out[at + x] = v & 255;
    }
  }
  return { width, height, channels, data: out };
}

// Mean absolute RGB delta (0-255) over all pixels, and the share of pixels with any channel off by more than 8.
export function diffImages(a, b) {
  if (a.width !== b.width || a.height !== b.height) return { sizeMismatch: [`${a.width}x${a.height}`, `${b.width}x${b.height}`] };
  let sum = 0, over = 0;
  for (let p = 0; p < a.width * a.height; p++) {
    let big = false;
    for (let c = 0; c < 3; c++) {
      const d = Math.abs(a.data[p * a.channels + c] - b.data[p * b.channels + c]);
      sum += d;
      if (d > 8) big = true;
    }
    if (big) over++;
  }
  const n = a.width * a.height;
  return { meanDelta: sum / (n * 3), differingPixelShare: over / n };
}

// Every number leaf of a JSON value as path -> value.
export function numbers(value, path = "", out = new Map()) {
  if (typeof value === "number") out.set(path, value);
  else if (Array.isArray(value)) value.forEach((v, i) => numbers(v, `${path}[${i}]`, out));
  else if (value && typeof value === "object") for (const [k, v] of Object.entries(value)) numbers(v, path ? `${path}.${k}` : k, out);
  return out;
}

// Geometry: the window frame and every measured region, in points. Page URLs, ids and timestamps are not compared.
const geometryNumbers = (g) => numbers({ frame: g.socketGeometry?.frame, bounds: g.cgWindow?.bounds, regions: g.regionsInWindowPoints });

export function compareStates(refDir, candDir, tol = TOLERANCES) {
  const names = (dir) => readdirSync(dir, { withFileTypes: true }).filter((d) => d.isDirectory()).map((d) => d.name);
  const rows = [];
  for (const name of [...new Set([...names(refDir), ...names(candDir)])].sort()) {
    const row = { state: name, ok: true, problems: [] };
    rows.push(row);
    const ref = join(refDir, name), cand = join(candDir, name);
    if (!existsSync(join(ref, "window.png")) || !existsSync(join(cand, "window.png"))) {
      row.ok = false;
      row.problems.push(`missing on the ${existsSync(join(ref, "window.png")) ? "candidate" : "reference"} side`);
      continue;
    }
    const d = diffImages(readPng(join(ref, "window.png")), readPng(join(cand, "window.png")));
    Object.assign(row, d);
    if (d.sizeMismatch) row.problems.push(`window size ${d.sizeMismatch[0]} vs ${d.sizeMismatch[1]}`);
    else {
      if (d.meanDelta > tol.pixelMeanDelta) row.problems.push(`mean pixel delta ${d.meanDelta.toFixed(2)} > ${tol.pixelMeanDelta}`);
      if (d.differingPixelShare > tol.differingPixelShare) row.problems.push(`${(d.differingPixelShare * 100).toFixed(2)}% of pixels differ > ${tol.differingPixelShare * 100}%`);
    }
    const ga = geometryNumbers(JSON.parse(readFileSync(join(ref, "geometry.json"), "utf8")));
    const gb = geometryNumbers(JSON.parse(readFileSync(join(cand, "geometry.json"), "utf8")));
    let worst = 0;
    for (const [path, v] of ga) {
      if (!gb.has(path)) row.problems.push(`geometry ${path} missing in the candidate`);
      else worst = Math.max(worst, Math.abs(v - gb.get(path)));
    }
    row.geometryMaxDeltaPoints = worst;
    if (worst > tol.geometryPoints) row.problems.push(`geometry differs by ${worst} pt > ${tol.geometryPoints}`);
    row.ok = row.problems.length === 0;
  }
  return rows;
}

const referenceRun = (doc) => doc.runs.find((r) => r.run === doc.reference);

// Resamples a progress curve (time -> 0..1) at `step` ms; linear between frames, 0 before the first and 1 after the last.
export function sampleCurve(curve, endMs, step = 8.35) {
  const out = [];
  for (let t = 0; t <= endMs; t += step) {
    const i = curve.findIndex((c) => c.tMs >= t);
    out.push(i < 0 ? 1 : i === 0 ? curve[0].progress : curve[i - 1].progress + ((t - curve[i - 1].tMs) / (curve[i].tMs - curve[i - 1].tMs || 1)) * (curve[i].progress - curve[i - 1].progress));
  }
  return out;
}

export function compareAnimation(refDoc, candDoc, tol = TOLERANCES) {
  const a = referenceRun(refDoc), b = referenceRun(candDoc);
  const end = Math.max(a.durationMs, b.durationMs);
  const sa = sampleCurve(a.curve, end), sb = sampleCurve(b.curve, end);
  const rmse = Math.sqrt(sa.reduce((s, v, i) => s + (v - sb[i]) ** 2, 0) / sa.length);
  const sigA = refDoc.endState?.lastFrameSignature16x10Rgb, sigB = candDoc.endState?.lastFrameSignature16x10Rgb;
  const endDelta = sigA && sigB && sigA.length === sigB.length ? sigA.reduce((s, v, i) => s + Math.abs(v - sigB[i]), 0) / sigA.length : null;
  const rows = {
    durationMs: { reference: a.durationMs, candidate: b.durationMs, delta: Math.round((b.durationMs - a.durationMs) * 10) / 10, tolerance: tol.durationMs },
    t50Ms: { reference: a.t50Ms, candidate: b.t50Ms, delta: Math.round((b.t50Ms - a.t50Ms) * 10) / 10, tolerance: tol.t50Ms },
    curveRmse: { value: Math.round(rmse * 1e4) / 1e4, tolerance: tol.curveRmse },
    endSignatureDelta: { value: endDelta === null ? null : Math.round(endDelta * 100) / 100, tolerance: tol.endSignatureDelta },
  };
  const problems = [];
  if (Math.abs(rows.durationMs.delta) > tol.durationMs) problems.push(`duration differs by ${rows.durationMs.delta} ms`);
  if (Math.abs(rows.t50Ms.delta) > tol.t50Ms) problems.push(`t50 differs by ${rows.t50Ms.delta} ms`);
  if (rmse > tol.curveRmse) problems.push(`curve RMSE ${rows.curveRmse.value} > ${tol.curveRmse}`);
  if (endDelta === null) problems.push("end signature missing or sized differently");
  else if (endDelta > tol.endSignatureDelta) problems.push(`end state differs by ${rows.endSignatureDelta.value} > ${tol.endSignatureDelta}`);
  return { animation: refDoc.animation, ok: problems.length === 0, problems, ...rows };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const args = process.argv.slice(2);
  const flags = {}, positional = [];
  for (let i = 1; i < args.length; i++) args[i].startsWith("--") ? (flags[args[i].slice(2)] = Number(args[++i])) : positional.push(args[i]);
  const tol = { ...TOLERANCES, ...(flags["pixel-tolerance"] !== undefined && { pixelMeanDelta: flags["pixel-tolerance"] }), ...(flags["geometry-tolerance"] !== undefined && { geometryPoints: flags["geometry-tolerance"] }),
    ...(flags["duration-tolerance"] !== undefined && { durationMs: flags["duration-tolerance"], t50Ms: flags["duration-tolerance"] }), ...(flags["curve-tolerance"] !== undefined && { curveRmse: flags["curve-tolerance"] }) };
  let bad = 0;
  if (args[0] === "states" && positional.length === 2) {
    for (const r of compareStates(positional[0], positional[1], tol)) {
      if (!r.ok) bad++;
      console.log(`${r.ok ? "ok  " : "FAIL"} ${r.state}: mean delta ${r.meanDelta?.toFixed(3) ?? "-"}, differing ${r.differingPixelShare === undefined ? "-" : (r.differingPixelShare * 100).toFixed(3) + "%"}, geometry max ${r.geometryMaxDeltaPoints ?? "-"} pt${r.problems.length ? ` (${r.problems.join("; ")})` : ""}`);
    }
  } else if (args[0] === "animation" && positional.length === 2) {
    const r = compareAnimation(JSON.parse(readFileSync(positional[0], "utf8")), JSON.parse(readFileSync(positional[1], "utf8")), tol);
    if (!r.ok) bad++;
    console.log(JSON.stringify(r, null, 2));
  } else {
    console.error("usage: compare.mjs states <ref dir> <candidate dir> | compare.mjs animation <ref.json> <candidate.json>");
    process.exit(64);
  }
  process.exit(bad ? 1 : 0);
}
