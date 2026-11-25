// Ported from ardia apps/browser/src/lib/pluginShortcuts.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Keyboard shortcuts that plugins give their actions: parse `"cmd+shift+k"` or `"⌘⇧K"`, refuse
//! the ones the browser keeps, and match key events.
//!
//! Changes from Ardia:
//! - `PluginShortcut & { command: boolean }` is `CommandShortcut`.
//! - `MODIFIER_WORDS` is a plain object in TS, so `constructor` and `__proto__` read as modifiers
//!   that are none of the four (inherited members). The port keeps that: such a word counts as a
//!   modifier, two of the same word are a repeat, and it sets no flag.
//! - `ShortcutKeyEvent` flags are `bool`: the caller gives the truthiness (`!!e.metaKey`).
//! - `bindingsOf` reads `props` through the `Props` trait; bindings hold `&A`, not a shared
//!   reference to the same object. `mergeShortcutBindings` takes the registrations by value and
//!   `heldElsewhere` has no default: pass `|_| false`.
//! - `depth` and `seq` are `i64`, so the sort has a total order (TS numbers can be `NaN`).

use super::decode::Fields;
use ria_core::identity::telemetry_wire::Json;

/// `PluginShortcutModifier` (`pluginShortcuts.ts:1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginShortcutModifier {
    Control,
    Option,
    Shift,
    Command,
}

impl PluginShortcutModifier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::Option => "option",
            Self::Shift => "shift",
            Self::Command => "command",
        }
    }
}

/// `PluginShortcut` (`pluginShortcuts.ts:3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginShortcut {
    pub key: String,
    pub control: bool,
    pub option: bool,
    pub shift: bool,
}

/// `PluginShortcut & { command: boolean }` (`pluginShortcuts.ts:68`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandShortcut {
    pub shortcut: PluginShortcut,
    pub command: bool,
}

/// `ShortcutKeyEvent` (`pluginShortcuts.ts:5`), with each flag already made `bool`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShortcutKeyEvent {
    pub key: Option<String>,
    pub meta_key: bool,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub shift_key: bool,
}

/// `ShortcutHandledKey` (`pluginShortcuts.ts:7`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutHandledKey {
    pub key: String,
    pub meta_key: bool,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub shift_key: bool,
}

/// `CommandFlag` (`pluginShortcuts.ts:9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandFlag {
    MetaKey,
    CtrlKey,
}

/// `MODIFIER_WORDS[word]` (`pluginShortcuts.ts:11`): the modifier, or an inherited member of a
/// plain object (`constructor`, `__proto__`) that is a value but no modifier.
fn modifier_word(word: &str) -> Option<&'static str> {
    Some(match word {
        "cmd" | "command" => "command",
        "ctrl" | "control" => "control",
        "opt" | "option" | "alt" => "option",
        "shift" => "shift",
        "constructor" => "constructor",
        "__proto__" => "__proto__",
        _ => return None,
    })
}

/// `MODIFIER_GLYPHS` (`pluginShortcuts.ts:21`).
fn modifier_glyph(c: char) -> Option<&'static str> {
    Some(match c {
        '⌘' => "command",
        '⌃' => "control",
        '⌥' => "option",
        '⇧' => "shift",
        _ => return None,
    })
}

const PUNCTUATION: &str = "-=[]\\;',./`";
const LIFECYCLE_KEYS: [&str; 6] = ["q", "w", "t", "n", "h", "m"];
const RESERVED: [&str; 16] = [
    "cmd+a",
    "cmd+c",
    "cmd+v",
    "cmd+x",
    "cmd+z",
    "cmd+shift+z",
    "cmd+shift+v",
    "cmd+opt+shift+v",
    "cmd+,",
    "cmd+`",
    "cmd+shift+`",
    "cmd+space",
    "cmd+ctrl+space",
    "cmd+opt+space",
    "cmd+ctrl+f",
    "cmd+ctrl+q",
];
const MAX_TEXT: usize = 40;
const FIRST_FUNCTION_KEY_CODE: u32 = 0xf704;

/// `FUNCTION_KEY.exec` (`pluginShortcuts.ts:23`, `/^f([1-9]|1[0-2])$/`): the function key number.
fn function_key(key: &str) -> Option<u32> {
    let digits = key.strip_prefix('f')?;
    let ok = matches!(digits.as_bytes(), [b'1'..=b'9'] | [b'1', b'0'..=b'2']);
    ok.then(|| digits.parse().ok()).flatten()
}

fn is_lower_letter(key: &str) -> bool {
    matches!(key.as_bytes(), [b'a'..=b'z'])
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `keyOf` (`pluginShortcuts.ts:44`).
fn key_of(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    if lower == "space" || function_key(&lower).is_some() {
        return Some(lower);
    }
    let mut chars = lower.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    (c.is_ascii_lowercase() || c.is_ascii_digit() || PUNCTUATION.contains(c)).then_some(lower)
}

/// `partsOf` (`pluginShortcuts.ts:52`): the modifiers and the key name.
fn parts_of(text: &str) -> Option<(Vec<&'static str>, String)> {
    if text.contains('+') {
        let mut words: Vec<&str> = text.split('+').map(super::js::js_trim).collect();
        let name = words.pop().unwrap_or("");
        if name.is_empty() && text.ends_with('+') {
            return None;
        }
        let modifiers = words
            .iter()
            .map(|w| modifier_word(&w.to_lowercase()))
            .collect::<Option<Vec<_>>>()?;
        return Some((modifiers, name.to_owned()));
    }
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;
    let mut modifiers = Vec::new();
    while at + 1 < chars.len() {
        let Some(m) = modifier_glyph(chars[at]) else {
            break;
        };
        modifiers.push(m);
        at += 1;
    }
    Some((modifiers, chars[at..].iter().collect()))
}

/// `parsePluginShortcut` (`pluginShortcuts.ts:68`).
pub fn parse_plugin_shortcut(raw: Option<&Json>) -> Option<CommandShortcut> {
    let Some(Json::String(raw)) = raw else {
        return None;
    };
    let text = super::js::js_trim(raw);
    if text.is_empty() || utf16_len(text) > MAX_TEXT {
        return None;
    }
    let (modifiers, name) = parts_of(text)?;
    if modifiers
        .iter()
        .enumerate()
        .any(|(i, m)| modifiers[..i].contains(m))
    {
        return None;
    }
    let key = key_of(&name)?;
    let has = |m: &str| modifiers.contains(&m);
    Some(CommandShortcut {
        shortcut: PluginShortcut {
            key,
            control: has("control"),
            option: has("option"),
            shift: has("shift"),
        },
        command: has("command"),
    })
}

/// `pluginShortcutText` (`pluginShortcuts.ts:81`).
pub fn plugin_shortcut_text(s: &PluginShortcut) -> String {
    let mut parts = vec!["cmd"];
    for (on, word) in [(s.control, "ctrl"), (s.option, "opt"), (s.shift, "shift")] {
        if on {
            parts.push(word);
        }
    }
    if !s.key.is_empty() {
        parts.push(&s.key);
    }
    parts.join("+")
}

/// `isRefusedShortcut` (`pluginShortcuts.ts:85`).
pub fn is_refused_shortcut(s: &CommandShortcut) -> bool {
    if !s.command {
        return true;
    }
    let key = &s.shortcut.key;
    if s.shortcut.shift && utf16_len(key) == 1 && !is_lower_letter(key) {
        return true;
    }
    LIFECYCLE_KEYS.contains(&key.as_str())
        || RESERVED.contains(&plugin_shortcut_text(&s.shortcut).as_str())
}

/// `pluginShortcutOf` (`pluginShortcuts.ts:91`).
pub fn plugin_shortcut_of(raw: Option<&Json>) -> Option<PluginShortcut> {
    let parsed = parse_plugin_shortcut(raw)?;
    (!is_refused_shortcut(&parsed)).then_some(parsed.shortcut)
}

/// `pluginShortcutModifiers` (`pluginShortcuts.ts:97`).
pub fn plugin_shortcut_modifiers(s: &PluginShortcut) -> Vec<PluginShortcutModifier> {
    use PluginShortcutModifier as M;
    [
        (s.control, M::Control),
        (s.option, M::Option),
        (s.shift, M::Shift),
        (true, M::Command),
    ]
    .into_iter()
    .filter_map(|(on, m)| on.then_some(m))
    .collect()
}

/// `pluginShortcutKeyChar` (`pluginShortcuts.ts:103`).
pub fn plugin_shortcut_key_char(s: &PluginShortcut) -> String {
    if s.key == "space" {
        return " ".to_owned();
    }
    match function_key(&s.key).and_then(|n| char::from_u32(FIRST_FUNCTION_KEY_CODE + n - 1)) {
        Some(c) => c.to_string(),
        None => s.key.clone(),
    }
}

/// `eventKeysOf` (`pluginShortcuts.ts:109`).
fn event_keys_of(s: &PluginShortcut) -> Vec<String> {
    if s.key == "space" {
        return vec![" ".to_owned()];
    }
    if function_key(&s.key).is_some() {
        return vec![s.key.to_uppercase()];
    }
    if is_lower_letter(&s.key) {
        vec![s.key.clone(), s.key.to_uppercase()]
    } else {
        vec![s.key.clone()]
    }
}

/// `flagsOf` (`pluginShortcuts.ts:115`): `metaKey`, `ctrlKey`, `altKey`, `shiftKey`.
fn flags_of(s: &PluginShortcut, command_flag: CommandFlag) -> [bool; 4] {
    let ctrl_key = s.control || command_flag == CommandFlag::CtrlKey;
    [
        command_flag == CommandFlag::MetaKey,
        ctrl_key,
        s.option,
        s.shift,
    ]
}

/// `pluginShortcutHandledKeys` (`pluginShortcuts.ts:120`).
pub fn plugin_shortcut_handled_keys(
    s: &PluginShortcut,
    command_flag: CommandFlag,
) -> Vec<ShortcutHandledKey> {
    let [meta_key, ctrl_key, alt_key, shift_key] = flags_of(s, command_flag);
    event_keys_of(s)
        .into_iter()
        .map(|key| ShortcutHandledKey {
            key,
            meta_key,
            ctrl_key,
            alt_key,
            shift_key,
        })
        .collect()
}

/// A key as `pluginShortcutMatches` compares it: one UTF-16 unit is lowercased.
fn comparable(key: &str) -> String {
    if utf16_len(key) == 1 {
        key.to_lowercase()
    } else {
        key.to_owned()
    }
}

/// `pluginShortcutMatches` (`pluginShortcuts.ts:125`).
pub fn plugin_shortcut_matches(
    s: &PluginShortcut,
    e: &ShortcutKeyEvent,
    command_flag: CommandFlag,
) -> bool {
    let Some(key) = e.key.as_deref().filter(|k| !k.is_empty()) else {
        return false;
    };
    if [e.meta_key, e.ctrl_key, e.alt_key, e.shift_key] != flags_of(s, command_flag) {
        return false;
    }
    let pressed = comparable(key);
    event_keys_of(s).iter().any(|k| comparable(k) == pressed)
}

/// `ShortcutBinding<A>` (`pluginShortcuts.ts:133`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutBinding<A> {
    pub shortcut: PluginShortcut,
    pub action: A,
}

/// `ShortcutRegistration<A>` (`pluginShortcuts.ts:135`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutRegistration<A> {
    pub depth: i64,
    pub seq: i64,
    pub bindings: Vec<ShortcutBinding<A>>,
}

/// `A extends { props: Record<string, unknown> }` (`pluginShortcuts.ts:137`).
pub trait Props {
    fn props(&self) -> &Fields;
}

/// `bindingsOf` (`pluginShortcuts.ts:137`).
pub fn bindings_of<A: Props>(actions: &[A]) -> Vec<ShortcutBinding<&A>> {
    actions
        .iter()
        .filter_map(|action| {
            let shortcut = action
                .props()
                .iter()
                .find(|(k, _)| k == "shortcut")
                .map(|(_, v)| v);
            plugin_shortcut_of(shortcut).map(|shortcut| ShortcutBinding { shortcut, action })
        })
        .collect()
}

/// `mergeShortcutBindings` (`pluginShortcuts.ts:144`): the first binding of each combo, by
/// depth then sequence, unless `held_elsewhere` holds it.
pub fn merge_shortcut_bindings<A>(
    registrations: impl IntoIterator<Item = ShortcutRegistration<A>>,
    held_elsewhere: impl Fn(&PluginShortcut) -> bool,
) -> Vec<ShortcutBinding<A>> {
    let mut ordered: Vec<_> = registrations.into_iter().collect();
    ordered.sort_by_key(|r| (r.depth, r.seq));
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for binding in ordered.into_iter().flat_map(|r| r.bindings) {
        if !seen.insert(plugin_shortcut_text(&binding.shortcut)) {
            continue;
        }
        if !held_elsewhere(&binding.shortcut) {
            out.push(binding);
        }
    }
    out
}

/// `pressedBinding` (`pluginShortcuts.ts:160`).
pub fn pressed_binding<'b, A>(
    bindings: &'b [ShortcutBinding<A>],
    e: &ShortcutKeyEvent,
    command_flag: CommandFlag,
) -> Option<&'b ShortcutBinding<A>> {
    bindings
        .iter()
        .find(|b| plugin_shortcut_matches(&b.shortcut, e, command_flag))
}

/// `handledKeysOf` (`pluginShortcuts.ts:164`).
pub fn handled_keys_of<A>(
    bindings: &[ShortcutBinding<A>],
    command_flag: CommandFlag,
) -> Vec<ShortcutHandledKey> {
    bindings
        .iter()
        .flat_map(|b| plugin_shortcut_handled_keys(&b.shortcut, command_flag))
        .collect()
}
