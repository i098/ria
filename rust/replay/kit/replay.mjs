// Ria original, no Ardia source.

export function stateJson(state) {
  const replacer = (_, value) => {
    if (value instanceof Map) value = Object.fromEntries(value);
    if (value instanceof Set) {
      const items = Array.from(value, stateJson);
      if (items.every((item) => typeof item === "string")) return items.sort();
      const keyed = items.map((item) => [JSON.stringify(item ?? null), item]);
      keyed.sort(([a], [b]) => a < b ? -1 : Number(a > b));
      return keyed.map(([, item]) => item);
    }
    if (value && typeof value === "object" && !Array.isArray(value)) {
      return Object.fromEntries(Object.keys(value).sort().map((key) => [key, value[key]]));
    }
    return value;
  };
  const text = JSON.stringify(state, replacer);
  return text === undefined ? undefined : JSON.parse(text);
}

export function nativeFake() {
  let calls = [];
  return {
    method(name, result) {
      return (...args) => {
        calls.push(stateJson({ name, args, result }));
        return result;
      };
    },
    takeCalls() {
      const taken = calls;
      calls = [];
      return taken;
    },
  };
}

export function replayDriver(createCase) {
  return async function replay(actions) {
    const { apply, state, platform, cleanup } = await createCase();
    try {
      const steps = [];
      for (const action of actions) {
        const result = await apply(action);
        steps.push(stateJson({ state: state(), calls: platform.takeCalls(), result }));
      }
      return steps;
    } finally {
      await cleanup();
    }
  };
}
