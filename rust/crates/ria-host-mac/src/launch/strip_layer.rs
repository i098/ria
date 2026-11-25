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
//! - The strip projects every tab of the browser window (issue #101). Ardia's strip is a projection of the `tabs.strip`
//!   transaction (`chromeTabs.ts:87-93`, `TabStrips.snapshot`). Ria reads the same tab list, activity and pin record from
//!   the window and [`TabStrips`](crate::tabs::ardia_tab_strip::TabStrips), polls it every [`POLL`], and redraws on a change.
//!   Ardia pushes a transaction on each change; the host has no change event for tabs opened outside the strip yet.
//! - A tab that has no strip key gets `tab:<browser id>` and its own page as its registry view, as the launch tab gets
//!   `launch`. Ardia's page layer chooses its own keys.
//! - A press on a tab sends `activate`; a press on its pin button sends `arrange` with the pinned tabs first
//!   (`chromeTabs.ts:143-159`), and the reply sets the button. A press on its close button closes the tab. Ardia closes
//!   through `tabs.close`; Ria calls `Tab::close`. The last tab has no effect, so the window keeps a page.
//! - The active tab is tracked in the layer: the first read marks the first tab, a press sets it, and it stays until a
//!   press or a close changes it. Known limit: the engine has no active-tab getter that Ria fills. `TabStrips::active`
//!   is set only by `TabStrips::activated`, which no Ria code calls, and `Window::activate_tab` keeps no record. A tab that
//!   becomes active outside the strip, such as a foreground open, does not move the active mark until it is pressed.
//! - Changes from Ardia (#110): every tab's page stays mounted in the page slot, not hidden; a page view paints only while it is visible, and it takes no clicks then, as in Ardia's sibling panes (`Surface::mount_page`).
//! - The page slot follows the active tab through `Surface::set_page`; Ardia shows each tab's page in its own page layer.
//! - A restored window layout keeps its cover and cached picture. `ria-surface` `Surface::set_page` always moves the page
//!   from the cover into the `page` slot below the strip, so the restored page rect is not kept. The cover stays behind
//!   the page as a placeholder. Keeping the page in its cover needs a `ria-surface` call that leaves the page's parent.

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use ria_engine::call::{ActionQueue, main_queue};
use ria_engine::types::{Number, TabStripCommand};
use ria_surface::{WindowHost as Surface, WindowSpec};
use rune::tok::{self, type_};
use rune::{AppSpec, Color, Cx, El, col, el, row, text};

use crate::core::bindings::{Tab, Window};
use crate::tabs::ardia_tab_strip::StripCommandResult;

use super::host_launch::after;
use super::window_host::WindowHost;

/// The tab key of the launch tab.
pub const LAUNCH_KEY: &str = "launch";

/// How often the layer reads the tab list of the window.
const POLL: Duration = Duration::from_millis(500);

const HEIGHT: f32 = 44.0;
const TITLE_BAR: f64 = 40.0;
const PAD: f64 = 12.0;
const GAP: f64 = 8.0;
const INNER_GAP: f64 = 4.0;
const TAB_W: f64 = 112.0;
const PIN_W: f64 = 40.0;
const CLOSE_W: f64 = 28.0;
const BUTTON_H: f64 = 28.0;
const TITLE_CHARS: usize = 12;

/// The three buttons of a tab in the strip.
#[derive(Clone, Copy)]
pub enum Button {
    Select,
    Pin,
    Close,
}

/// The window point of the center of a button of the tab at `index`, as the dev `click` action takes it.
pub fn button_center(index: usize, button: Button) -> (f64, f64) {
    let group = PAD + index as f64 * (TAB_W + INNER_GAP + PIN_W + INNER_GAP + CLOSE_W + GAP);
    let (offset, width) = match button {
        Button::Select => (0.0, TAB_W),
        Button::Pin => (TAB_W + INNER_GAP, PIN_W),
        Button::Close => (TAB_W + INNER_GAP + PIN_W + INNER_GAP, CLOSE_W),
    };
    (
        group + offset + width / 2.0,
        TITLE_BAR + f64::from(HEIGHT) / 2.0,
    )
}

/// One tab as the strip draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: String,
    pub title: String,
    pub active: bool,
    pub pinned: bool,
}

/// A press in the strip.
enum Action {
    Select(String),
    TogglePin(String),
    Close(String),
}

/// What the chrome layer draws and sends.
#[derive(Default)]
pub struct StripState {
    strip: i32,
    rows: Vec<Row>,
    send: Option<Box<dyn Fn(Action)>>,
    /// A press changed the active mark since the last read; the next read reports it.
    touched: bool,
    /// Called with the rows each time they change.
    changed: Option<ChangedFn>,
}

type ChangedFn = Rc<dyn Fn(&[Row])>;
type Shared = Rc<RefCell<StripState>>;

fn chrome(state: &Shared, cx: &mut Cx<Shared>) -> El {
    let dark = cx.system_dark();
    let tabs = state
        .borrow()
        .rows
        .iter()
        .fold(row().gap(GAP as f32), |strip, tab| {
            strip.child(tab_group(tab, dark))
        });
    col()
        .w_full()
        .h_full()
        .fill(Color::TRANSPARENT)
        .child(tabs.w_full().h(HEIGHT).px(PAD as f32).center_y())
        .child(el().key("page").w_full().grow(1.0))
}

fn tab_group(tab: &Row, dark: bool) -> El {
    let theme = tok::theme(dark);
    let fill = |on| {
        if on {
            tok::accent(0, dark)
        } else {
            theme.control_off
        }
    };
    let button = |name: &str, role: &'static str, label: String, width, on, caption: String| {
        el().key(format!("{name}:{}", tab.key))
            .role(role)
            .label(label)
            .focusable(true)
            .h(BUTTON_H as f32)
            .w(width as f32)
            .radius(8.0)
            .fill(fill(on))
            .center()
            .child(text(caption).font(type_::LABEL).color(theme.text))
    };
    let (select, pin, close) = (
        action_key(&tab.key, Action::Select),
        action_key(&tab.key, Action::TogglePin),
        action_key(&tab.key, Action::Close),
    );
    row()
        .gap(INNER_GAP as f32)
        .child(
            button(
                "select",
                "tab",
                tab.title.clone(),
                TAB_W,
                tab.active,
                short(&tab.title),
            )
            .on_press(select),
        )
        .child(
            button(
                "pin",
                "button",
                format!("{} {}", if tab.pinned { "Unpin" } else { "Pin" }, tab.title),
                PIN_W,
                tab.pinned,
                if tab.pinned { "Pinned" } else { "Pin" }.into(),
            )
            .on_press(pin),
        )
        .child(
            button(
                "close",
                "button",
                format!("Close {}", tab.title),
                CLOSE_W,
                false,
                "x".into(),
            )
            .on_press(close),
        )
}

/// The press handler that sends `make(key)` through the layer.
fn action_key(key: &str, make: fn(String) -> Action) -> impl Fn(&mut Shared) + 'static {
    let key = key.to_owned();
    move |state: &mut Shared| {
        if let Some(send) = &state.borrow().send {
            send(make(key.clone()));
        }
    }
}

fn short(title: &str) -> String {
    if title.chars().count() <= TITLE_CHARS {
        return title.to_owned();
    }
    let head: String = title.chars().take(TITLE_CHARS - 1).collect();
    format!("{head}\u{2026}")
}

fn overlay(_: &Shared, _: &mut Cx<Shared>) -> El {
    col().w_full().h_full()
}

/// The keys in the order Ardia's strip sends with `arrange` after a pin press (`chromeTabs.ts:143-159`):
/// pinned tabs first, the toggled tab at the end of that group when it becomes pinned, else at the start of the rest.
/// The count is the number of pinned tabs.
fn arrange_toggle(rows: &[Row], key: &str) -> Option<(Vec<String>, usize)> {
    let target = rows.iter().find(|row| row.key == key)?;
    let others = |pinned| {
        rows.iter()
            .filter(move |row| row.key != key && row.pinned == pinned)
    };
    let mut keys: Vec<_> = others(true).map(|row| row.key.clone()).collect();
    if !target.pinned {
        keys.push(key.to_owned());
    }
    let count = keys.len();
    if target.pinned {
        keys.push(key.to_owned());
    }
    keys.extend(others(false).map(|row| row.key.clone()));
    Some((keys, count))
}

/// The key that becomes active when `key` closes: the next tab, else the previous one.
fn successor(rows: &[Row], key: &str) -> Option<String> {
    let index = rows.iter().position(|row| row.key == key)?;
    rows.get(index + 1)
        .or_else(|| index.checked_sub(1).and_then(|i| rows.get(i)))
        .map(|row| row.key.clone())
}

/// Carry the active mark over to `next`: keep the old active tab if it still exists, else the first.
fn mark_active(previous: &[Row], next: &mut [Row]) {
    let key = previous
        .iter()
        .find(|row| row.active)
        .filter(|old| next.iter().any(|row| row.key == old.key))
        .map(|old| old.key.clone())
        .or_else(|| next.first().map(|row| row.key.clone()));
    for row in next.iter_mut() {
        row.active = Some(&row.key) == key.as_ref();
    }
}

/// Read the rows of the browser window. A tab without a strip key gets one. Return None when the window has no profile.
fn read_rows(host: &WindowHost, previous: &[Row]) -> Option<Vec<Row>> {
    let window = host.main()?;
    let profile = window.active_profile()?;
    let strips = host.strips().borrow();
    let mut rows = Vec::new();
    for tab in window.tabs_for_profile(&profile).iter() {
        if tab.closed() {
            continue;
        }
        let known = host
            .chrome()
            .tab_registry()
            .borrow()
            .app_tab_id(&tab)
            .map(str::to_owned);
        let key = match known {
            Some(key) => key,
            None => {
                let key = format!(
                    "tab:{}",
                    host.chrome().tab_registry().borrow_mut().browser_id(&tab)
                );
                if host.page_for(&tab, &key).is_none() {
                    continue;
                }
                key
            }
        };
        let title = tab.title().to_string();
        let title = if title.is_empty() {
            tab.url().to_string()
        } else {
            title
        };
        rows.push(Row {
            key,
            title,
            active: false,
            pinned: strips.is_pinned(&tab),
        });
    }
    // A window always has a tab; an empty list is a read in the middle of a change.
    if rows.is_empty() {
        return None;
    }
    mark_active(previous, &mut rows);
    Some(rows)
}

/// The strip layer and the surface it owns. Drop it before the engine shuts down.
pub struct StripLayer {
    /// Keeps the surface alive until drop.
    _surface: Rc<Surface<StripState>>,
}

impl StripLayer {
    /// Key the launch tab, host the Rune layers in the window, and show the page inside them.
    pub fn install(
        host: &Rc<WindowHost>,
        core: &Window,
        tab: &Tab,
        changed: ChangedFn,
    ) -> Result<Self, String> {
        let profile = core.active_profile().ok_or("the window has no profile")?;
        let page = host
            .page_for(tab, LAUNCH_KEY)
            .ok_or("the launch tab has no view")?;
        let state = Rc::new(RefCell::new(StripState {
            strip: core.chrome_window_id_for_profile(&profile),
            rows: Vec::new(),
            send: None,
            touched: false,
            changed: Some(changed),
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
        refresh(host, &surface, &state);
        let (host, surface_weak, state_weak) = (
            Rc::downgrade(host),
            Rc::downgrade(&surface),
            Rc::downgrade(&state),
        );
        state.borrow_mut().send = Some(sender(
            host.clone(),
            surface_weak.clone(),
            state_weak.clone(),
        ));
        poll(host, surface_weak, state_weak);
        Ok(Self { _surface: surface })
    }
}

/// The app has no React Native props to tell a page that it shows: the active row is the visible, selected page and
/// every other row is neither. Known limit: Ardia sets these from the pane layout, which Ria does not have.
fn mark_selected(host: &WindowHost, rows: &[Row]) {
    for row in rows {
        let tab = host
            .chrome()
            .tab_registry()
            .borrow()
            .tab_with_app_id(&row.key);
        if let Some(tab) = tab {
            host.set_page_active(&tab, row.active);
        }
    }
}

/// Read the tab list again and redraw when it changed. Stop when the layer is gone.
fn refresh(host: &WindowHost, surface: &Surface<StripState>, state: &Shared) {
    let (previous, touched) = {
        let mut state = state.borrow_mut();
        (state.rows.clone(), std::mem::take(&mut state.touched))
    };
    let Some(rows) = read_rows(host, &previous) else {
        return;
    };
    let active = rows.iter().find(|row| row.active);
    let page = active.and_then(|row| {
        host.chrome()
            .tab_registry()
            .borrow()
            .tab_with_app_id(&row.key)
    });
    // The page slot shows the active tab, so a new or selected tab does not cover the strip.
    if let Some((tab, key)) = page.zip(active.map(|row| row.key.as_str()))
        && let Some(view) = host.page_for(&tab, key)
    {
        surface.set_page(Some(&view));
    }
    // Every other tab's page stays mounted in the same slot (Ardia: sibling panes).
    let views: Vec<_> = rows
        .iter()
        .filter_map(|row| {
            let tab = host
                .chrome()
                .tab_registry()
                .borrow()
                .tab_with_app_id(&row.key)?;
            host.page_for(&tab, &row.key)
        })
        .collect();
    for view in &views {
        surface.mount_page(view);
    }
    surface.retain_pages(&views.iter().map(|view| &**view).collect::<Vec<_>>());
    mark_selected(host, &rows);
    if touched || rows != previous {
        let changed = {
            let mut state = state.borrow_mut();
            state.rows.clone_from(&rows);
            state.changed.clone()
        };
        surface.mark_dirty(true, false);
        if let Some(changed) = changed {
            changed(&rows);
        }
    }
}

fn poll(
    host: Weak<WindowHost>,
    surface: Weak<Surface<StripState>>,
    state: Weak<RefCell<StripState>>,
) {
    after(POLL, move || {
        let (Some(host_rc), Some(surface_rc), Some(state_rc)) =
            (host.upgrade(), surface.upgrade(), state.upgrade())
        else {
            return;
        };
        refresh(&host_rc, &surface_rc, &state_rc);
        poll(host, surface, state);
    });
}

/// Run an action from the layer on a later main-queue turn, then read the tab list again.
fn sender(
    host: Weak<WindowHost>,
    surface: Weak<Surface<StripState>>,
    state: Weak<RefCell<StripState>>,
) -> Box<dyn Fn(Action)> {
    Box::new(move |action| {
        let (host, surface, state) = (host.clone(), surface.clone(), state.clone());
        main_queue::post(Box::new(move || {
            let (Some(host), Some(surface), Some(state)) =
                (host.upgrade(), surface.upgrade(), state.upgrade())
            else {
                return;
            };
            run(&host, &state, action);
            refresh(&host, &surface, &state);
        }));
    })
}

fn run(host: &WindowHost, state: &Shared, action: Action) {
    let (strip, rows) = {
        let state = state.borrow();
        (Number(f64::from(state.strip)), state.rows.clone())
    };
    match action {
        Action::Select(key) => {
            // `state/tabs/tabReveal.ts:106` calls `revealTab` when the selection changes, before the page switch settles.
            host.hooks().reveal_tab(&key, None);
            if let StripCommandResult::Done(_) = host.strip_command(
                Number(1.0),
                TabStripCommand::Activate {
                    strip,
                    key: key.clone(),
                },
            ) {
                set_active(state, &key);
            }
        }
        Action::TogglePin(key) => {
            if let Some((keys, count)) = arrange_toggle(&rows, &key) {
                host.strip_command(
                    Number(1.0),
                    TabStripCommand::Arrange {
                        strip,
                        keys,
                        pinned: Number(count as f64),
                    },
                );
            }
        }
        Action::Close(key) => {
            if rows.len() < 2 {
                return;
            }
            let Some(tab) = host.chrome().tab_registry().borrow().tab_with_app_id(&key) else {
                return;
            };
            if rows.iter().any(|row| row.active && row.key == key)
                && let Some(next) = successor(&rows, &key)
            {
                host.strip_command(
                    Number(1.0),
                    TabStripCommand::Activate {
                        strip,
                        key: next.clone(),
                    },
                );
                set_active(state, &next);
            }
            tab.close();
        }
    }
}

fn set_active(state: &Shared, key: &str) {
    let mut state = state.borrow_mut();
    state.touched = true;
    for row in &mut state.rows {
        row.active = row.key == key;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: &str, active: bool, pinned: bool) -> Row {
        Row {
            key: key.into(),
            title: key.into(),
            active,
            pinned,
        }
    }

    #[test]
    fn pinning_puts_the_tab_after_the_pinned_ones() {
        let rows = [
            row("a", false, true),
            row("b", true, false),
            row("c", false, false),
        ];
        assert_eq!(
            arrange_toggle(&rows, "c"),
            Some((vec!["a".into(), "c".into(), "b".into()], 2))
        );
        assert_eq!(
            arrange_toggle(&rows, "a"),
            Some((vec!["a".into(), "b".into(), "c".into()], 0))
        );
        assert_eq!(arrange_toggle(&rows, "z"), None);
    }

    #[test]
    fn closing_the_active_tab_picks_the_next_then_the_previous() {
        let rows = [
            row("a", false, false),
            row("b", false, false),
            row("c", false, false),
        ];
        assert_eq!(successor(&rows, "a").as_deref(), Some("b"));
        assert_eq!(successor(&rows, "c").as_deref(), Some("b"));
    }

    #[test]
    fn a_new_tab_stays_inactive_and_a_closed_active_tab_falls_back() {
        let first = [row("launch", true, false)];
        let mut next = vec![row("launch", false, false), row("tab:2", false, false)];
        mark_active(&first, &mut next);
        assert!(next[0].active && !next[1].active);
        let mut pressed = vec![row("launch", false, false), row("tab:2", true, false)];
        let mut same = vec![
            row("launch", false, false),
            row("tab:2", false, false),
            row("tab:3", false, false),
        ];
        mark_active(&pressed, &mut same);
        assert!(same[1].active && !same[2].active);
        let mut gone = vec![row("launch", false, false)];
        mark_active(&pressed, &mut gone);
        assert!(gone[0].active);
        pressed.clear();
        let mut start = vec![row("launch", false, false)];
        mark_active(&pressed, &mut start);
        assert!(start[0].active);
    }

    #[test]
    fn button_centers_follow_the_layout() {
        assert_eq!(button_center(0, Button::Select), (68.0, 62.0));
        assert_eq!(
            button_center(1, Button::Pin),
            (12.0 + 188.0 + 8.0 + 112.0 + 4.0 + 20.0, 62.0)
        );
        assert_eq!(
            button_center(2, Button::Close).0,
            12.0 + 2.0 * 196.0 + 160.0 + 14.0
        );
    }
}
