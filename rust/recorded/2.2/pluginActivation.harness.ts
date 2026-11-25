import * as k from "./pluginActivation.ts";
import { contributesOf } from "./pluginContributions.ts";
import type { Check } from "./pluginChecks.ts";

// `derivedActivationEvents` reads its contributions with `contributesOf` first.
export const constantsCase = (): unknown => ({ STARTUP_EVENTS: k.STARTUP_EVENTS, ACTIVATION_PREFIXES: k.ACTIVATION_PREFIXES });
export const activationEventProblemCase = (event: unknown): string | null => k.activationEventProblem(event);
export const activationEventsOfCase = (value: unknown): unknown => {
  const c: Check = { problems: [] };
  return { result: k.activationEventsOf(c, value), problems: c.problems };
};
export const derivedActivationEventsCase = (contributes: unknown, main: boolean, sidecar: boolean): string[] =>
  k.derivedActivationEvents(contributesOf({ problems: [] }, contributes, main), { main, sidecar });
export const startsEagerlyCase = (events: string[]): boolean => k.startsEagerly(events);
export const urlActivationHostsCase = (events: string[]): string[] => k.urlActivationHosts(events);
