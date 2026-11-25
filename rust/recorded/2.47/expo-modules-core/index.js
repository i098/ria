// Stand-in for the one export that `packages/import/src/index.ts` reads. The replay harness
// sets `globalThis.__riaNative` to the fake native module before it loads `index.ts`.
export const requireNativeModule = () => globalThis.__riaNative;
