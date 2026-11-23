#!/usr/bin/env node
// Check K: sends requests.jsonl to Ardia and to Ria over their control sockets and compares the two transcripts.
//   node compare.mjs live --ardia <data dir or socket> --ria <data dir or socket> [--requests FILE] [--write DIR]
//   node compare.mjs files <ardia transcript.jsonl> <ria transcript.jsonl>
// A transcript is one JSON object per line: {"request": {...}, "response": {...}} with the request id removed.
// The `files` mode also reads the `socket.jsonl` files that capture-states.mjs writes (lines {dir, at, msg}).
// Ids, times and data-folder paths are removed before the compare. Exit 1 when the transcripts differ.
import { readFileSync, writeFileSync, mkdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import net from "node:net";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const JSONL = (text) => text.split("\n").filter(Boolean).map((l) => JSON.parse(l));

// Keys whose value changes between runs. `id` covers window, tab, group and request ids; ids are not part of the contract.
const VOLATILE = new Set(["id", "at", "time", "targetId", "pageGeneration", "generation", "lastSyncedAt", "pid"]);
const TIME_KEY = /(At|Time)$/;

// Removes volatile keys and rewrites the data-folder path to "<data>"; `data` is the folder of the transcript.
export function normalize(value, data) {
  if (Array.isArray(value)) return value.map((v) => normalize(v, data));
  if (value && typeof value === "object")
    return Object.fromEntries(Object.entries(value).filter(([k]) => !VOLATILE.has(k) && !TIME_KEY.test(k)).map(([k, v]) => [k, normalize(v, data)]));
  if (typeof value === "string" && data) return value.split(data).join("<data>");
  return value;
}

// Pairs each request with its response by id.
export function pairUp(lines, data) {
  const sent = new Map(), out = [];
  for (const { msg } of lines) {
    if (msg.method !== undefined) sent.set(msg.id, msg);
    else if (sent.has(msg.id)) {
      const { method, params } = sent.get(msg.id);
      const { id, ...reply } = msg;
      out.push(normalize({ request: { method, params }, response: reply }, data));
    }
  }
  return out;
}

export function compareTranscripts(a, b) {
  const problems = [];
  if (a.length !== b.length) problems.push(`${a.length} exchanges vs ${b.length}`);
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    const x = JSON.stringify(a[i]), y = JSON.stringify(b[i]);
    if (x !== y) problems.push(`#${i + 1} ${a[i].request.method}: Ardia ${x.slice(0, 200)} | Ria ${y.slice(0, 200)}`);
  }
  return { ok: problems.length === 0, exchanges: Math.min(a.length, b.length), problems };
}

const socketPath = (p) => (statSync(p).isDirectory() ? join(p, "ArdiaControl.sock") : p);

// Sends each request in order on one connection; returns the raw send/recv lines.
export async function runRequests(path, requests) {
  const sock = net.connect(socketPath(path));
  await new Promise((resolve, reject) => { sock.once("connect", resolve); sock.once("error", reject); });
  const waiting = new Map(), lines = [];
  let buf = "";
  sock.on("data", (d) => {
    buf += d;
    for (let nl; (nl = buf.indexOf("\n")) >= 0; ) {
      const msg = JSON.parse(buf.slice(0, nl));
      buf = buf.slice(nl + 1);
      lines.push({ msg });
      waiting.get(msg.id)?.();
    }
  });
  try {
    for (const [i, r] of requests.entries()) {
      const msg = { id: i + 1, ...r };
      lines.push({ msg });
      const reply = new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error(`${r.method}: no reply in 10 s`)), 10000);
        waiting.set(msg.id, () => { clearTimeout(timer); resolve(); });
      });
      sock.write(JSON.stringify(msg) + "\n");
      await reply;
    }
  } finally { sock.destroy(); }
  return lines;
}

const dataOf = (p) => (statSync(p).isDirectory() ? p : dirname(p));

async function main() {
  const [mode, ...rest] = process.argv.slice(2);
  const flags = {}, positional = [];
  for (let i = 0; i < rest.length; i++) rest[i].startsWith("--") ? (flags[rest[i].slice(2)] = rest[++i]) : positional.push(rest[i]);
  let a, b;
  if (mode === "files" && positional.length === 2) {
    const read = (p) => JSONL(readFileSync(p, "utf8"));
    const [x, y] = positional.map(read);
    // Capture files and transcripts both work; a transcript line has `request`.
    const asPairs = (lines) => (lines[0]?.request ? lines : pairUp(lines, flags.data));
    a = asPairs(x); b = asPairs(y);
  } else if (mode === "live" && flags.ardia && flags.ria) {
    const requests = JSONL(readFileSync(flags.requests ?? join(here, "requests.jsonl"), "utf8"));
    const [la, lb] = await Promise.all([runRequests(flags.ardia, requests), runRequests(flags.ria, requests)]);
    a = pairUp(la, dataOf(flags.ardia)); b = pairUp(lb, dataOf(flags.ria));
    if (flags.write) {
      mkdirSync(flags.write, { recursive: true });
      for (const [name, t] of [["ardia", a], ["ria", b]]) writeFileSync(join(flags.write, `${name}.jsonl`), t.map((e) => JSON.stringify(e)).join("\n") + "\n");
    }
  } else {
    console.error("usage: compare.mjs live --ardia <data dir|socket> --ria <data dir|socket> [--requests FILE] [--write DIR] | compare.mjs files <ardia.jsonl> <ria.jsonl>");
    process.exit(64);
  }
  const r = compareTranscripts(a, b);
  console.log(`${r.ok ? "ok  " : "FAIL"} socket transcript: ${r.exchanges} exchanges${r.problems.length ? `\n${r.problems.join("\n")}` : ""}`);
  process.exit(r.ok ? 0 : 1);
}

if (import.meta.url === `file://${process.argv[1]}`) await main();
