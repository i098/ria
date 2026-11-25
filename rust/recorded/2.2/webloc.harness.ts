import * as webloc from "./webloc.ts";

export const weblocText = (url: string, id?: string) => webloc.weblocText(url, id);
export const parseWebloc = (text: string) => webloc.parseWebloc(text);
export const weblocLinkId = (text: string) => webloc.weblocLinkId(text);

// `newLinkId` reads the clock and `Math.random`: record its shape, not its value.
export function newLinkIdShape(): boolean {
  const id = webloc.newLinkId();
  return /^[0-9a-z]{9,}$/.test(id) && id !== webloc.newLinkId();
}
