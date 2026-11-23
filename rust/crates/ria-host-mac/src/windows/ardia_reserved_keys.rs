// Ported from ardia packages/ardiacore/ios/ArdiaReservedKeys.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::Cell;

use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSApplication, NSEvent, NSEventModifierFlags as Mods, NSEventType, NSMenu, NSMenuItem,
    NSUserInterfaceItemIdentification, NSView, NSWindow,
};
use objc2_foundation::{NSString, ns_string};

use crate::core::bindings::PageView;

const QUIT: &str = "ardia.shortcut.std:quit:";
const COPY_URL: &str = "ardia.command.copyUrl";
const FULL_SCREEN: &str = "ardia.shortcut.std:toggleFullScreen:";
const SIDEBAR: &str = "ardia.command.toggleSidebar";
const RESERVED: &[&str] = &[
    "ardia.command.closeTab",
    "ardia.shortcut.std:closeWindow:",
    "ardia.command.newIncognitoWindow",
    "ardia.command.newTab",
    "ardia.command.newWindow",
    "ardia.command.reopenClosedTab",
    "ardia.command.nextTab",
    "ardia.command.previousTab",
    "ardia.command.nextProfile",
    "ardia.command.previousProfile",
    "ardia.command.tabSwitcher",
    QUIT,
    COPY_URL,
    "ardia.command.devTools",
    "ardia.command.toggleDevTools",
    "ardia.command.searchTabs",
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KeyContext {
    #[default]
    Windowed,
    FullScreenBrowser,
    FullScreenPage,
}

impl KeyContext {
    fn includes(self, identifier: &str) -> bool {
        match self {
            Self::Windowed => RESERVED.contains(&identifier),
            Self::FullScreenBrowser => identifier == FULL_SCREEN || RESERVED.contains(&identifier),
            Self::FullScreenPage => matches!(identifier, QUIT | FULL_SCREEN),
        }
    }
}

fn first_code(key: &NSString) -> u16 {
    if key.is_empty() {
        0
    } else {
        key.characterAtIndex(0)
    }
}

fn is_function_key(code: u16) -> bool {
    (0xF700..=0xF8FF).contains(&code)
}

fn modifiers(code: u16, mods: Mods) -> Mods {
    let mask = Mods::Command | Mods::Shift | Mods::Option | Mods::Control | Mods::Function;
    if is_function_key(code) {
        mods & mask & !Mods::Function
    } else {
        mods & mask
    }
}

fn menu_item(
    menu: &NSMenu,
    services: Option<&NSMenu>,
    key: &NSString,
    mods: Mods,
    includes: &impl Fn(&str) -> bool,
) -> Option<Retained<NSMenuItem>> {
    let wanted = modifiers(first_code(key), mods);
    for item in menu.itemArray().iter() {
        if let Some(submenu) = item.submenu() {
            if !services.is_some_and(|services| std::ptr::eq(&*submenu, services))
                && let Some(found) = menu_item(&submenu, services, key, mods, includes)
            {
                return Some(found);
            }
            continue;
        }
        if item.isAlternate() || (item.isHidden() && !item.allowsKeyEquivalentWhenHidden()) {
            continue;
        }
        let Some(id) = item.identifier() else {
            continue;
        };
        if !objc2::rc::autoreleasepool(|pool| {
            // SAFETY: The borrow ends inside the pool that supplied the UTF-8 storage.
            includes(unsafe { id.to_str(pool) })
        }) {
            continue;
        }
        let equivalent = item.keyEquivalent();
        if equivalent.is_empty() {
            continue;
        }
        let lowercase = equivalent.lowercaseString();
        let mut item_mods = modifiers(first_code(&equivalent), item.keyEquivalentModifierMask());
        if !equivalent.isEqualToString(&lowercase) {
            item_mods |= Mods::Shift;
        }
        if item_mods == wanted && lowercase.isEqualToString(key) {
            return Some(item);
        }
    }
    None
}

fn menu_item_for_event(
    app: &NSApplication,
    event: &NSEvent,
    includes: impl Fn(&str) -> bool,
) -> Option<Retained<NSMenuItem>> {
    let mods = event.modifierFlags() & Mods::DeviceIndependentFlagsMask;
    let key = event.charactersIgnoringModifiers()?.lowercaseString();
    let first = first_code(&key);
    if !(0xF704..=0xF726).contains(&first)
        && !(mods.contains(Mods::Function) && !is_function_key(first))
        && !mods.intersects(Mods::Command | Mods::Control | Mods::Option)
    {
        return None;
    }
    let key = if first == 0x19 && key.length() == 1 {
        ns_string!("\t")
    } else {
        &key
    };
    let menu = app.mainMenu()?;
    let services = app.servicesMenu();
    if let Some(item) = menu_item(&menu, services.as_deref(), key, mods, &includes) {
        return Some(item);
    }
    let unshifted = event
        .charactersByApplyingModifiers(Mods::empty())?
        .lowercaseString();
    if unshifted.is_empty() || unshifted.isEqualToString(key) {
        return None;
    }
    menu_item(&menu, services.as_deref(), &unshifted, mods, &includes)
}

pub fn reserved_menu_item(
    app: &NSApplication,
    event: &NSEvent,
    context: KeyContext,
) -> Option<Retained<NSMenuItem>> {
    menu_item_for_event(app, event, |id| context.includes(id))
}

pub fn is_reserved_binding(
    app: &NSApplication,
    key: &NSString,
    unshifted: &NSString,
    mods: Mods,
) -> bool {
    let Some(menu) = app.mainMenu() else {
        return false;
    };
    let services = app.servicesMenu();
    [key, unshifted].into_iter().any(|key| {
        !key.is_empty()
            && menu_item(
                &menu,
                services.as_deref(),
                &key.lowercaseString(),
                mods,
                &|id| KeyContext::Windowed.includes(id),
            )
            .is_some()
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct KeyPress {
    at: f64,
    window: isize,
}

#[derive(Default)]
pub struct ReservedKeys {
    sidebar: Cell<KeyPress>,
}

impl ReservedKeys {
    fn is_sidebar_key(app: &NSApplication, event: &NSEvent) -> bool {
        event.r#type() == NSEventType::KeyDown
            && menu_item_for_event(app, event, |id| id == SIDEBAR).is_some()
    }

    pub fn is_second_sidebar_press(&self, app: &NSApplication, event: &NSEvent) -> bool {
        if !Self::is_sidebar_key(app, event) || event.isARepeat() {
            return false;
        }
        let next = KeyPress {
            at: event.timestamp(),
            window: event.windowNumber(),
        };
        let first = self.sidebar.get();
        if first == next {
            return false;
        }
        let second = first.at > 0.
            && first.window == next.window
            && next.at - first.at <= NSEvent::doubleClickInterval();
        self.sidebar
            .set(if second { KeyPress::default() } else { next });
        second
    }

    pub fn keeps_sidebar_key_in_page(
        &self,
        app: &NSApplication,
        window: &NSWindow,
        event: &NSEvent,
    ) -> bool {
        if !Self::is_sidebar_key(app, event) {
            return false;
        }
        let typing = window
            .firstResponder()
            .and_then(|responder| responder.downcast::<NSView>().ok())
            .is_some_and(|view| PageView::is_editing_text(&view));
        if !typing {
            self.sidebar.set(KeyPress::default());
        }
        typing
    }

    pub fn perform(&self, app: &NSApplication, event: &NSEvent, context: KeyContext) -> bool {
        event.r#type() == NSEventType::KeyDown
            && (reserved_menu_item(app, event, context).is_some()
                || self.is_second_sidebar_press(app, event))
            && app
                .mainMenu()
                .is_some_and(|menu| menu.performKeyEquivalent(event))
    }
}

pub fn keeps_reserved_key_in_dev_tools(
    app: &NSApplication,
    event: &NSEvent,
    context: KeyContext,
    dev_tools: &NSView,
) -> bool {
    if modifiers(b'c'.into(), event.modifierFlags()) != (Mods::Command | Mods::Shift)
        || !event
            .charactersIgnoringModifiers()
            .is_some_and(|key| key.lowercaseString().isEqualToString(ns_string!("c")))
        || !reserved_menu_item(app, event, context)
            .and_then(|item| item.identifier())
            .is_some_and(|id| id.isEqualToString(ns_string!("ardia.command.copyUrl")))
    {
        return false;
    }
    event
        .window(app.mtm())
        .and_then(|window| window.firstResponder())
        .and_then(|responder| responder.downcast::<NSView>().ok())
        .is_some_and(|focused| focused.isDescendantOf(dev_tools))
}

pub fn is_reserved_key_event(mtm: MainThreadMarker, event: &NSEvent, context: KeyContext) -> bool {
    reserved_menu_item(&NSApplication::sharedApplication(mtm), event, context).is_some()
}
