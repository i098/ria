// Ported from ardia packages/ardiacore/ios/ArdiaPopupWindow.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
//
// Changes from Ardia:
// - `window:didRemoveTab:` also tells the Ria `TabStrips` record that the tab left the window, so the host pin
//   record is cleared (issue #71, wired by issue #49). Ardia keeps no host pin record. The call sits in the
//   production delegate, so a tab adopted out of a popup window reaches `TabStrips::removed`.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{Retained, Weak as ObjcWeak};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, Message, define_class, msg_send};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSApplication,
    NSAutoresizingMaskOptions as Mask, NSEvent, NSFontWeightRegular, NSImage,
    NSImageSymbolConfiguration, NSModalResponse, NSModalResponseAbort, NSScreen,
    NSUserInterfaceItemIdentification, NSWindow, NSWindowCollectionBehavior, NSWindowOrderingMode,
    NSWindowStyleMask, NSWindowTabbingMode, NSWindowWillCloseNotification,
};
use objc2_foundation::{
    NSComparisonResult, NSNotification, NSNotificationCenter, NSPoint, NSRect, NSSize, NSString,
    NSURL, ns_string,
};
use ria_engine::call::main_queue;
use serde_json::Value;

use super::ardia_popup_origin_strip::{PopupOriginStrip, popup_origin_text};
use super::ardia_reserved_keys::{KeyContext, reserved_menu_item};
use crate::core::bindings::{Adoption, Tab, Window};
use crate::core::convert::to_json;
use crate::core::host::resolve_permission;
use crate::tabs::ardia_tab_strip::TabStrips;

const STRIP_HEIGHT: f64 = 28.;

pub struct Popups {
    mtm: MainThreadMarker,
    background: bool,
    windows: RefCell<Vec<Rc<Popup>>>,
    strips: Rc<RefCell<TabStrips>>,
}

struct Permission {
    id: String,
    question: String,
    symbol: &'static str,
}

fn permission_kind(kind: &str) -> (&'static str, Option<&'static str>, Option<&'static str>) {
    let (symbol, noun) = match kind {
        "camera" => ("video", "camera"),
        "microphone" => ("mic", "microphone"),
        "cameraPanTiltZoom" => ("video", "camera's pan, tilt and zoom"),
        "location" => ("location", "location"),
        "clipboard" => ("doc.on.clipboard", "clipboard"),
        "midi" => ("pianokeys", "MIDI devices"),
        "localFonts" => ("textformat", "fonts"),
        "windowManagement" => (
            "macwindow.on.rectangle",
            "displays to open and place windows",
        ),
        "fileSystem" => ("folder", "files"),
        "keyboardLock" => ("keyboard", "keyboard"),
        "pointerLock" => ("cursorarrow", "mouse pointer"),
        "sensors" => ("gyroscope", "motion sensors"),
        "localNetwork" => ("network", "devices on your local network"),
        "vr" => ("visionpro", "virtual reality devices"),
        "ar" => ("visionpro", "augmented reality data"),
        "handTracking" => ("hand.raised", "hand tracking"),
        "screen" => {
            return (
                "rectangle.on.rectangle",
                None,
                Some("Allow %@ to share your screen?"),
            );
        }
        "notifications" => {
            return ("bell", None, Some("Allow %@ to send you notifications?"));
        }
        "multipleDownloads" => {
            return (
                "arrow.down.circle",
                None,
                Some("Allow %@ to download multiple files?"),
            );
        }
        "idleDetection" => {
            return (
                "person.crop.circle.badge.clock",
                None,
                Some("Allow %@ to know when you're actively using this device?"),
            );
        }
        "storageAccess" => {
            return (
                "externaldrive",
                None,
                Some("Allow %@ to use cookies and site data while embedded?"),
            );
        }
        "diskQuota" => {
            return (
                "internaldrive",
                None,
                Some("Allow %@ to store files on your device?"),
            );
        }
        "protocolHandler" => {
            return ("envelope", None, Some("Make %@ your default mail app?"));
        }
        "identityProvider" => {
            return (
                "person.badge.key",
                None,
                Some("Allow %@ to sign you in with an identity provider?"),
            );
        }
        "webAppInstallation" => {
            return ("app.badge", None, Some("Allow %@ to install web apps?"));
        }
        "capturedSurfaceControl" => {
            return (
                "rectangle.on.rectangle",
                None,
                Some("Allow %@ to scroll and zoom the tab you share?"),
            );
        }
        _ => return ("questionmark.circle", None, None),
    };
    (symbol, Some(noun), None)
}

fn permission_prompt(request: &Value) -> (String, &'static str) {
    let origin = request["origin"].as_str().unwrap_or("");
    let host = NSURL::URLWithString(&NSString::from_str(origin))
        .and_then(|url| url.host())
        .map(|host| host.to_string())
        .unwrap_or_default();
    let site = host.strip_prefix("www.").unwrap_or(&host);
    let site = if site.is_empty() { origin } else { site };
    let mut kinds = request["permissions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|kind| (kind, permission_kind(kind)))
        .peekable();
    let symbol = kinds
        .peek()
        .map_or("questionmark.circle", |(_, (symbol, _, _))| *symbol);
    let mut nouns = Vec::new();
    let mut access = true;
    if let Some((kind, (_, noun, question))) = kinds.next() {
        if let Some(question) = question
            && kinds.peek().is_none()
        {
            return (question.replacen("%@", site, 1), symbol);
        }
        access = question.is_none() && !kind.is_empty();
        nouns.push(noun.unwrap_or(kind));
    }
    for (kind, (_, noun, question)) in kinds {
        access &= question.is_none() && !kind.is_empty();
        nouns.push(noun.unwrap_or(kind));
    }
    let verb = if access { "access your" } else { "use" };
    let question = match nouns.split_last() {
        Some((last, rest)) if !rest.is_empty() => {
            format!("Allow {site} to {verb} {} and {last}?", rest.join(", "))
        }
        Some((last, _)) => format!("Allow {site} to {verb} {last}?"),
        None => format!("Allow {site} to {verb} ?"),
    };
    (question, symbol)
}

struct Popup {
    owner: Weak<Popups>,
    core: Retained<Window>,
    tab: RefCell<Option<Retained<Tab>>>,
    opener: Option<ObjcWeak<NSWindow>>,
    opener_fullscreen: bool,
    strip: RefCell<Option<Retained<PopupOriginStrip>>>,
    delegate: Retained<PopupDelegate>,
    observer: Retained<AnyObject>,
    adopting: Cell<bool>,
    show_inactive: Cell<bool>,
    permissions: RefCell<VecDeque<Permission>>,
    sheet: RefCell<Option<(String, Retained<NSAlert>)>>,
}

impl Popups {
    pub fn new(
        mtm: MainThreadMarker,
        background: bool,
        strips: Rc<RefCell<TabStrips>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            mtm,
            background,
            windows: RefCell::new(Vec::new()),
            strips,
        })
    }

    pub fn count(&self) -> usize {
        self.windows.borrow().len()
    }

    /// The core windows of the open popups.
    pub fn windows(&self) -> Vec<Retained<Window>> {
        self.windows
            .borrow()
            .iter()
            .map(|popup| popup.core.clone())
            .collect()
    }

    pub fn ask_permission(&self, tab: &Tab, request: &Value) -> bool {
        let Some(id) = request["id"].as_str() else {
            return false;
        };
        let popup = self
            .windows
            .borrow()
            .iter()
            .find(|popup| {
                popup
                    .tab
                    .borrow()
                    .as_ref()
                    .is_some_and(|item| std::ptr::eq(&**item, tab))
            })
            .cloned();
        let Some(popup) = popup else { return false };
        let (question, symbol) = permission_prompt(request);
        popup.permissions.borrow_mut().push_back(Permission {
            id: id.into(),
            question,
            symbol,
        });
        popup.show_permission();
        true
    }

    pub fn drop_permission(&self, id: &str) {
        for popup in self.windows.borrow().clone() {
            let queued = popup
                .permissions
                .borrow()
                .iter()
                .position(|request| request.id == id);
            if let Some(index) = queued {
                popup.permissions.borrow_mut().remove(index);
                break;
            }
            let showing = popup
                .sheet
                .borrow()
                .as_ref()
                .is_some_and(|(request, _)| request == id);
            if showing {
                popup.dismiss_permission();
                popup.show_permission();
                break;
            }
        }
    }

    pub fn open(
        self: &Rc<Self>,
        tab: &Tab,
        opener: Option<&NSWindow>,
        opener_fullscreen: bool,
    ) -> Retained<Window> {
        let popup = self.create(Some(tab), opener, opener_fullscreen);
        let weak = Rc::downgrade(&popup);
        main_queue::post(Box::new(move || {
            if let Some(popup) = weak.upgrade() {
                popup.take_tab();
            }
        }));
        popup.core.clone()
    }

    pub fn for_chrome(
        self: &Rc<Self>,
        opener: Option<&NSWindow>,
        opener_fullscreen: bool,
    ) -> Retained<Window> {
        let popup = self.create(None, opener, opener_fullscreen);
        let weak = Rc::downgrade(&popup);
        main_queue::after(
            Duration::from_secs(20),
            Box::new(move || {
                if let Some(popup) = weak.upgrade()
                    && popup.tab.borrow().is_none()
                {
                    popup.core.close();
                }
            }),
        );
        popup.core.clone()
    }

    fn create(
        self: &Rc<Self>,
        tab: Option<&Tab>,
        opener: Option<&NSWindow>,
        opener_fullscreen: bool,
    ) -> Rc<Popup> {
        let features = tab.and_then(Tab::popup_features);
        let feature = |key| {
            features
                .as_ref()?
                .objectForKey(key)
                .map(|n| n.doubleValue())
                .filter(|n| n.is_finite())
        };
        let size = NSSize::new(
            feature(ns_string!("width")).unwrap_or(500.).max(200.),
            feature(ns_string!("height")).unwrap_or(600.).max(150.) + STRIP_HEIGHT,
        );
        // SAFETY: The engine is loaded and the window is allocated on the main thread.
        let core = unsafe {
            Window::init_with_content_rect(self.mtm.alloc(), NSRect::new(NSPoint::ZERO, size))
        };
        let window = core.window();
        window.setStyleMask(
            window.styleMask()
                | NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::Resizable,
        );
        // SAFETY: The popup retains the engine window until the close notification.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTabbingMode(NSWindowTabbingMode::Disallowed);
        window.setMinSize(NSSize::new(200., 150.));
        window.setContentSize(size);
        place(
            &window,
            opener,
            self.mtm,
            feature(ns_string!("x")),
            feature(ns_string!("y")),
        );
        let popup = Rc::new_cyclic(|weak: &Weak<Popup>| {
            let delegate = PopupDelegate::new(weak.clone());
            // SAFETY: The popup owns a delegate that implements the used window selectors.
            unsafe { core.set_delegate(Some(&delegate)) };
            let closing = weak.clone();
            let callback = RcBlock::new(move |_: NonNull<NSNotification>| {
                if let Some(popup) = closing.upgrade() {
                    popup.closed();
                }
            });
            // SAFETY: This callback filters on the live main-thread popup window.
            let observer = unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(NSWindowWillCloseNotification),
                    Some(&window),
                    None,
                    &callback,
                )
            };
            Popup {
                owner: Rc::downgrade(self),
                core,
                tab: RefCell::new(tab.map(Tab::retain)),
                opener: opener.map(ObjcWeak::from),
                opener_fullscreen,
                strip: RefCell::new(None),
                delegate,
                observer: observer.into(),
                adopting: Cell::new(false),
                show_inactive: Cell::new(false),
                permissions: RefCell::default(),
                sheet: RefCell::default(),
            }
        });
        self.windows.borrow_mut().push(popup.clone());
        popup
    }
}

fn place(
    window: &NSWindow,
    parent: Option<&NSWindow>,
    mtm: MainThreadMarker,
    x: Option<f64>,
    y: Option<f64>,
) {
    let screen = parent
        .and_then(NSWindow::screen)
        .or_else(|| NSScreen::mainScreen(mtm));
    let visible = screen.as_ref().map(|screen| screen.visibleFrame());
    let parent = parent.map_or(NSRect::ZERO, NSWindow::frame);
    let parent_top = parent.origin.y + parent.size.height;
    let mut frame = window.frame();
    if let Some(visible) = visible {
        frame.size.width = frame.size.width.min(visible.size.width);
        frame.size.height = frame.size.height.min(visible.size.height);
    }
    let left = parent.origin.x + (parent.size.width - frame.size.width) / 2.;
    if x.is_some() || y.is_some() {
        let primary = NSScreen::screens(mtm)
            .firstObject()
            .map_or(NSRect::ZERO, |screen| screen.frame());
        let top = primary.origin.y + primary.size.height;
        let content = window.contentRectForFrameRect(frame);
        frame.origin = NSPoint::new(
            x.unwrap_or(left),
            top - y.unwrap_or(top - parent_top + 60.) - content.size.height,
        );
    } else if parent != NSRect::ZERO {
        frame.origin = NSPoint::new(left, parent_top - frame.size.height - 60.);
    } else {
        window.center();
        frame.origin = window.frame().origin;
    }
    if let Some(visible) = visible {
        frame.origin.x = frame
            .origin
            .x
            .min(visible.origin.x + visible.size.width - frame.size.width)
            .max(visible.origin.x);
        frame.origin.y = frame
            .origin
            .y
            .min(visible.origin.y + visible.size.height - frame.size.height)
            .max(visible.origin.y);
    }
    window.setFrame_display(frame, false);
}

impl Popup {
    fn show_permission(self: &Rc<Self>) {
        if self.sheet.borrow().is_some() {
            return;
        }
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        let Some(request) = self.permissions.borrow_mut().pop_front() else {
            return;
        };
        let alert = NSAlert::new(owner.mtm);
        alert.setMessageText(&NSString::from_str(&request.question));
        let config = NSImageSymbolConfiguration::configurationWithPointSize_weight(32., unsafe {
            NSFontWeightRegular
        });
        if let Some(icon) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str(request.symbol),
            None,
        )
        .and_then(|icon| icon.imageWithSymbolConfiguration(&config))
        {
            unsafe { alert.setIcon(Some(&icon)) };
        }
        alert.addButtonWithTitle(ns_string!("Allow"));
        alert
            .addButtonWithTitle(ns_string!("Don't Allow"))
            .setKeyEquivalent(ns_string!("\u{1b}"));
        self.sheet
            .replace(Some((request.id.clone(), alert.clone())));
        let weak = Rc::downgrade(self);
        let callback = RcBlock::new(move |response: NSModalResponse| {
            let Some(popup) = weak.upgrade() else { return };
            if !popup
                .sheet
                .borrow()
                .as_ref()
                .is_some_and(|(id, _)| id == &request.id)
            {
                return;
            }
            popup.sheet.take();
            if response == NSAlertFirstButtonReturn {
                resolve_permission(&request.id, "accept", true);
            } else if response == NSAlertSecondButtonReturn {
                resolve_permission(&request.id, "deny", true);
            }
            popup.show_permission();
        });
        alert.beginSheetModalForWindow_completionHandler(&self.core.window(), Some(&callback));
    }

    fn dismiss_permission(&self) {
        if let Some((_, alert)) = self.sheet.take() {
            let sheet = alert.window();
            if let Some(parent) = sheet.sheetParent() {
                parent.endSheet_returnCode(&sheet, NSModalResponseAbort);
            }
        }
    }

    fn closed(self: &Rc<Self>) {
        self.permissions.borrow_mut().clear();
        self.dismiss_permission();
        if let Some(tab) = self.tab.take() {
            self.detach_tab_delegate(&tab);
        }
        let closed = self.clone();
        main_queue::post(Box::new(move || {
            if let Some(owner) = closed.owner.upgrade() {
                owner
                    .windows
                    .borrow_mut()
                    .retain(|item| !Rc::ptr_eq(item, &closed));
            }
        }));
    }

    fn detach_tab_delegate(&self, tab: &Tab) {
        let owned: &AnyObject = &self.delegate;
        // SAFETY: Tab's delegate getter returns a retained weak reference when a delegate exists.
        let attached: Option<Retained<AnyObject>> = unsafe { msg_send![tab, delegate] };
        if attached
            .as_deref()
            .is_some_and(|delegate| std::ptr::eq(delegate, owned))
        {
            // SAFETY: This popup is still the tab's current delegate on the main thread.
            unsafe { tab.set_delegate(None) };
        }
    }

    fn take_tab(self: &Rc<Self>) {
        let Some(tab) = self.tab.borrow().clone().filter(|tab| !tab.closed()) else {
            self.core.close();
            return;
        };
        self.adopting.set(true);
        let result = self.core.adopt_tab(&tab);
        self.adopting.set(false);
        match result {
            Adoption::Refused => self.core.close(),
            Adoption::Adopted => self.show_tab(),
            Adoption::Busy => {
                let weak = Rc::downgrade(self);
                main_queue::post(Box::new(move || {
                    if let Some(popup) = weak.upgrade() {
                        popup.take_tab();
                    }
                }));
            }
        }
    }

    fn show_tab(&self) {
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        let Some(tab) = self.tab.borrow().clone().filter(|tab| !tab.closed()) else {
            self.core.close();
            return;
        };
        self.core.set_active_profile(tab.profile().as_deref());
        // SAFETY: The popup owns the delegate, which implements the used tab selectors.
        unsafe { tab.set_delegate(Some(&self.delegate)) };
        let host = self.core.host_view();
        let Some(page) = tab.view() else {
            self.core.close();
            return;
        };
        page.removeFromSuperview();
        let size = host.bounds().size;
        let height = (size.height - STRIP_HEIGHT).max(0.);
        let flipped = host.isFlipped();
        page.setFrame(NSRect::new(
            NSPoint::new(0., if flipped { STRIP_HEIGHT } else { 0. }),
            NSSize::new(size.width, height),
        ));
        page.setAutoresizingMask(Mask::ViewWidthSizable | Mask::ViewHeightSizable);
        tab.note_in_shown_slot();
        host.addSubview(&page);
        let strip = PopupOriginStrip::new(
            owner.mtm,
            NSRect::new(
                NSPoint::new(0., if flipped { 0. } else { height }),
                NSSize::new(size.width, STRIP_HEIGHT),
            ),
        );
        strip.setAutoresizingMask(
            Mask::ViewWidthSizable
                | if flipped {
                    Mask::ViewMaxYMargin
                } else {
                    Mask::ViewMinYMargin
                },
        );
        host.addSubview_positioned_relativeTo(&strip, NSWindowOrderingMode::Above, Some(&page));
        self.strip.replace(Some(strip));
        self.update();
        let window = self.core.window();
        if self.opener_fullscreen
            || self
                .opener
                .as_ref()
                .and_then(ObjcWeak::load)
                .is_some_and(|opener| opener.styleMask().contains(NSWindowStyleMask::FullScreen))
        {
            window.setCollectionBehavior(
                (window.collectionBehavior() - NSWindowCollectionBehavior::FullScreenPrimary)
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
        }
        if owner.background || self.show_inactive.get() {
            self.core.show_inactive();
        } else {
            window.makeKeyAndOrderFront(None);
        }
        tab.focus();
    }

    fn update(&self) {
        let Some(tab) = self.tab.borrow().clone() else {
            return;
        };
        let url = tab.url();
        let title = tab.title();
        let url_text = url.to_string();
        if title.is_empty() || title.compare(&url) == NSComparisonResult::Same {
            self.core
                .window()
                .setTitle(&NSString::from_str(&popup_origin_text(&url_text)));
        } else {
            self.core.window().setTitle(&title);
        }
        if let Some(strip) = self.strip.borrow().as_ref() {
            let security = to_json(&tab.security_info());
            strip.show_url(&url_text, security["level"].as_str());
        }
    }
}

impl Drop for Popup {
    fn drop(&mut self) {
        // SAFETY: The center issued this token and the delegate is still alive.
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(&self.observer);
            self.core.set_delegate(None);
        }
        if let Some(tab) = self.tab.take() {
            self.detach_tab_delegate(&tab);
        }
    }
}

struct PopupIvars(Weak<Popup>);

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "RiaPopupDelegate"]
    #[ivars = PopupIvars]
    struct PopupDelegate;

    impl PopupDelegate {
        #[unsafe(method(tabDidChangeTitle:))]
        fn title_changed(&self, _tab: &Tab) { self.changed(); }

        #[unsafe(method(tabDidChangeURL:))]
        fn url_changed(&self, _tab: &Tab) { self.changed(); }

        #[unsafe(method(tabDidChangeSecurity:))]
        fn security_changed(&self, _tab: &Tab) { self.changed(); }

        #[unsafe(method(tab:requestsActivation:))]
        fn activation(&self, _tab: &Tab, _reason: &NSString) {
            if let Some(popup) = self.ivars().0.upgrade()
                && popup.owner.upgrade().is_some_and(|owner| !owner.background)
            {
                popup.core.window().makeKeyAndOrderFront(None);
            }
        }

        #[unsafe(method(window:showRequestedActive:))]
        fn show_requested(&self, _window: &Window, active: bool) {
            if let Some(popup) = self.ivars().0.upgrade() && popup.strip.borrow().is_none() {
                popup.show_inactive.set(!active);
            }
        }

        #[unsafe(method(window:didInsertTab:opener:disposition:))]
        fn inserted(&self, _window: &Window, tab: &Tab, _opener: Option<&Tab>, disposition: &NSString) {
            let Some(popup) = self.ivars().0.upgrade() else { return };
            if popup.adopting.get() { return; }
            let mut current = popup.tab.borrow_mut();
            if current.as_ref().is_some_and(|item| std::ptr::eq(&**item, tab)) { return; }
            if current.is_none() {
                *current = Some(tab.retain());
                drop(current);
                let weak = Rc::downgrade(&popup);
                main_queue::post(Box::new(move || {
                    if let Some(popup) = weak.upgrade() {
                        popup.show_tab();
                    }
                }));
                return;
            }
            drop(current);
            if let Some(owner) = popup.owner.upgrade()
                && disposition.compare(ns_string!("popup")) == NSComparisonResult::Same
            {
                owner.open(tab, Some(&popup.core.window()), false);
            }
        }

        #[unsafe(method(window:didRemoveTab:))]
        fn removed(&self, window: &Window, tab: &Tab) {
            let Some(popup) = self.ivars().0.upgrade() else { return };
            if let Some(owner) = popup.owner.upgrade() {
                let strip = tab.profile().map_or(-1, |profile| window.chrome_window_id_for_profile(&profile));
                owner.strips.borrow_mut().removed(strip, tab);
            }
            if popup.tab.borrow().as_ref().is_some_and(|item| std::ptr::eq(&**item, tab)) {
                let weak = Rc::downgrade(&popup);
                main_queue::post(Box::new(move || {
                    if let Some(popup) = weak.upgrade() { popup.core.close(); }
                }));
            }
        }

        #[unsafe(method(window:preHandleKeyEvent:))]
        fn pre_handle_key(&self, _window: &Window, event: &NSEvent) -> objc2::runtime::Bool {
            let Some(popup) = self.ivars().0.upgrade() else { return objc2::runtime::Bool::NO };
            let Some(owner) = popup.owner.upgrade() else { return objc2::runtime::Bool::NO };
            let app = NSApplication::sharedApplication(owner.mtm);
            let Some(item) = reserved_menu_item(&app, event, KeyContext::Windowed) else { return objc2::runtime::Bool::NO };
            let handled = if item.identifier().is_some_and(|id| id.compare(ns_string!("ardia.command.closeTab")) == NSComparisonResult::Same) {
                popup.core.close();
                true
            } else {
                app.mainMenu().is_some_and(|menu| menu.performKeyEquivalent(event))
            };
            handled.into()
        }
    }
);

impl PopupDelegate {
    fn new(popup: Weak<Popup>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(PopupIvars(popup));
        // SAFETY: NSObject receives an allocated instance with initialized Rust ivars.
        unsafe { msg_send![super(this), init] }
    }

    fn changed(&self) {
        if let Some(popup) = self.ivars().0.upgrade() {
            popup.update();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::permission_prompt;
    use serde_json::json;

    #[test]
    fn permission_prompts_use_site_specific_words_and_the_first_symbol() {
        for (permissions, question, symbol) in [
            (
                vec!["capturedSurfaceControl"],
                "Allow example.test to scroll and zoom the tab you share?",
                "rectangle.on.rectangle",
            ),
            (
                vec!["camera", "microphone", "location"],
                "Allow example.test to access your camera, microphone and location?",
                "video",
            ),
            (
                vec!["notifications", "futureKind"],
                "Allow example.test to use notifications and futureKind?",
                "bell",
            ),
        ] {
            let request = json!({
                "origin": "https://www.example.test:8443/room",
                "permissions": permissions,
            });
            assert_eq!(permission_prompt(&request), (question.into(), symbol));
        }
    }
}
