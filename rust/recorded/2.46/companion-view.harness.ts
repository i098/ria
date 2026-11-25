// Ria original, no Ardia source.
import { emptyScope, type ScopeState } from "./scopeState.ts";
import { fromLegacy, fromLegacyUrl } from "./companion/legacy.ts";
import { themeOf } from "./companion/theme.ts";
import { bookmarkTree, buildView, dataOwner, defaultProfileId, historyList, liveProfiles, pinnedArea } from "./companion/view.ts";

type Partial = { records?: ScopeState["records"]; seen?: ScopeState["seen"] } | null;

const scope = (s: Partial) => (s ? { ...emptyScope(), ...s } : undefined);

const calls: Record<string, (...args: never[]) => unknown> = {
  buildView: (scopes: Record<string, Partial>, me: string) =>
    buildView(Object.fromEntries(Object.entries(scopes).map(([k, s]) => [k, scope(s)!])), me),
  liveProfiles: (app: Partial) => [...liveProfiles(scope(app))],
  defaultProfileId: (entries: [string, object][]) => defaultProfileId(new Map(entries)),
  dataOwner: (app: Partial, syncId: string) => dataOwner(scope(app), syncId),
  pinnedArea: (s: Partial) => pinnedArea(scope(s)),
  bookmarkTree: (s: Partial) => bookmarkTree(scope(s)),
  historyList: (s: Partial) => historyList(scope(s)),
  themeOf: (app: Partial, syncId: string, theme?: unknown) => themeOf(scope(app)!, syncId, theme),
  fromLegacy: (value: unknown) => fromLegacy(value),
  fromLegacyUrl: (url: string) => fromLegacyUrl(url),
};

/** Runs each step `[name, ...args]` on the companion view units; `null` stands for a missing scope. */
export function replayCompanionView(input: string): unknown[] {
  const steps: [string, ...never[]][] = JSON.parse(input);
  return steps.map(([name, ...args]) => calls[name]!(...args));
}
