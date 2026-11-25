// Ria original, no Ardia source.

// Check S for slice 2.14 (state/splits): records the real Ardia split store, then replays each case on `ria-state`.
//
//   node record-splits.mjs ARDIA_SCRATCH OUT_DIR RUST_BIN
//
// RUST_BIN is the built `ria-replay-splits`. A case that reaches a sibling `todo!()` is pending: it is left out of
// `OUT_DIR/cases.jsonl` and listed in `OUT_DIR/pending.json` (see PENDING.md). The other cases go to `rust-anything diff`.

import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadArdia } from "../kit/load-ardia.mjs";
import { nativeFake, replayDriver } from "../kit/replay.mjs";

const kit = join(dirname(fileURLToPath(import.meta.url)), "..", "kit");

// A tiny deterministic generator, so the recording is the same on each run.
let seed = 14;
const rnd = (n) => (seed = (seed * 1103515245 + 12345) % 2147483648) % n;

const tab = (id, { window = "w1", profile = "p", pin = null, at = 0, url = `https://${id}.test/` } = {}) => ({
  id, windowId: window, profileId: profile, url, title: id, favicon: null, pin, muted: false, zoom: 1, customTitle: null,
  customIcon: null, pinnedUrl: null, page: { kind: "idle" }, openerId: null, createdAt: 0, lastActiveAt: at,
});
const win = (id, tabIds, active, profile = "p") => ({
  kind: "main", id, profileId: profile, tabIds, activeTabIds: active ? { [profile]: active } : {}, frame: null, createdAt: 0,
});
const plain = (tabId, size) => ({ size, tabId });
const stack = (a, b, size) => ({ size, stack: [a, b], stackSizes: [0.5, 0.5] });
const split = (id, orientation, slots, window = "w1") => ({ id, windowId: window, orientation, slots });
const row = { as: "row" };
const tile = { as: "tile" };

const world = (tabs, windows, splits = [], groups = []) => ({
  type: "seed",
  tabs: Object.fromEntries(tabs.map((t) => [t.id, t])),
  windows: Object.fromEntries(windows.map((w) => [w.id, w])),
  groups: Object.fromEntries(groups.map((g) => [g.id, g])),
  splits: Object.fromEntries(splits.map((v) => [v.id, v])),
});

const ids = (n, p = "t") => Array.from({ length: n }, (_, i) => `${p}${i + 1}`);

// ---- action sequences -------------------------------------------------------------------------------------------
function sequences() {
  const out = [];
  const t = ids(5).map((id, i) => tab(id, { at: i }));
  const base = (splits) => world(t, [win("w1", ids(5), "t1")], splits);
  const two = split("s1", "horizontal", [plain("t1", 0.5), plain("t2", 0.5)]);
  const three = split("s1", "horizontal", [plain("t1", 0.2), plain("t2", 0.3), plain("t3", 0.5)]);
  const stacked = split("s1", "horizontal", [plain("t1", 0.5), stack("t2", "t3", 0.5)]);

  // Store methods that only change `splits`.
  for (const v of [two, three, stacked, split("s1", "vertical", three.slots)]) {
    out.push([base([v]), { type: "flipSplit", splitId: "s1" }, { type: "flipSplit", splitId: "nope" }]);
    out.push([base([v]), { type: "toggleSplitOrientation", splitId: "s1" }, { type: "toggleSplitOrientation", splitId: "s1" }]);
    out.push([base([v]), { type: "separateSplit", splitId: "s1" }, { type: "separateSplit", splitId: "s1" }]);
    for (const tabId of ["t1", "t2", "t3", "t4", "nope"]) {
      for (const delta of [-1, 1]) out.push([base([v]), { type: "movePane", tabId, delta }, { type: "movePane", tabId, delta }]);
    }
    const n = v.slots.length;
    out.push([base([v]), { type: "updateSplit", id: "s1", patch: { sizes: Array.from({ length: n }, (_, i) => (i + 1) / (n * (n + 1) / 2)) } }]);
    out.push([base([v]), { type: "updateSplit", id: "s1", patch: { sizes: [0.9] } }, { type: "updateSplit", id: "nope", patch: { sizes: [0.5, 0.5] } }]);
    out.push([base([v]), { type: "updateSplit", id: "s1", patch: { stackSizes: [0.25, 0.75] } }]);
    out.push([base([v]), { type: "updateSplit", id: "s1", patch: { sizes: Array.from({ length: n }, () => 1 / n), stackSizes: [0.7, 0.3] } }]);
  }

  // removeTabFromSplit: a tab of a 2-pane split ends the split; a tab that is not in a split does nothing.
  out.push([base([two]), { type: "removeTabFromSplit", tabId: "t1" }]);
  out.push([base([two]), { type: "removeTabFromSplit", tabId: "t2" }]);
  out.push([base([two]), { type: "removeTabFromSplit", tabId: "t4" }, { type: "removeTabFromSplit", tabId: "nope" }]);
  out.push([base([stacked]), { type: "removeTabFromSplit", tabId: "t2" }]);

  // createSplit that is refused.
  const mixed = world([tab("a1"), tab("a2", { profile: "q" }), tab("b1", { window: "w2" }), tab("a3")], [win("w1", ["a1", "a2", "a3"], "a1"), win("w2", ["b1"], "b1")]);
  out.push([mixed, { type: "createSplit", tabIds: ["a1"] }, { type: "createSplit", tabIds: ["a1", "a1"] }, { type: "createSplit", tabIds: ["a1", "a2"] },
    { type: "createSplit", tabIds: ["a1", "b1"] }, { type: "createSplit", tabIds: ["a1", "nope"] }, { type: "createSplit", tabIds: [] },
    { type: "createSplit", tabIds: ["t1", "t2", "t3", "t4"], orientation: "vertical" }]);

  // openSplitPane that is refused: no window, no anchor, bad tab, other window or profile.
  out.push([mixed, { type: "openSplitPane", windowId: "nope" }, { type: "openSplitPane", windowId: "w2", options: { anchorTabId: "a1" } },
    { type: "openSplitPane", windowId: "w1", options: { anchorTabId: "nope" } }, { type: "openSplitPane", windowId: "w1", options: { tabId: "a1", anchorTabId: "a1" } },
    { type: "openSplitPane", windowId: "w1", options: { tabId: "b1", anchorTabId: "a1" } }, { type: "openSplitPane", windowId: "w1", options: { tabId: "a2", anchorTabId: "a1" } },
    { type: "openSplitPane", windowId: "w1", options: { tabId: "nope", anchorTabId: "a1" } }]);

  // Focus, link, drop and replace calls that stop early.
  out.push([base([two]), { type: "focusPane", windowId: "nope", delta: 1 }, { type: "focusPane", windowId: "w1", delta: 1 }]);
  out.push([world(t, [win("w1", ids(5), "t4")], [two]), { type: "focusPane", windowId: "w1", delta: 1 }, { type: "focusSplitPane", windowId: "w1", delta: -1 }]);
  out.push([world(t, [win("w1", ids(5), null)], [two]), { type: "focusPane", windowId: "w1", delta: 1 }]);
  out.push([base([two]), { type: "openInSplit", tabId: "nope" }, { type: "openInSplit", tabId: "t1" }]);
  out.push([base([two]), { type: "openLinkInSplit", sourceTabId: "nope", url: "https://x.test/" }]);
  out.push([base([two]), { type: "dropTabIntoSplit", tabId: "t1", targetTabId: "t1", side: "left" }, { type: "dropTabIntoSplit", tabId: "t3", targetTabId: "nope", side: "top" }]);
  const pins = world([tab("r1", { pin: row }), tab("r2", { pin: row }), tab("r3", { pin: row, window: "w2" }), tab("k1", { pin: tile }), tab("l1")], [win("w1", ["k1", "r1", "r2", "l1"], "r1"), win("w2", ["r3"], "r3")],
    [split("s1", "horizontal", [plain("r1", 0.5), plain("r2", 0.5)])]);
  out.push([pins, { type: "dropTabOnRow", tabId: "nope", rowTabId: "r1", side: "left" }, { type: "dropTabOnRow", tabId: "r1", rowTabId: "r1", side: "left" },
    { type: "dropTabOnRow", tabId: "r3", rowTabId: "r1", side: "right" }, { type: "dropTabOnRow", tabId: "k1", rowTabId: "r1", side: "left" },
    { type: "dropTabOnRow", tabId: "r2", rowTabId: "r1", side: "left" }, { type: "dropTabOnRow", tabId: "l1", rowTabId: "k1", side: "right" }]);
  out.push([base([two]), { type: "replaceSplitPane", paneTabId: "t4", tabId: "t5" }, { type: "replaceSplitPane", paneTabId: "t1", tabId: "t2" },
    { type: "replaceSplitPane", paneTabId: "t1", tabId: "nope" }]);
  out.push([base([two]), { type: "closeSplitView", splitId: "nope" }, { type: "toggleTabLayout", windowId: "nope" }]);

  // Sequences that reach a sibling: they wait for it (PENDING.md).
  out.push([base([]), { type: "createSplit", tabIds: ["t1", "t2"] }]);
  out.push([base([]), { type: "createSplit", tabIds: ["t3", "t1", "t2"], orientation: "vertical" }]);
  out.push([base([two]), { type: "openSplitPane", windowId: "w1", options: { anchorTabId: "t1", tabId: "t3", side: "bottom" } }]);
  out.push([base([three]), { type: "openSplitPane", windowId: "w1", options: { anchorTabId: "t1", tabId: "t4" } }]);
  out.push([base([two]), { type: "openSplitPane", windowId: "w1", options: { url: "https://n.test/", background: true } }]);
  out.push([base([two]), { type: "openSplitPane", windowId: "w1", options: { anchorTabId: "t1", tabId: "t3", inPlaceOf: "t2" } }]);
  out.push([base([three]), { type: "removeTabFromSplit", tabId: "t2" }]);
  out.push([base([two]), { type: "replaceSplitPane", paneTabId: "t1", tabId: "t3" }]);
  out.push([base([two]), { type: "focusPane", windowId: "w1", delta: 1 }]);
  out.push([base([two]), { type: "openInSplit", tabId: "t3", side: "left" }]);
  out.push([base([two]), { type: "openLinkInSplit", sourceTabId: "t1", url: "https://l.test/" }]);
  out.push([base([two]), { type: "dropTabIntoSplit", tabId: "t3", targetTabId: "t1", side: "right" }]);
  out.push([base([two]), { type: "toggleTabLayout", windowId: "w1" }]);
  return out;
}

// ---- pure helper calls ------------------------------------------------------------------------------------------
function randomSplit(n, tabs) {
  const slots = [];
  let used = 0;
  while (used < n) {
    const stacked = n - used >= 2 && rnd(3) === 0;
    const size = [0, 0.2, 0.25, 0.5, 1, 1 / 3][rnd(6)];
    slots.push(stacked ? stack(tabs[used], tabs[used + 1], size) : plain(tabs[used], size));
    used += stacked ? 2 : 1;
  }
  return split("s", ["horizontal", "vertical"][rnd(2)], slots);
}

function calls() {
  const out = [];
  const pool = ids(7);
  for (let i = 0; i < 40; i++) {
    const n = 2 + rnd(2);
    const s = randomSplit(n, pool.slice(0, n));
    const gone = pool.filter(() => rnd(3) === 0);
    out.push({ type: "call", fn: "removeFromSplits", splits: { s, o: split("o", "vertical", [plain("t6", 0.5), plain("t7", 0.5)]) }, gone });
    out.push({ type: "call", fn: "stackAt", split: s });
    out.push({ type: "call", fn: "splitTabIds", split: s });
    out.push({ type: "call", fn: "isStacked", split: s, tabId: pool[rnd(4)] });
    out.push({ type: "call", fn: "withPaneIds", split: s, ids: [...pool.slice(0, n)].reverse() });
    out.push({ type: "call", fn: "paneChunks", ids: pool.slice(0, rnd(8)) });
    out.push({ type: "call", fn: "evenSlots", ids: pool.slice(0, 1 + rnd(4)) });
    out.push({ type: "call", fn: "splitOf", splits: { s }, tabId: [...pool, "", null][rnd(9)] });
  }
  for (let i = 0; i < 30; i++) {
    const n = 2 + rnd(2);
    const names = pool.slice(0, n);
    const pinOf = () => [null, null, row, tile][rnd(4)];
    const tabs = Object.fromEntries(pool.concat("x1").filter((id) => id !== "t7" || rnd(2)).map((id) => [id, tab(id, { pin: pinOf(), window: rnd(5) ? "w1" : "w2", profile: rnd(5) ? "p" : "q" })]));
    const s = randomSplit(n, names);
    out.push({ type: "call", fn: "splitRowPanes", tabs, split: rnd(5) ? s : null });
    out.push({ type: "call", fn: "withSplitRows", tabs, splits: { s }, ids: [names[0], "x1"] });
    out.push({ type: "call", fn: "splitRowHead", tabs, splits: { s }, order: [...pool].reverse(), id: pool[rnd(4)] });
    out.push({ type: "call", fn: "sanitizeSplits", splits: { s, u: split("u", "horizontal", [plain(names[0], 0.5), plain("t5", 0.5)]), v: split("v", "horizontal", ["t5", "t6", "t7", "x1"].map((x) => plain(x, 0.25))) }, tabs });
    out.push({ type: "call", fn: "splitsOnRow", tabs, tabId: pool[rnd(7)], rowTabId: pool[rnd(7)] });
    out.push({ type: "call", fn: "crossesLane", anchor: pinOf(), tab: pinOf() });
    out.push({ type: "call", fn: "splitLane", pin: pinOf() });
  }
  out.push({ type: "call", fn: "splitsWhereUnavailable", splits: {}, available: false });
  out.push({ type: "call", fn: "splitsWhereUnavailable", splits: { s: randomSplit(2, ["t1", "t2"]) }, available: false });
  out.push({ type: "call", fn: "splitsWhereUnavailable", splits: { s: randomSplit(2, ["t1", "t2"]) }, available: true });
  return out;
}

// ---- Ardia driver -----------------------------------------------------------------------------------------------
async function ardiaReplay(scratch) {
  const load = await loadArdia(scratch);
  const [actions, lib, lanes, { useBrowser }] = await Promise.all([
    load("state/splits/actions.ts"), load("state/splits/splits.ts"), load("state/splits/lanes.ts"), load("state/browser/store.ts"),
  ]);
  const S = () => useBrowser.getState();
  S().hydrate({});
  const pure = {
    removeFromSplits: (a) => lib.removeFromSplits(a.splits, new Set(a.gone)),
    sanitizeSplits: (a) => lib.sanitizeSplits(a.splits, a.tabs),
    splitRowPanes: (a) => lib.splitRowPanes({ tabs: a.tabs }, a.split ?? undefined),
    withSplitRows: (a) => lib.withSplitRows({ tabs: a.tabs, splits: a.splits }, a.ids),
    splitRowHead: (a) => lib.splitRowHead({ tabs: a.tabs, splits: a.splits }, a.order, a.id),
    paneChunks: (a) => lib.paneChunks(a.ids),
    stackAt: (a) => lib.stackAt(a.split),
    splitTabIds: (a) => lib.splitTabIds(a.split),
    isStacked: (a) => lib.isStacked(a.split, a.tabId),
    evenSlots: (a) => lib.evenSlots(a.ids),
    withPaneIds: (a) => lib.withPaneIds(a.split, a.ids),
    splitOf: (a) => lib.splitOf({ splits: a.splits }, a.tabId ?? undefined) ?? null,
    splitsWhereUnavailable: (a) => lib.splitsWhereUnavailable({ splits: a.splits }, a.available),
    splitLane: (a) => lanes.splitLane({ pin: a.pin ?? undefined }),
    crossesLane: (a) => lanes.crossesLane({ pin: a.anchor ?? undefined }, { pin: a.tab ?? undefined }),
    splitsOnRow: (a) => lanes.splitsOnRow({ tabs: a.tabs }, a.tabId, a.rowTabId),
  };
  const project = () => {
    const s = S();
    return {
      splits: s.splits,
      windows: Object.fromEntries(Object.entries(s.windows).map(([id, w]) => [id, { tabIds: w.tabIds, activeTabIds: w.activeTabIds }])),
      groups: Object.fromEntries(Object.entries(s.groups).map(([id, g]) => [id, g.tabIds])),
    };
  };
  const apply = (a) => {
    switch (a.type) {
      case "seed": return void useBrowser.setState({ tabs: a.tabs, windows: a.windows, groups: a.groups, splits: a.splits });
      case "call": return pure[a.fn](a);
      case "createSplit": return void S().createSplit(a.tabIds, a.orientation);
      case "openSplitPane": return void actions.openSplitPane(a.windowId, a.options);
      case "focusPane": return S().focusPane(a.windowId, a.delta);
      case "focusSplitPane": return actions.focusSplitPane(a.windowId, a.delta);
      case "openInSplit": return actions.openInSplit(a.tabId, a.side);
      case "openLinkInSplit": return actions.openLinkInSplit(a.sourceTabId, a.url, a.adoptId);
      case "dropTabIntoSplit": return void actions.dropTabIntoSplit(a.tabId, a.targetTabId, a.side);
      case "dropTabOnRow": return void actions.dropTabOnRow(a.tabId, a.rowTabId, a.side);
      case "closeSplitView": return actions.closeSplitView(a.splitId);
      case "updateSplit": return actions.updateSplit(a.id, a.patch);
      case "replaceSplitPane": return actions.replaceSplitPane(a.paneTabId, a.tabId);
      case "removeTabFromSplit": return actions.removeTabFromSplit(a.tabId);
      case "separateSplit": return S().separateSplit(a.splitId);
      case "flipSplit": return S().flipSplit(a.splitId);
      case "toggleSplitOrientation": return S().toggleSplitOrientation(a.splitId);
      case "movePane": return S().movePane(a.tabId, a.delta);
      case "toggleTabLayout": return S().toggleTabLayout(a.windowId);
      default: throw new Error(`Unknown splits action: ${a.type}`);
    }
  };
  return replayDriver(() => ({ platform: nativeFake(), state: project, apply, cleanup() {} }));
}

// ---- main -------------------------------------------------------------------------------------------------------
const [scratch, outDir, bin] = process.argv.slice(2).map((p) => p && resolve(p));
if (!scratch || !outDir || !bin) throw new Error("Usage: node record-splits.mjs ARDIA_SCRATCH OUT_DIR RUST_BIN");
const replay = await ardiaReplay(scratch);
const all = [...sequences(), ...calls().map((c) => [world([], []), c])];
const runnable = [];
const pending = [];
for (const [index, actions] of all.entries()) {
  const ok = await replay(actions);
  const run = spawnSync(bin, { input: JSON.stringify(actions), encoding: "utf8", maxBuffer: 1 << 28 });
  if (run.status !== 0 && /not yet implemented/.test(run.stderr)) {
    pending.push({ index, actions: actions.slice(1).map((a) => a.type === "call" ? `call:${a.fn}` : a.type), blockedBy: run.stderr.match(/not yet implemented: (.*)/)?.[1] });
  } else runnable.push({ fn: "replay", args: [actions], ok });
}
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, "cases.jsonl"), runnable.map((c) => JSON.stringify(c)).join("\n") + "\n");
writeFileSync(join(outDir, "pending.json"), JSON.stringify(pending, null, 2) + "\n");
copyFileSync(join(kit, "sample", "rust-anything.json"), join(outDir, "rust-anything.json"));
console.log(`Recorded ${all.length} cases: ${runnable.length} to replay, ${pending.length} pending (${join(outDir, "pending.json")}).`);
execFileSync("rust-anything", ["diff", outDir, "--module", join(kit, "sample", "index.cjs"), "--ignore-key-order"], {
  stdio: "inherit", env: { ...process.env, RIA_REPLAY_BIN: bin },
});
