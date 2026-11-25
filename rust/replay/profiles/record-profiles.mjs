// Ria original, no Ardia source.

import { once } from "node:events";
import { isDeepStrictEqual } from "node:util";
import { loadArdia } from "../kit/load-ardia.mjs";
import { nativeFake, replayDriver } from "../kit/replay.mjs";

const [scratch] = process.argv.slice(2);
if (!scratch) throw new Error("Usage: node record-profiles.mjs ARDIA_SCRATCH");

const load = await loadArdia(scratch);
// current.ts first: loading the store before it deadlocks the import cycle.
await load("state/browser/current.ts");
const [{ useBrowser }, actions, picker, request, clear] = await Promise.all([
  load("state/browser/store.ts"),
  load("state/profiles/actions.ts"),
  load("state/profiles/iconPicker.ts"),
  load("state/profiles/createProfileRequest.ts"),
  load("state/profiles/clearDataRequest.ts"),
]);
useBrowser.getState().hydrate({});
const start = {
  profiles: useBrowser.getState().profiles,
  profileOrder: useBrowser.getState().profileOrder,
  orphanedProfileData: [],
};

const replay = replayDriver(() => {
  useBrowser.setState(start);
  picker.useProfileIconPicker.setState({ picker: null });
  request.useCreateProfileRequest.setState({ request: null, sheet: null });
  clear.useClearDataRequest.setState({ windowId: null });
  return {
    platform: nativeFake(),
    state: () => {
      const s = useBrowser.getState();
      return {
        profiles: s.profiles,
        profileOrder: s.profileOrder,
        orphanedProfileData: s.orphanedProfileData,
        picker: picker.useProfileIconPicker.getState().picker,
        request: request.useCreateProfileRequest.getState().request,
        sheet: request.useCreateProfileRequest.getState().sheet,
      };
    },
    async apply(a) {
      const s = useBrowser.getState();
      switch (a.type) {
        case "seed":
          useBrowser.setState({ profiles: a.profiles, profileOrder: a.profileOrder, orphanedProfileData: a.orphanedProfileData ?? [] });
          return;
        case "updateProfile": return s.updateProfile(a.id, a.patch);
        case "setProfileLook": return s.setProfileLook(a.id, a.look);
        case "reorderProfiles": return s.reorderProfiles(a.ids);
        case "profileDataDeleted": return s.profileDataDeleted(a.engineId);
        case "chooseIcon": return actions.chooseIcon(a.profileId, a.icon);
        case "setShareTopApps": return actions.setShareTopApps(a.profileId, a.share);
        case "commitProfileName": return actions.commitProfileName(a.profileId, a.text);
        case "openProfileIconPicker": return picker.openProfileIconPicker(a.profileId, a.windowId, a.place, null);
        case "closeProfileIconPicker": return picker.closeProfileIconPicker();
        case "closeProfileIconPickerIn": return picker.closeProfileIconPickerIn(a.windowId);
        case "chooseProfileIcon": return picker.chooseProfileIcon(a.arg0, a.icon);
        case "closeProfileIconPickerIfShown": {
          const live = picker.useProfileIconPicker.getState().picker;
          return picker.closeProfileIconPickerIfShown(isDeepStrictEqual(live, a.picker) ? live : a.picker);
        }
        case "requestCreateProfile": {
          void request.requestCreateProfile(a.windowId, a.preset);
          return;
        }
        case "endCreateProfileRequest": return request.endCreateProfileRequest({ ...a.request, resolve() {} }, a.id);
        case "takeCreateProfileSheet": return request.takeCreateProfileSheet();
        case "requestClearData": return clear.requestClearData(a.windowId);
        default: throw new Error(`unknown action ${a.type}`);
      }
    },
    cleanup() {},
  };
});

const base = { id: "w", name: "Work", color: "blue", icon: null, createdAt: 5, presetThemes: {}, seenTheme: null, seenColor: "blue" };
const seed = {
  type: "seed",
  profiles: { ...start.profiles, w: base, x: { ...base, id: "x", name: "X", color: "green", dataId: "w", switcherDot: true } },
  profileOrder: ["default", "w", "x"],
  orphanedProfileData: ["e1", "e2"],
};
const pk = { profileId: "w", windowId: "win", place: "sidebar", anchor: null };

const patches = [
  { name: "  Spaced  " }, { icon: "star" }, { icon: null }, { switcherDot: false }, { pinnedCollapsed: true },
  { shareTopApps: true }, { syncTabs: false }, { passwordManager: "pm" }, {},
];
const looks = [
  { color: "red" }, { theme: null }, { theme: { a: 1 } }, { theme: null, color: "pink" }, { theme: { a: 2 }, color: "yellow" },
  { presetThemes: { red: { b: 1 } } }, {},
];
const cases = [];
for (const id of ["w", "x", "nope"]) {
  for (const patch of patches) cases.push([seed, { type: "updateProfile", id, patch }, { type: "updateProfile", id, patch: { icon: "z" } }]);
  for (const look of looks) cases.push([seed, { type: "setProfileLook", id, look }]);
  for (const icon of ["a", null]) {
    cases.push([seed, { type: "chooseIcon", profileId: id, icon }]);
    cases.push([seed, { type: "openProfileIconPicker", profileId: id, windowId: "win", place: "settings", anchorView: null },
      { type: "chooseProfileIcon", arg0: { ...pk, profileId: id }, icon }]);
  }
  for (const share of [true, false]) cases.push([seed, { type: "setShareTopApps", profileId: id, share }]);
  for (const text of ["", "  ", "New", " Work ", "Work", "\u00a0Pad\ufeff"]) cases.push([seed, { type: "commitProfileName", profileId: id, text }]);
}
for (const ids of [[], ["x"], ["x", "w"], ["nope", "x"], ["x", "x"], ["default", "w", "x"]]) cases.push([seed, { type: "reorderProfiles", ids }]);
for (const engineId of ["e1", "e2", "e3"]) cases.push([seed, { type: "profileDataDeleted", engineId }]);
for (const windowId of ["win", "other"]) {
  cases.push([seed, { type: "openProfileIconPicker", profileId: "w", windowId: "win", place: "sidebar", anchorView: null },
    { type: "closeProfileIconPickerIn", windowId },
    { type: "closeProfileIconPickerIn", windowId: "win" }]);
}
cases.push([seed, { type: "openProfileIconPicker", profileId: "w", windowId: "win", place: "sidebar", anchorView: null },
  { type: "closeProfileIconPickerIfShown", picker: pk },
  { type: "closeProfileIconPickerIfShown", picker: { ...pk, place: "settings" } },
  { type: "closeProfileIconPicker" }]);
for (const windowId of [undefined, "", "gone"]) {
  for (const preset of [undefined, {}, { name: "N", shareWith: "w" }]) {
    const req = { type: "requestCreateProfile", ...(windowId === undefined ? {} : { windowId }), ...(preset ? { preset } : {}) };
    cases.push([seed, req, { type: "takeCreateProfileSheet" }, { type: "takeCreateProfileSheet" }]);
  }
}
for (const windowId of ["win", null]) cases.push([seed, { type: "requestClearData", windowId }]);
cases.push([seed, { type: "endCreateProfileRequest", request: { windowId: "win", preset: {} }, id: "w" }]);

for (const actions of cases) {
  const line = JSON.stringify({ fn: "replay", args: [actions], ok: await replay(actions) });
  if (!process.stdout.write(`${line}\n`)) await once(process.stdout, "drain");
}
