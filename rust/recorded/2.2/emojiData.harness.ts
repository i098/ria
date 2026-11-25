import { EMOJI_GROUPS } from "./emojiData.ts";

// emojiData.ts exports only data: this gives it to the recorder as a value.
export function emojiGroups(): { title: string; list: string }[] {
  return EMOJI_GROUPS.map(({ title, list }) => ({ title, list }));
}
