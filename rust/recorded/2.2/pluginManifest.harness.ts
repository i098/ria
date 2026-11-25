import type { Fields } from "./decode.ts";
import { checkManifest, contributionsOf, defaultValues, networkLabel, readManifest } from "./pluginManifest.ts";

export const readManifestCase = (text: string): unknown => readManifest(text);

export const checkManifestCase = (raw: unknown, declaredManifest?: number): unknown => checkManifest(raw as Fields, declaredManifest);

// `contributionsOf`, `defaultValues` and `networkLabel` take a checked manifest, which cannot cross the bridge:
// each case reads it from `text`, and gives null when the text has no manifest.
export function manifestUsesCase(text: string): unknown {
  const { manifest } = readManifest(text);
  return manifest && { contributions: contributionsOf(manifest), defaults: defaultValues(manifest), network: networkLabel(manifest.permissions) };
}
