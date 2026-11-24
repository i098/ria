// Ria original, no Ardia source.

import { resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";

export async function loadArdia(scratch) {
  const root = pathToFileURL(resolve(scratch) + sep);
  await import(new URL("apps/browser/src/test-loader.mjs", root));
  return (path) => import(new URL(`apps/browser/src/${path}`, root));
}
