// Ported from ardia apps/browser/src/state/tabs/chromeTabs.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The Rune tab strip layer of the browser window (issues #101, #153).
//!
//! Ardia draws its tab strip in the page layer (`apps/browser/src/state/tabs/chromeTabs.ts`) and sends an Arrange command
//! with `sendTabStripCommand` (`chromeTabs.ts:159`, `send` at `:176-179`; `packages/ardiacore/src/tabs.ts:54-56`).
//! `TabsModule.swift:43-44` hands it to `ArdiaTabStrip.command`. Ria draws the strip as a Rune layer around the native
//! page (`ria-surface`) and sends the same Arrange command to [`WindowHost::strip_command`].
//!
//! Changes from Ardia:
//! - The strip shows the launch tab only: one tab button and one pin button. Ardia projects every tab of the strip.
//!   Showing the other tabs needs the `tabs.strip` transaction in this layer (issue #101 stays open for it).
//! - A press on the pin button sends `arrange` with `pinned` 1 or 0 for the launch tab, and the reply sets the button.
//! - A restored window layout keeps its cover and cached picture. `ria-surface` `Surface::set_page` always moves the page
//!   from the cover into the `page` slot below the strip, so the restored page rect is not kept. The cover stays behind
//!   the page as a placeholder. Keeping the page in its cover needs a `ria-surface` call that leaves the page's parent.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ria_engine::call::{ActionQueue, main_queue};
use ria_engine::types::{Number, TabStripCommand, TabStripTransaction};
use ria_surface::{WindowHost as Surface, WindowSpec};
use rune::tok::{self, type_};
use rune::{AppSpec, Color, Cx, El, col, el, row, text};

use crate::core::bindings::{Tab, Window};
use crate::tabs::ardia_tab_strip::StripCommandResult;

use super::window_host::WindowHost;

/// The tab key of the launch tab.
pub const LAUNCH_KEY: &str = "launch";

const HEIGHT: f32 = 44.0;

/// What the chrome layer draws and sends.
#[derive(Default)]
pub struct StripState {
    strip: i32,
    pinned: bool,
    send: Option<Box<dyn Fn(TabStripCommand)>>,
}

type Shared = Rc<RefCell<StripState>>;

fn chrome(state: &Shared, cx: &mut Cx<Shared>) -> El {
    let theme = tok::theme(cx.system_dark());
    let pinned = state.borrow().pinned;
    let tab = el()
        .key("tab")
        .role("tab")
        .label("Launch tab")
        .h(28.0)
        .w(140.0)
        .radius(8.0)
        .fill(theme.control_off)
        .px(12.0)
        .center_y()
        .child(text("Tab").font(type_::LABEL).color(theme.text));
    let pin = pin_button(pinned, cx.system_dark());
    col()
        .w_full()
        .h_full()
        .fill(Color::TRANSPARENT)
        .child(
            row()
                .w_full()
                .h(HEIGHT)
                .px(12.0)
                .gap(8.0)
                .center_y()
                .child(tab)
                .child(pin),
        )
        .child(el().key("page").w_full().grow(1.0))
}

fn pin_button(pinned: bool, dark: bool) -> El {
    let theme = tok::theme(dark);
    el().key("pin")
        .role("button")
        .label(if pinned { "Unpin tab" } else { "Pin tab" })
        .focusable(true)
        .h(28.0)
        .w(84.0)
        .radius(8.0)
        .fill(if pinned {
            tok::accent(0, dark)
        } else {
            theme.control_off
        })
        .center()
        .child(
            text(if pinned { "Pinned" } else { "Pin" })
                .font(type_::LABEL)
                .color(theme.text),
        )
        .on_press(|state: &mut Shared| {
            let state = state.borrow();
            if let Some(send) = &state.send {
                send(TabStripCommand::Arrange {
                    strip: Number(f64::from(state.strip)),
                    keys: vec![LAUNCH_KEY.into()],
                    pinned: Number(if state.pinned { 0.0 } else { 1.0 }),
                });
            }
        })
}

fn overlay(_: &Shared, _: &mut Cx<Shared>) -> El {
    col().w_full().h_full()
}

/// The strip layer and the surface it owns. Drop it before the engine shuts down.
pub struct StripLayer {
    /// Keeps the surface alive until drop.
    _surface: Rc<Surface<StripState>>,
}

impl StripLayer {
    /// Key the launch tab, host the Rune layers in the window, and show the page inside them.
    pub fn install(host: &Rc<WindowHost>, core: &Window, tab: &Tab) -> Result<Self, String> {
        let profile = core.active_profile().ok_or("the window has no profile")?;
        let page = tab.view().ok_or("the launch tab has no view")?;
        host.chrome()
            .tab_registry()
            .borrow_mut()
            .set_view(tab, Some((&page, LAUNCH_KEY.into())));
        let state = Rc::new(RefCell::new(StripState {
            strip: core.chrome_window_id_for_profile(&profile),
            pinned: false,
            send: None,
        }));
        let queue = ActionQueue::new();
        let weak_queue = Rc::downgrade(&queue);
        queue.set_waker(move || {
            let queue = weak_queue.clone();
            main_queue::post(Box::new(move || {
                if let Some(queue) = queue.upgrade() {
                    queue.drain();
                }
            }));
        });
        let spec = WindowSpec {
            chrome: AppSpec::new("Ria strip", (1360.0, 860.0), Shared::default, chrome),
            overlay: AppSpec::new("Ria overlay", (1360.0, 860.0), Shared::default, overlay),
            page_slot: "page",
            text_snapshot: |_| None,
        };
        let surface = Rc::new(Surface::new(&core.host_view(), state.clone(), spec, queue)?);
        surface.set_page(Some(&page));
        let send = sender(
            Rc::downgrade(host),
            Rc::downgrade(&surface),
            Rc::downgrade(&state),
        );
        state.borrow_mut().send = Some(send);
        Ok(Self { _surface: surface })
    }
}

/// Run a command from the layer on a later main-queue turn, then show the pin state that the strip reports.
fn sender(
    host: Weak<WindowHost>,
    surface: Weak<Surface<StripState>>,
    state: Weak<RefCell<StripState>>,
) -> Box<dyn Fn(TabStripCommand)> {
    Box::new(move |command| {
        let (host, surface, state) = (host.clone(), surface.clone(), state.clone());
        main_queue::post(Box::new(move || {
            let (Some(host), Some(surface), Some(state)) =
                (host.upgrade(), surface.upgrade(), state.upgrade())
            else {
                return;
            };
            let StripCommandResult::Done(transaction) = host.strip_command(Number(1.0), command)
            else {
                return;
            };
            state.borrow_mut().pinned = launch_pinned(&transaction);
            surface.mark_dirty(true, false);
        }));
    })
}

/// True when the transaction reports the launch tab as pinned.
fn launch_pinned(transaction: &TabStripTransaction) -> bool {
    transaction
        .strips
        .iter()
        .flat_map(|strip| &strip.tabs)
        .any(|tab| tab.key.as_deref() == Some(LAUNCH_KEY) && tab.pinned)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn transaction(pinned: bool) -> TabStripTransaction {
        serde_json::from_value(serde_json::json!({
            "rev": 1, "cmd": 1,
            "strips": [{
                "strip": 1, "window": 1, "profile": "Default",
                "tabs": [{"key": LAUNCH_KEY, "browser": 1, "index": 0, "active": true, "pinned": pinned}]
            }]
        }))
        .unwrap()
    }

    #[test]
    fn the_pin_follows_the_strip_reply() {
        assert!(launch_pinned(&transaction(true)));
        assert!(!launch_pinned(&transaction(false)));
    }
}
