import * as k from "./pluginPermissions.ts";
import type { Check } from "./pluginChecks.ts";

// A permissions case records what `permissionsOf` read, its problems, and every helper over the
// result; a missing case reads both sides with `permissionsOf` first.
export const constantsCase = (): unknown => ({ PLUGIN_PERMISSIONS: k.PLUGIN_PERMISSIONS, PERMISSION_KEYS: k.PERMISSION_KEYS, TAB_ACCESS: k.TAB_ACCESS, NO_PERMISSIONS: k.NO_PERMISSIONS });
export const legacyPermissionsCase = (given: unknown): unknown => k.legacyPermissions(given);
export const permissionsCase = (value: unknown): unknown => {
  const c: Check = { problems: [] };
  const p = k.permissionsOf(c, value);
  return { result: p, problems: c.problems, granted: k.grantedPermissions(p), canReadTabs: k.canReadTabs(p), canOpenTabs: k.canOpenTabs(p), hasNetwork: k.hasNetwork(p), allowedHosts: k.allowedHostsOf(p) };
};
export const missingPermissionsCase = (caller: unknown, callee: unknown): string[] => k.missingPermissions(k.permissionsOf({ problems: [] }, caller), k.permissionsOf({ problems: [] }, callee));
