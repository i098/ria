import * as s from "./pluginShortcuts.ts";
import type { CommandFlag, PluginShortcut, ShortcutBinding, ShortcutKeyEvent, ShortcutRegistration } from "./pluginShortcuts.ts";

// The plain exports, and the generic ones over JSON actions. `held` lists the combos
// (`pluginShortcutText`) that `heldElsewhere` holds; without it the default runs.
// The guards throw on an argument that breaks the TS type (the recorder makes those), so a
// case records that error and not what JS reads from such a value (`[].shift` is a function).
type Action = { props: Record<string, unknown> };

const isObject = (x: unknown): x is Record<string, unknown> => typeof x === "object" && x !== null && !Array.isArray(x);
const fail = (message: string): never => {
  throw new TypeError(message);
};
const shortcutIn = (x: unknown): PluginShortcut => (isObject(x) && typeof x.key === "string" ? (x as PluginShortcut) : fail("shortcut.key must be text"));
const eventIn = (x: unknown): ShortcutKeyEvent => (isObject(x) ? (x as ShortcutKeyEvent) : fail("e must be an object"));
const flagIn = (x: unknown): CommandFlag => (x === "metaKey" || x === "ctrlKey" ? x : fail("commandFlag must be metaKey or ctrlKey"));
const listIn = (x: unknown, what: string): unknown[] => (Array.isArray(x) ? x : fail(`${what} must be a list`));
const bindingsIn = (x: unknown): ShortcutBinding<Action>[] =>
  listIn(x, "bindings").map((b) => (shortcutIn(isObject(b) ? b.shortcut : null), b as ShortcutBinding<Action>));
const whole = (x: unknown, what: string): number => (Number.isSafeInteger(x) && !Object.is(x, -0) ? (x as number) : fail(what));
const registrationsIn = (x: unknown): ShortcutRegistration<Action>[] =>
  listIn(x, "registrations").map((r) => {
    if (!isObject(r)) return fail("registration must be an object");
    whole(r.depth, "registration.depth");
    whole(r.seq, "registration.seq");
    bindingsIn(r.bindings);
    return r as unknown as ShortcutRegistration<Action>;
  });

export function parsePluginShortcutCase(raw: unknown): unknown {
  return s.parsePluginShortcut(raw);
}

export function pluginShortcutTextCase(shortcut: unknown): string {
  return s.pluginShortcutText(shortcutIn(shortcut));
}

export function isRefusedShortcutCase(shortcut: unknown): boolean {
  return s.isRefusedShortcut(shortcutIn(shortcut) as PluginShortcut & { command: boolean });
}

export function pluginShortcutOfCase(raw: unknown): unknown {
  return s.pluginShortcutOf(raw);
}

export function pluginShortcutModifiersCase(shortcut: unknown): string[] {
  return s.pluginShortcutModifiers(shortcutIn(shortcut));
}

export function pluginShortcutKeyCharCase(shortcut: unknown): string {
  return s.pluginShortcutKeyChar(shortcutIn(shortcut));
}

export function pluginShortcutHandledKeysCase(shortcut: unknown, commandFlag: unknown): unknown {
  return s.pluginShortcutHandledKeys(shortcutIn(shortcut), flagIn(commandFlag));
}

export function pluginShortcutMatchesCase(shortcut: unknown, e: unknown, commandFlag: unknown): boolean {
  return s.pluginShortcutMatches(shortcutIn(shortcut), eventIn(e), flagIn(commandFlag));
}

export function bindingsOfCase(actions: unknown): unknown {
  return s.bindingsOf(listIn(actions, "actions").map((a) => (isObject(a) && isObject(a.props) ? (a as Action) : fail("action.props must be an object"))));
}

export function mergeShortcutBindingsCase(registrations: unknown, held?: unknown): unknown {
  const regs = registrationsIn(registrations);
  if (held === undefined) return s.mergeShortcutBindings(regs);
  const combos = listIn(held, "held").map((h) => (typeof h === "string" ? h : fail("held must be a list of text")));
  return s.mergeShortcutBindings(regs, (x) => combos.includes(s.pluginShortcutText(x)));
}

export function pressedBindingCase(bindings: unknown, e: unknown, commandFlag: unknown): unknown {
  return s.pressedBinding(bindingsIn(bindings), eventIn(e), flagIn(commandFlag));
}

export function handledKeysOfCase(bindings: unknown, commandFlag: unknown): unknown {
  return s.handledKeysOf(bindingsIn(bindings), flagIn(commandFlag));
}
