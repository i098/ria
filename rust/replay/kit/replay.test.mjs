// Ria original, no Ardia source.

import assert from "node:assert/strict";
import { test } from "node:test";
import { nativeFake, replayDriver, stateJson } from "./replay.mjs";

test("state JSON sorts keys and sets and omits functions and undefined fields", () => {
  const state = {
    z: new Map([["z", 1], ["skip", undefined], ["a", null], ["b", { z: true, a: false }]]),
    a: new Set(["雪", "z", "a"]),
    skip: () => {},
  };
  assert.equal(JSON.stringify(stateJson(state)), '{"a":["a","z","雪"],"z":{"a":null,"b":{"a":false,"z":true},"z":1}}');
});

test("replay snapshots every action and retains native calls in order", async () => {
  const replay = replayDriver(() => {
    const platform = nativeFake();
    const state = { total: 0 };
    const get = platform.method("get", 4);
    const save = platform.method("save");
    return {
      apply: async (key) => {
        if (!key) return;
        state.total += await get(key);
        save(state.total);
      },
      state: () => state,
      platform,
      cleanup() {},
    };
  });
  assert.deepEqual(await replay(["a", null, "b"]), [
    { state: { total: 4 }, calls: [{ name: "get", args: ["a"], result: 4 }, { name: "save", args: [4] }] },
    { state: { total: 4 }, calls: [] },
    { state: { total: 8 }, calls: [{ name: "get", args: ["b"], result: 4 }, { name: "save", args: [8] }] },
  ]);
});
