// Ria original, no Ardia source.

use std::ops::Range;

use objc2::runtime::Sel;
use objc2_app_kit::{NSEvent, NSEventModifierFlags};
use objc2_foundation::{NSNotFound, NSRange};
use rune::Runtime;
use rune::{Ime, Key, KeyEvent, Mods};

pub(crate) const NO_RANGE: NSRange = NSRange::new(NSNotFound as usize, 0);

/// The committed text and byte selection of the focused Rune field.
/// Do not supply the contents of a secure field.
pub struct TextInputSnapshot {
    text: String,
    selection: Range<usize>,
}

impl TextInputSnapshot {
    pub fn new(text: String, selection: Range<usize>) -> Option<Self> {
        text.get(selection.clone())?;
        Some(Self { text, selection })
    }

    pub fn selected_range(&self) -> NSRange {
        let start = self.text[..self.selection.start].encode_utf16().count();
        let len = self.text[self.selection.clone()].encode_utf16().count();
        NSRange::new(start, len)
    }

    pub fn substring(&self, range: NSRange) -> Option<&str> {
        self.text.get(byte_range(&self.text, range)?)
    }

    pub(crate) fn byte_range(&self, range: NSRange) -> Option<Range<usize>> {
        byte_range(&self.text, range)
    }
}

pub(crate) fn byte_range(text: &str, range: NSRange) -> Option<Range<usize>> {
    if range.location == NO_RANGE.location {
        return None;
    }
    let end = range.location.checked_add(range.length)?;
    Some(byte_index(text, range.location)?..byte_index(text, end)?)
}

fn byte_index(text: &str, target: usize) -> Option<usize> {
    let mut units = 0;
    for (index, ch) in text.char_indices() {
        if units == target {
            return Some(index);
        }
        units += ch.len_utf16();
        if units > target {
            return None;
        }
    }
    (units == target).then_some(text.len())
}

pub(crate) enum Input {
    Move(f32, f32),
    Leave,
    Button {
        x: f32,
        y: f32,
        down: bool,
        secondary: bool,
    },
    Wheel {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
    Focus(bool),
    Key(KeyEvent, bool),
    KeyUp(KeyEvent),
    Ime {
        change: Ime,
        replacement: Option<Range<usize>>,
    },
}

impl Input {
    pub(crate) fn send<S: 'static>(self, runtime: &mut Runtime<S>) {
        match self {
            Self::Move(x, y) => runtime.pointer_move(x, y),
            Self::Leave => runtime.pointer_leave(),
            Self::Button {
                x,
                y,
                down,
                secondary,
            } => match (down, secondary) {
                (true, false) => runtime.pointer_down(x, y),
                (false, false) => runtime.pointer_up(x, y),
                (true, true) => runtime.secondary_down(x, y),
                (false, true) => runtime.secondary_up(x, y),
            },
            Self::Wheel { x, y, dx, dy } => runtime.wheel_xy(x, y, dx, dy),
            Self::Focus(focused) => runtime.window_focus(focused),
            Self::Key(event, true) => runtime.repeat_key(event),
            Self::Key(event, false) => runtime.key(event),
            Self::KeyUp(event) => {
                runtime.key_up(event);
            }
            Self::Ime {
                change,
                replacement,
            } => {
                if let Some(range) = replacement {
                    runtime.ime(Ime::Select(range));
                }
                runtime.ime(change);
            }
        }
    }
}

pub(crate) fn modifiers(event: &NSEvent) -> Mods {
    let flags = event.modifierFlags();
    let mut mods = Mods::default();
    mods.shift = flags.contains(NSEventModifierFlags::Shift);
    mods.cmd = flags.contains(NSEventModifierFlags::Command);
    mods.logo = mods.cmd;
    mods.alt = flags.contains(NSEventModifierFlags::Option);
    mods.ctrl = flags.contains(NSEventModifierFlags::Control);
    mods
}

pub(crate) fn native_key(event: &NSEvent, mods: Mods) -> Option<Key> {
    let code = event.keyCode();
    match code {
        36 | 76 => Some(Key::Enter),
        48 => Some(Key::Tab),
        51 => Some(Key::Backspace),
        53 => Some(Key::Escape),
        115 => Some(Key::Home),
        116 => Some(Key::PageUp),
        117 => Some(Key::Delete),
        119 => Some(Key::End),
        121 => Some(Key::PageDown),
        123 => Some(Key::Left),
        124 => Some(Key::Right),
        125 => Some(Key::Down),
        126 => Some(Key::Up),
        _ => {
            let text = event.charactersIgnoringModifiers()?.to_string();
            if mods.cmd {
                text.chars()
                    .next()
                    .map(|ch| Key::Command(ch.to_ascii_lowercase()))
            } else {
                (!text.is_empty()).then_some(Key::Text(text))
            }
        }
    }
}

pub(crate) fn command(selector: Sel, mut mods: Mods) -> Option<KeyEvent> {
    let name = selector.name().to_str().ok()?;
    let name = name.strip_suffix(':')?;
    let name = match name.strip_suffix("AndModifySelection") {
        Some(name) => {
            mods.shift = true;
            name
        }
        None => name,
    };
    let key = match name {
        "moveLeft" | "moveBackward" => Key::Left,
        "moveRight" | "moveForward" => Key::Right,
        "moveUp" => Key::Up,
        "moveDown" => Key::Down,
        "moveToBeginningOfLine" | "moveToLeftEndOfLine" | "moveToBeginningOfDocument" => Key::Home,
        "moveToEndOfLine" | "moveToRightEndOfLine" | "moveToEndOfDocument" => Key::End,
        "deleteBackward" => Key::Backspace,
        "deleteForward" => Key::Delete,
        "insertNewline" | "insertNewlineIgnoringFieldEditor" => Key::Enter,
        "insertTab" => Key::Tab,
        "insertBacktab" => {
            mods.shift = true;
            Key::Tab
        }
        "cancelOperation" => Key::Escape,
        "pageUp" | "scrollPageUp" => Key::PageUp,
        "pageDown" | "scrollPageDown" => Key::PageDown,
        _ => return None,
    };
    Some(KeyEvent { key, mods })
}
