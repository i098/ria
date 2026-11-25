// Ria original, no Ardia source.

//! The window host of the app: the one `ChromeWindowHost`, its shared `TabStrips`, and the `Popups` built on them.
//!
//! Ardia keeps these in process-wide statics (`ArdiaTabStrip.mm`, `ArdiaPopupWindow.mm`) that `ArdiaHostEngineDelegate.mm:122,204,212`
//! and `TabsModule.swift:11` reach by class method. Ria keeps one value and routes the same entries through it.
//!
//! Changes from Ardia:
//! - Ardia reaches `ArdiaTabStrip.command` from the page layer (`TabsModule.swift`). The app has no page layer yet, so
//!   [`WindowHost::strip_command`] is the entry that routes `tabs.stripCommand` into the shared `TabStrips` (issue #153).
//! - The strip scopes are the browser window of the launch and the open popups, not every `ArdiaWindowController` (issue #153).

use std::cell::RefCell;
use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use ria_engine::types::{Number, TabStripCommand};
use serde_json::Value;

use crate::core::bindings::{Tab, Window};
use crate::core::host::State;
use crate::diagnostics::ardia_paint_probe::PaintProbe;
use crate::tabs::ardia_page_hooks::PageHooks;
use crate::tabs::ardia_tab_strip::{StripCommandResult, StripWindow, TabStrips};
use crate::tabs::ardia_web_view::{ArdiaWebView, LiveViews};
use crate::tabs::chromium_tab_view::ChromiumTabView;
use crate::windows::ardia_chrome_window_host::ChromeWindowHost;
use crate::windows::ardia_popup_window::Popups;
use crate::windows::ardia_window_controller::Controllers;

pub struct WindowHost {
    chrome: ChromeWindowHost,
    popups: Rc<Popups>,
    main: RefCell<Option<Retained<Window>>>,
    hooks: Rc<PageHooks>,
    paint_probe: Rc<PaintProbe>,
    /// The page views of the tabs; the registry and `LiveViews` hold them weakly.
    pages: RefCell<Vec<Retained<ArdiaWebView>>>,
}

impl WindowHost {
    /// `emit` takes the events of the paint probe.
    pub fn new(
        mtm: MainThreadMarker,
        state: Rc<State>,
        background: bool,
        emit: impl Fn(&str, Value) + 'static,
    ) -> Rc<Self> {
        let controllers = Controllers::new(mtm, background, false, |_| true);
        let chrome = ChromeWindowHost::new(mtm, state, controllers.clone());
        let popups = Popups::new(mtm, background, chrome.strips().clone());
        Rc::new_cyclic(|this: &std::rc::Weak<Self>| {
            let weak = this.clone();
            let hooks = PageHooks::new(
                Rc::new(LiveViews::default()),
                move |window| weak.upgrade()?.chrome.root_view(window),
                |_| {},
            );
            // `make_page` installs the hooks: guarding `NSApplication` here would make the plain class before the engine's own.
            // shortcut: no consent epoch exists in the app yet, so the probe reports with an empty one.
            let paint_probe = hooks.paint_probe(controllers, emit, String::new);
            paint_probe.watch();
            Self {
                chrome,
                popups,
                main: RefCell::default(),
                hooks,
                paint_probe,
                pages: RefCell::default(),
            }
        })
    }

    pub fn hooks(&self) -> &Rc<PageHooks> {
        &self.hooks
    }

    /// The `PaintProbeLine` of the focus summary.
    pub fn paint_line(&self) -> String {
        self.paint_probe.line()
    }

    /// The page view of `tab`: the one the registry holds, or a new `ArdiaWebView` that keys the tab as `key`.
    pub fn page_for(&self, tab: &Tab, key: &str) -> Option<Retained<NSView>> {
        let known = self.chrome.tab_registry().borrow().view_for_tab(tab);
        if let Some(view) = known {
            return Some(view);
        }
        let mtm = MainThreadMarker::new()?;
        let page = ChromiumTabView::make_page(mtm, &self.hooks);
        page.attach_tab(tab.retain(), key.to_owned(), self.chrome.tab_registry());
        let mut pages = self.pages.borrow_mut();
        pages.retain(|page| !page.tab_closed());
        pages.push(page.clone());
        Some(Retained::into_super(page))
    }

    /// Release the page views, before the engine shuts down.
    pub fn drop_pages(&self) {
        self.pages.borrow_mut().clear();
    }

    pub fn chrome(&self) -> &ChromeWindowHost {
        &self.chrome
    }

    pub fn popups(&self) -> &Rc<Popups> {
        &self.popups
    }

    pub fn strips(&self) -> &Rc<RefCell<TabStrips>> {
        self.chrome.strips()
    }

    /// Record the browser window: it opens popups and is the scope of strip commands.
    pub fn set_main(&self, window: Option<Retained<Window>>) {
        *self.main.borrow_mut() = window;
    }

    /// The browser window, once the launch recorded it.
    pub fn main(&self) -> Option<Retained<Window>> {
        self.main.borrow().clone()
    }

    /// `ardia_host::PopupWindowForChromeBrowser`.
    pub fn popup_for_chrome(&self) -> Retained<Window> {
        let main = self.main.borrow();
        let opener = main.as_ref().map(|window| window.window());
        self.popups.for_chrome(opener.as_deref(), false)
    }

    /// `ardia_host::AskPermissionInPopupWindow`.
    pub fn ask_permission(&self, tab: &Tab, request: &Value) -> bool {
        self.popups.ask_permission(tab, request)
    }

    /// `ardia_host::DropPopupWindowPermission`.
    pub fn drop_permission(&self, id: &str) {
        self.popups.drop_permission(id);
    }

    /// `tabs.stripCommand`: run `command` on the strip that matches its ID, in the browser window or a popup.
    /// A window without an active profile has no strip.
    pub fn strip_command(&self, number: Number, command: TabStripCommand) -> StripCommandResult {
        let windows: Vec<_> = self
            .main
            .borrow()
            .iter()
            .cloned()
            .chain(self.popups.windows())
            .filter_map(|window| {
                let profile = window.active_profile()?;
                let name = profile.name().to_string();
                Some((window, profile, name))
            })
            .collect();
        let scopes: Vec<_> = windows
            .iter()
            .map(|(window, profile, name)| StripWindow {
                window,
                profile,
                profile_name: name,
                app_window: None,
                host_changes: false,
                standalone: false,
            })
            .collect();
        let mut registry = self.chrome.tab_registry().borrow_mut();
        self.strips()
            .borrow_mut()
            .command(number, command, &scopes, &mut registry)
    }
}
