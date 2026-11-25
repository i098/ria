// Ria original, no Ardia source.

// Check S of slice 2.10: replays each action sequence on the real Ardia store of `state/app` and on the Rust port,
// and compares the state JSON and the native calls after each action.
// Usage: node check.mjs ARDIA_SCRATCH RUST_REPLAY_BIN [cases.jsonl]

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { loadArdia } from "../kit/load-ardia.mjs";
import { nativeFake, replayDriver, stateJson } from "../kit/replay.mjs";
import { registerHooks } from "node:module";
import assert from "node:assert/strict";

const [scratch, bin, out] = process.argv.slice(2);

// Timers: the driver fires the indicator hold itself, and no timer keeps the process alive.
const timers = [];
const realSetTimeout = globalThis.setTimeout;
globalThis.setTimeout = (callback, ms, ...args) => {
  if (ms === 3000) {
    timers.push(callback);
    return timers.length;
  }
  return realSetTimeout(callback, ms, ...args);
};
globalThis.clearTimeout = ((clear) => (id) => (typeof id === "number" && id <= timers.length && id > 0 ? void (timers[id - 1] = null) : clear(id)))(globalThis.clearTimeout);

registerHooks({
  resolve(specifier, context, next) {
    if (!context.parentURL?.endsWith("/state/app/programControl.ts") || specifier !== "../pages/urls") return next(specifier, context);
    const stub = "export const isInternalTab = (t) => /^ardia:\\/\\//i.test(t.page.nav?.url ?? t.url ?? '');";
    return { url: `data:text/javascript,${encodeURIComponent(stub)}`, shortCircuit: true };
  },
});

const load = await loadArdia(scratch);
const { useBrowser } = await load("state/browser/store.ts");
const { emitEvent } = await load("test-native/index.ts");
const { nativeModule } = await load("../../../packages/shell/src/index.tsx");
useBrowser.getState().hydrate({});
useBrowser.getState().updateSettings({ cliControl: false });

let generation = 0;
const settle = () => new Promise((resolve) => setImmediate(resolve));

async function instance() {
  const id = ++generation;
  const [toasts, control, install] = await Promise.all([
    load(`state/app/toasts.ts?case=${id}`),
    load(`state/app/programControl.ts?case=${id}`),
    load(`state/app/linuxCommandInstall.ts?case=${id}`),
  ]);
  return { toasts, control, install };
}

// The fakes that the Rust fake records: `{name, args}` in the order of the calls.
const pending = [];
const controlCalls = nativeFake();
globalThis.acTestCliControl = (on) => {
  controlCalls.method(on === null ? "engine.cliControlStatus" : "engine.setCliControl")(...(on === null ? [] : [{ on }]));
  return new Promise((resolve, reject) => pending.push({ resolve, reject }));
};

const cliModule = nativeModule("ArdiaCli");
const documentModule = nativeModule("ArdiaDocuments");
let installListeners = [];
let installRun = null;
let installedVersion = null;
const documents = new Map();
cliModule.installedVersion = () => installedVersion;
cliModule.canInstallUnattended = () => false;
cliModule.addListener = (_name, listener) => {
  installListeners.push(listener);
  return { remove: () => void (installListeners = installListeners.filter((l) => l !== listener)) };
};
cliModule.install = () => {
  controlCalls.method("ArdiaCli.install")();
  return new Promise((resolve) => (installRun = resolve));
};
documentModule.readDocument = (name) => documents.get(name) ?? null;
documentModule.writeDocument = (name, contents) => {
  controlCalls.method("ArdiaDocuments.writeDocument")(name, contents);
  documents.set(name, contents);
};

function snapshot(it) {
  const { shown, caller } = it.control.useControlIndicator.getState();
  const { installing, step, installedVersion, binPath, error } = it.install.useCommandInstall.getState();
  return { shown, caller, server: it.control.useControlServer.getState().server, installing, step, installedVersion, binPath, error, on: false, toasts: it.toasts.useToasts.getState().toasts };
}

const withClock = (now, run) => {
  const clock = Date.now;
  Date.now = () => now;
  try {
    return run();
  } finally {
    Date.now = clock;
  }
};

const replay = replayDriver(async () => {
  const it = await instance();
  pending.length = 0;
  timers.length = 0;
  documents.clear();
  installedVersion = null;
  installRun = null;
  return {
    platform: controlCalls,
    state: () => snapshot(it),
    async apply(action) {
      switch (action.type) {
        case "showToast":
          return void it.toasts.showToast(action.windowId, action.title, action.message ?? undefined, action.options ? { ...action.options, ...(action.options.action ? { action: { ...action.options.action, run() {} } } : {}) } : undefined);
        case "hideToast":
          return void it.toasts.hideToast(action.windowId, action.id);
        case "hideKeyedToast":
          return void it.toasts.hideKeyedToast(action.windowId, action.key);
        case "noteControlCaller":
          return void it.control.noteControlCaller(action.caller);
        case "noteSocketRun":
          return void it.control.noteSocketRun(action.delta);
        case "noteSocketServing":
          return void it.control.noteSocketServing(action.serving);
        case "indicatorHoldElapsed": {
          const timer = timers.find(Boolean);
          timers[timers.indexOf(timer)] = null;
          return void timer?.();
        }
        case "startProgramControl":
          it.control.startProgramControl();
          return void (await settle());
        case "controlChanged":
          emitEvent("engine.cliControl", action.server);
          return void (await settle());
        case "controlCallSettled": {
          const call = pending.shift();
          if (action.error) call.reject(new Error(action.error));
          else call.resolve(action.server);
          return void (await settle());
        }
        case "installCommandInBackground":
          void it.install.installCommandInBackground(action.reason);
          return void (await settle());
        case "commandInstallLine":
          for (const listener of installListeners) listener({ line: action.line });
          return;
        case "commandInstallFinished": {
          installedVersion = action.installedVersion ?? null;
          const { now, installedVersion: _, type: __, ...result } = action;
          const clock = Date.now;
          Date.now = () => now;
          try {
            installRun({ ...result, appVersion: "" });
            await settle();
          } finally {
            Date.now = clock;
          }
          return;
        }
        default:
          throw new Error(`Unknown app action: ${action.type}`);
      }
    },
    cleanup() {},
  };
});

const at = (type, fields = {}) => ({ type, ...fields });
const off = { state: "off", connections: 0, generation: 0, file: "" };
const cases = {
  toastsKeyAndHide: [
    at("showToast", { windowId: "w", title: "One" }),
    at("showToast", { windowId: "w", title: "Two", message: "m", options: { key: "k", icon: "i", sticky: true, action: { title: "Undo" } } }),
    at("showToast", { windowId: "w", title: "Three", options: { key: "k" } }),
    at("showToast", { windowId: "w", title: "Four", options: { key: "other" } }),
    at("showToast", { windowId: "v", title: "Elsewhere" }),
    at("hideToast", { windowId: "w", id: 1 }),
    at("hideKeyedToast", { windowId: "w", key: "k" }),
    at("hideKeyedToast", { windowId: "w", key: "other" }),
    at("hideToast", { windowId: "v", id: 99 }),
    at("hideToast", { windowId: "v", id: 5 }),
    at("hideToast", { windowId: "nowhere", id: 5 }),
  ],
  callerNotes: [
    at("noteControlCaller", { caller: { name: "Claude", app: "/Applications/Claude.app" } }),
    at("noteControlCaller", { caller: { name: "Claude", app: "/Applications/Claude.app" } }),
    at("noteControlCaller", { caller: { name: "Codex", app: null } }),
  ],
  socketIndicator: [
    at("noteSocketServing", { serving: true }),
    at("noteSocketRun", { delta: -1 }),
    at("noteSocketRun", { delta: 1 }),
    at("noteControlCaller", { caller: { name: "Claude", app: null } }),
    at("noteSocketRun", { delta: -1 }),
    at("indicatorHoldElapsed"),
    at("noteSocketRun", { delta: 1 }),
    at("noteSocketRun", { delta: 1 }),
    at("noteSocketRun", { delta: -1 }),
    at("noteSocketServing", { serving: false }),
    at("indicatorHoldElapsed"),
  ],
  holdIsCancelledByAConnection: [
    at("noteSocketServing", { serving: true }),
    at("noteSocketRun", { delta: 1 }),
    at("noteSocketRun", { delta: -1 }),
    at("noteSocketRun", { delta: 1 }),
    at("indicatorHoldElapsed"),
    at("noteSocketRun", { delta: -1 }),
    at("indicatorHoldElapsed"),
  ],
  controlStartAndStatus: [
    at("startProgramControl"),
    at("startProgramControl"),
    at("controlCallSettled", { request: 1, server: off }),
    at("controlChanged", { server: { ...off, reason: "stopped itself" } }),
    at("controlChanged", { server: { state: "external", connections: 0, generation: 0, file: "" } }),
    at("controlChanged", { server: { ...off, generation: 3 } }),
    at("controlChanged", { server: { ...off, generation: 2, reason: "late" } }),
  ],
  controlCallRejected: [
    at("startProgramControl"),
    at("controlCallSettled", { request: 1, error: "engine is gone" }),
  ],
  installFailsThenSucceeds: [
    at("installCommandInBackground", { reason: "settings" }),
    at("installCommandInBackground", { reason: "upgrade" }),
    at("commandInstallLine", { line: "step download" }),
    at("commandInstallLine", { line: "noise" }),
    at("commandInstallFinished", { ok: false, error: "no network", installedVersion: null, now: 1000 }),
    at("installCommandInBackground", { reason: "upgrade" }),
    at("commandInstallLine", { line: "step link now" }),
    at("commandInstallFinished", { ok: true, binPath: "/usr/local/bin/ardia", version: "1.2.3", installedVersion: "1.2.3", now: 2000 }),
    at("installCommandInBackground", { reason: "settings" }),
    at("commandInstallFinished", { ok: false, installedVersion: "1.2.3", now: 3000 }),
  ],
};

let failures = 0;
const lines = [];
for (const [name, actions] of Object.entries(cases)) {
  const expected = await replay(actions);
  lines.push(JSON.stringify({ fn: "replay", args: [actions], ok: expected }));
  const actual = JSON.parse(execFileSync(bin, { input: JSON.stringify(actions), encoding: "utf8" }));
  try {
    assert.deepEqual(stateJson(actual), stateJson(expected));
    console.log(`ok   ${name} (${actions.length} actions)`);
  } catch (error) {
    failures++;
    console.log(`FAIL ${name}\n${error.message.split("\n").slice(0, 40).join("\n")}`);
  }
}
if (out) writeFileSync(out, lines.join("\n") + "\n");
console.log(`${Object.keys(cases).length - failures} of ${Object.keys(cases).length} sequences match`);
process.exit(failures ? 1 : 0);
