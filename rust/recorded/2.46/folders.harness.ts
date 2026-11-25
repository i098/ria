// Ria original, no Ardia source.
import { folderChildren } from "./folders.ts";

type Case = { parentOf: [string, string][]; keys: string[]; fallback: string };

// Runs each case; gives the entries of `parentOf` after the call and of the children map.
export function replayFolders(input: string): unknown[] {
  const cases: Case[] = JSON.parse(input);
  return cases.map(({ parentOf, keys, fallback }) => {
    const map = new Map(parentOf);
    const children = folderChildren(map, keys, fallback);
    return { parentOf: [...map], children: [...children] };
  });
}
