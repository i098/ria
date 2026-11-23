// Test hook for `rust-anything record|diff|fuzz`: fixes `Date.now()` to `RIA_NOW_MS`, so that the
// original and the port read the same clock. Use: NODE_OPTIONS=--import=$PWD/clock-freeze.mjs
Date.now = () => Number(process.env.RIA_NOW_MS);
