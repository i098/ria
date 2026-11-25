//! `RuneView`: an `NSView` that hosts a Rune `LayerHost` on its own `CAMetalLayer`.
//!
//! The view forwards AppKit pointer, wheel, key and `NSTextInputClient` (IME) calls to the
//! runtime, resizes the layer with the view, steps one Rune frame per `CADisplayLink` tick, and
//! answers NSAccessibility through `accesskit_macos`.

#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, class, define_class, msg_send, sel};
use objc2_app_kit::{NSAppearanceCustomization, NSEvent, NSEventModifierFlags, NSTextInputClient, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSNotFound, NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize, NSString,
};
use rune::embed::LayerHost;
use rune::{AppSpec, Clause, Cx, El, Ime, Key, KeyEvent, Mods, ScrollSpec, TextEdit, Tx, col, el, text, tok};
use rune::tok::type_;

#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {}

/// Where the Rune layer sits in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Proof B: a sidebar strip next to the page view, opaque.
    Beside,
    /// Proof C: a transparent layer above the page view (and above a glass view).
    Above,
    /// Proof C: a second transparent layer above the page view, drawing the suggestion list.
    Overlay,
}

// ---------------------------------------------------------------------------------------------
// The Rune app drawn in the layer.

pub struct State {
    pub presses: u32,
    pub shown: u32,
    pub hover: bool,
    pub phase: f32,
    pub dark: bool,
    pub field: TextEdit,
    /// Proof C: the second layer is the overlay (a list of suggestion rows); the first is the chrome (button and text field).
    pub overlay: bool,
    pub transparent: bool,
}

thread_local! {
    /// Last scroll offset the list reported; the proof reads it as "the wheel reached Rune".
    pub static SCROLL_Y: Cell<f32> = const { Cell::new(0.0) };
    /// The one counter every layer reads (the proof's stand-in for the one `AppState`).
    static SHARED: Cell<u32> = const { Cell::new(0) };
    /// The appearance the view reported on the last tick, and the theme the last built tree used.
    static SYSTEM_DARK: Cell<bool> = const { Cell::new(true) };
    pub static DRAWN_DARK: Cell<bool> = const { Cell::new(true) };
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    DRAWN_DARK.with(|c| c.set(s.dark));
    let t = tok::theme(s.dark);
    let transparent = s.transparent;
    let button = el()
        .key("inc")
        .role("button")
        .label("Increment counter")
        .focusable(true)
        .w_full()
        .h(36.0)
        .radius(8.0)
        .center()
        .fill(if s.hover { tok::accent(0, s.dark) } else { t.control_off })
        .on_press(|s: &mut State| {
            s.presses += 1;
            SHARED.with(|c| c.set(c.get() + 1));
        })
        .on_hover_change(|s: &mut State, hovered, _tx| s.hover = hovered)
        .child(text(format!("Pressed {}", s.presses)).font(type_::TITLE).color(t.text));
    let field = text_input_row(&s.field);
    let mut list = col().key("list").w_full().gap(4.0);
    for i in 0..60 {
        list = list.child(text(format!("row {i}")).font(type_::BODY).color(t.text_secondary).h(22.0));
    }
    let bar = el().key("bar").h(6.0).w(40.0 + 160.0 * (0.5 + 0.5 * s.phase.sin())).radius(3.0).fill(tok::accent(1, s.dark));
    let root = col().w_full().h_full().padding(12.0).gap(10.0);
    let title = text("Ria layer").font(type_::TITLE).color(t.text);
    if transparent {
        // Proof C: the chrome layer has the title, the button and the text field; the overlay layer has the shared counter and a suggestion list.
        let shared = text(format!("Shared {}", s.shown)).key("shared").font(type_::BODY).color(t.text);
        if s.overlay {
            let rows = (0..4).fold(col().key("suggestions").role("list").label("Suggestions").w_full().gap(4.0), |c, i| {
                c.child(text(format!("Suggestion {i}")).key(format!("suggestion-{i}")).role("listitem").label(format!("Suggestion {i}")).focusable(true).font(type_::BODY).color(t.text).h(22.0))
            });
            return root.child(shared).child(rows);
        }
        return root.child(title).child(button).child(field).child(shared).child(bar);
    }
    root.fill(t.bg)
        .child(title)
        .child(button)
        .child(field)
        .child(bar)
        .child(col().key("scroll").w_full().grow(1.0).scroll(ScrollSpec { stick_bottom: false }).on_scroll_offset(|p| SCROLL_Y.with(|c| c.set(p.offset[1]))).child(list))
}

fn text_input_row(edit: &TextEdit) -> El {
    rune::text_input(edit, type_::BODY).key("field").on_edit(|s: &mut State| &mut s.field).w_full().h(32.0).single_line()
}

fn poll(s: &mut State, t: f64, _tx: &mut Tx) -> bool {
    s.phase = (t * 3.0) as f32;
    s.shown = SHARED.with(Cell::get);
    s.dark = SYSTEM_DARK.with(Cell::get);
    true
}

fn spec() -> AppSpec<State> {
    AppSpec {
        poll: Some(poll),
        ..AppSpec::new("Ria layer", (320.0, 560.0), || State { presses: 0, shown: 0, hover: false, phase: 0.0, dark: true, field: TextEdit::new(""), overlay: false, transparent: false }, view)
    }
}

// ---------------------------------------------------------------------------------------------
// Frame statistics.

#[derive(Clone, Copy)]
struct Sample {
    interval_ms: f64,
    callback_ms: f64,
    cpu_us: u64,
    drew: bool,
}

#[derive(Default)]
pub struct Stats {
    active: Option<(String, Vec<Sample>, Instant)>,
    pub refresh_hz: f64,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[(((sorted.len() - 1) as f64) * p).round() as usize]
}

impl Stats {
    pub fn begin(&mut self, name: &str) {
        self.active = Some((name.to_owned(), Vec::new(), Instant::now()));
    }

    fn push(&mut self, s: Sample) {
        if let Some((_, v, _)) = &mut self.active {
            v.push(s);
        }
    }

    /// Ends the running phase and returns one JSON line of numbers.
    pub fn end(&mut self) -> Option<String> {
        let (name, v, began) = self.active.take()?;
        let wall = began.elapsed().as_secs_f64();
        let nominal = if self.refresh_hz > 0.0 { 1000.0 / self.refresh_hz } else { 16.667 };
        // The first interval of a phase spans the previous phase's work, so it is not a frame time.
        let intervals: Vec<f64> = v.iter().skip(1).map(|s| s.interval_ms).collect();
        let mut sorted = intervals.clone();
        sorted.sort_by(f64::total_cmp);
        let mut callbacks: Vec<f64> = v.iter().map(|s| s.callback_ms).collect();
        callbacks.sort_by(f64::total_cmp);
        let mut cpu: Vec<f64> = v.iter().map(|s| s.cpu_us as f64 / 1000.0).collect();
        cpu.sort_by(f64::total_cmp);
        let late = intervals.iter().filter(|i| **i > nominal * 1.5).count();
        let over = intervals.iter().filter(|i| **i > 20.7).count();
        let drew = v.iter().filter(|s| s.drew).count();
        let mean = if intervals.is_empty() { 0.0 } else { intervals.iter().sum::<f64>() / intervals.len() as f64 };
        Some(format!(
            "{{\"event\":\"frame_times\",\"phase\":\"{name}\",\"seconds\":{wall:.2},\"ticks\":{},\"drew\":{drew},\"display_hz\":{:.1},\"nominal_ms\":{nominal:.3},\"interval_ms\":{{\"mean\":{mean:.3},\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"max\":{:.3}}},\"fps\":{:.1},\"late_over_1.5x\":{late},\"over_20.7ms\":{over},\"callback_ms\":{{\"p50\":{:.3},\"p95\":{:.3},\"max\":{:.3}}},\"rune_cpu_ms\":{{\"p50\":{:.3},\"p95\":{:.3},\"max\":{:.3}}}}}",
            v.len(),
            self.refresh_hz,
            percentile(&sorted, 0.5),
            percentile(&sorted, 0.95),
            percentile(&sorted, 0.99),
            sorted.last().copied().unwrap_or(0.0),
            if mean > 0.0 { 1000.0 / mean } else { 0.0 },
            percentile(&callbacks, 0.5),
            percentile(&callbacks, 0.95),
            callbacks.last().copied().unwrap_or(0.0),
            percentile(&cpu, 0.5),
            percentile(&cpu, 0.95),
            cpu.last().copied().unwrap_or(0.0),
        ))
    }
}

// ---------------------------------------------------------------------------------------------
// Accessibility handlers: they only record what the OS asked; `tick` answers on the main thread.

struct Activation(Arc<AtomicBool>);

impl rune::accesskit::ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<rune::accesskit::TreeUpdate> {
        self.0.store(true, Ordering::Relaxed);
        None
    }
}

struct Actions(Arc<Mutex<Vec<String>>>);

impl rune::accesskit::ActionHandler for Actions {
    fn do_action(&mut self, request: rune::accesskit::ActionRequest) {
        if let Ok(mut log) = self.0.lock() {
            log.push(format!("{:?} on node {:?}", request.action, request.target_node));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The view.

/// What the view handed to the Rune runtime: the "Rune event log" of check B1.
#[derive(Default)]
pub struct EventLog {
    pub moves: Vec<(f32, f32)>,
    pub wheels: u32,
    pub keys: u32,
}

pub struct Ivars {
    pub rect_frames: Cell<u32>,
    pub rect_bad: Cell<u32>,
    pub check_rect: Cell<bool>,
    pub log: RefCell<EventLog>,
    mode: Mode,
    host: RefCell<Option<Box<LayerHost<State>>>>,
    marked: RefCell<Option<String>>,
    mods: Cell<Mods>,
    link: RefCell<Option<Retained<AnyObject>>>,
    pub stats: RefCell<Stats>,
    a11y: RefCell<Option<accesskit_macos::SubclassingAdapter>>,
    a11y_link: RefCell<rune::AccessibilityLink>,
    ax_asked: Arc<AtomicBool>,
    pub ax_actions: Arc<Mutex<Vec<String>>>,
    prev_ts: Cell<f64>,
    pub ticks: Cell<u64>,
    pub frames_drawn: Cell<u64>,
    pub last_error: RefCell<Option<String>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaRuneView"]
    #[ivars = Ivars]
    pub struct RuneView;

    impl RuneView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool {
            self.ivars().mode == Mode::Beside
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(makeBackingLayer))]
        fn make_backing_layer(&self) -> *mut AnyObject {
            let class = AnyClass::get(c"CAMetalLayer").expect("QuartzCore is linked");
            let layer: Retained<AnyObject> = unsafe { msg_send![class, new] };
            Retained::autorelease_return(layer)
        }

        #[unsafe(method(wantsUpdateLayer))]
        fn wants_update_layer(&self) -> bool {
            true
        }

        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {}

        #[unsafe(method(viewDidMoveToWindow))]
        fn view_did_move_to_window(&self) {
            let _: () = unsafe { msg_send![super(self), viewDidMoveToWindow] };
            self.attach();
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            let _: () = unsafe { msg_send![super(self), setFrameSize: size] };
            self.resync();
        }

        #[unsafe(method(viewDidChangeBackingProperties))]
        fn view_did_change_backing_properties(&self) {
            let _: () = unsafe { msg_send![super(self), viewDidChangeBackingProperties] };
            self.resync();
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            let _: () = unsafe { msg_send![super(self), updateTrackingAreas] };
            let options = NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect;
            let area = unsafe {
                NSTrackingArea::initWithRect_options_owner_userInfo(self.mtm().alloc(), NSRect::ZERO, options, Some(self), None)
            };
            self.addTrackingArea(&area);
        }

        #[unsafe(method(tick:))]
        fn tick(&self, link: &AnyObject) {
            self.step(link);
        }

        // ---- pointer and wheel -----------------------------------------------------------
        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            let (x, y) = self.point(event);
            self.ivars().log.borrow_mut().moves.push((x, y));
            self.with_host(|h| h.runtime_mut().pointer_move(x, y));
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let (x, y) = self.point(event);
            self.with_host(|h| h.runtime_mut().pointer_move(x, y));
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.with_host(|h| h.runtime_mut().pointer_leave());
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if let Some(window) = self.window() {
                window.makeFirstResponder(Some(self));
            }
            let (x, y) = self.point(event);
            self.with_host(|h| {
                h.runtime_mut().pointer_move(x, y);
                h.runtime_mut().pointer_down(x, y);
            });
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let (x, y) = self.point(event);
            self.with_host(|h| h.runtime_mut().pointer_up(x, y));
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            let (x, y) = self.point(event);
            self.with_host(|h| h.runtime_mut().secondary_down(x, y));
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &NSEvent) {
            let (x, y) = self.point(event);
            self.with_host(|h| h.runtime_mut().secondary_up(x, y));
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let (x, y) = self.point(event);
            let (dx, dy) = (event.scrollingDeltaX() as f32, event.scrollingDeltaY() as f32);
            self.ivars().log.borrow_mut().wheels += 1;
            self.with_host(|h| {
                h.runtime_mut().pointer_move(x, y);
                h.runtime_mut().wheel_xy(x, y, dx, dy);
            });
        }

        // ---- keys ------------------------------------------------------------------------
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            let m = mods(event);
            self.ivars().mods.set(m);
            if m.cmd {
                let chars = event.charactersIgnoringModifiers().map(|s| s.to_string()).unwrap_or_default();
                if let Some(c) = chars.chars().next() {
                    self.ivars().log.borrow_mut().keys += 1;
                    self.with_host(|h| h.runtime_mut().key(KeyEvent { key: Key::Command(c.to_ascii_lowercase()), mods: m }));
                    return;
                }
            }
            let events = NSArray::from_slice(&[event]);
            let _: () = unsafe { msg_send![self, interpretKeyEvents: &*events] };
        }

        #[unsafe(method(doCommandBySelector:))]
        fn do_command_by_selector(&self, selector: Sel) {
            let name = selector.name().to_string_lossy().into_owned();
            let mut m = self.ivars().mods.get();
            let extend = name.ends_with("AndModifySelection:");
            if extend {
                m.shift = true;
            }
            let key = match name.trim_end_matches("AndModifySelection:").trim_end_matches(':') {
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
                    m.shift = true;
                    Key::Tab
                }
                "cancelOperation" => Key::Escape,
                "pageUp" | "scrollPageUp" => Key::PageUp,
                "pageDown" | "scrollPageDown" => Key::PageDown,
                _ => return,
            };
            self.ivars().log.borrow_mut().keys += 1;
            self.with_host(|h| h.runtime_mut().key(KeyEvent { key, mods: m }));
        }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            if let Some(a) = self.ivars().a11y.borrow_mut().as_mut() {
                if let Some(events) = a.update_view_focus_state(true) {
                    events.raise();
                }
            }
            true
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            if let Some(a) = self.ivars().a11y.borrow_mut().as_mut() {
                if let Some(events) = a.update_view_focus_state(false) {
                    events.raise();
                }
            }
            true
        }
    }

    unsafe impl NSObjectProtocol for RuneView {}

    unsafe impl NSTextInputClient for RuneView {
        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool {
            self.ivars().marked.borrow().is_some()
        }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            match &*self.ivars().marked.borrow() {
                Some(s) => NSRange { location: 0, length: s.encode_utf16().count() },
                None => NSRange { location: NSNotFound as usize, length: 0 },
            }
        }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            NSRange { location: NSNotFound as usize, length: 0 }
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn set_marked_text(&self, string: &AnyObject, selected: NSRange, _replacement: NSRange) {
            let text = plain_string(string);
            if text.is_empty() {
                *self.ivars().marked.borrow_mut() = None;
                self.with_host(|h| h.runtime_mut().ime(Ime::Preedit(String::new(), None, Vec::new())));
                return;
            }
            let cursor = if selected.location == NSNotFound as usize {
                None
            } else {
                let start = utf16_to_byte(&text, selected.location);
                let end = utf16_to_byte(&text, selected.location + selected.length);
                Some((start, end))
            };
            let clauses = match cursor {
                Some((a, b)) if a != b => vec![Clause::new(0..text.len()), Clause::new(a..b).target()],
                _ => Vec::new(),
            };
            *self.ivars().marked.borrow_mut() = Some(text.clone());
            self.with_host(|h| h.runtime_mut().ime(Ime::Preedit(text, cursor, clauses)));
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            if let Some(text) = self.ivars().marked.borrow_mut().take() {
                self.with_host(|h| h.runtime_mut().ime(Ime::Commit(text)));
            }
        }

        #[unsafe(method(validAttributesForMarkedText))]
        fn valid_attributes_for_marked_text(&self) -> *mut NSArray<NSAttributedStringKey> {
            Retained::autorelease_return(NSArray::new())
        }

        #[unsafe(method(attributedSubstringForProposedRange:actualRange:))]
        fn attributed_substring(&self, _range: NSRange, _actual: *mut NSRange) -> *mut NSAttributedString {
            std::ptr::null_mut()
        }

        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, string: &AnyObject, _replacement: NSRange) {
            let text = plain_string(string);
            let was_marked = self.ivars().marked.borrow_mut().take().is_some();
            let m = self.ivars().mods.get();
            if was_marked {
                self.with_host(|h| h.runtime_mut().ime(Ime::Commit(text)));
            } else {
                self.ivars().log.borrow_mut().keys += 1;
                self.with_host(|h| h.runtime_mut().key(KeyEvent { key: Key::Text(text), mods: m }));
            }
        }

        #[unsafe(method(characterIndexForPoint:))]
        fn character_index_for_point(&self, _point: NSPoint) -> usize {
            NSNotFound as usize
        }

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect_for_character_range(&self, _range: NSRange, _actual: *mut NSRange) -> NSRect {
            let area = self.ivars().host.borrow().as_ref().and_then(|h| h.runtime().ime_area).unwrap_or([0.0, 0.0, 1.0, 20.0]);
            let local = NSRect::new(NSPoint::new(area[0] as f64, area[1] as f64), NSSize::new(area[2] as f64, area[3] as f64));
            let in_window = self.convertRect_toView(local, None);
            match self.window() {
                Some(w) => w.convertRectToScreen(in_window),
                None => in_window,
            }
        }
    }
);

#[repr(C)]
#[derive(Clone, Copy)]
struct FrameRateRange {
    minimum: f32,
    maximum: f32,
    preferred: f32,
}

// SAFETY: matches `CAFrameRateRange`: three floats.
unsafe impl objc2::Encode for FrameRateRange {
    const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CAFrameRateRange", &[objc2::Encoding::Float, objc2::Encoding::Float, objc2::Encoding::Float]);
}

fn plain_string(object: &AnyObject) -> String {
    let is_attributed: bool = unsafe { msg_send![object, isKindOfClass: class!(NSAttributedString)] };
    let string: Retained<NSString> =
        if is_attributed { unsafe { msg_send![object, string] } } else { unsafe { Retained::retain(object as *const AnyObject as *mut NSString).expect("non-null") } };
    string.to_string()
}

fn utf16_to_byte(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (i, c) in text.char_indices() {
        if seen >= units {
            return i;
        }
        seen += c.len_utf16();
    }
    text.len()
}

fn mods(event: &NSEvent) -> Mods {
    let f = event.modifierFlags();
    let mut m = Mods::default();
    m.shift = f.contains(NSEventModifierFlags::Shift);
    m.cmd = f.contains(NSEventModifierFlags::Command);
    m.logo = m.cmd;
    m.alt = f.contains(NSEventModifierFlags::Option);
    m.ctrl = f.contains(NSEventModifierFlags::Control);
    m
}

impl RuneView {
    pub fn new(mtm: MainThreadMarker, mode: Mode, frame: NSRect) -> Retained<Self> {
        let ivars = Ivars {
            rect_frames: Cell::new(0),
            rect_bad: Cell::new(0),
            check_rect: Cell::new(false),
            log: RefCell::new(EventLog::default()),
            mode,
            host: RefCell::new(None),
            marked: RefCell::new(None),
            mods: Cell::new(Mods::default()),
            link: RefCell::new(None),
            stats: RefCell::new(Stats::default()),
            a11y: RefCell::new(None),
            a11y_link: RefCell::new(rune::AccessibilityLink::new()),
            ax_asked: Arc::new(AtomicBool::new(false)),
            ax_actions: Arc::new(Mutex::new(Vec::new())),
            prev_ts: Cell::new(0.0),
            ticks: Cell::new(0),
            frames_drawn: Cell::new(0),
            last_error: RefCell::new(None),
        };
        let this = mtm.alloc::<Self>().set_ivars(ivars);
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        this.setWantsLayer(true);
        this
    }

    pub fn host_ready(&self) -> bool {
        self.ivars().host.borrow().is_some()
    }

    pub fn with_host<R>(&self, f: impl FnOnce(&mut LayerHost<State>) -> R) -> Option<R> {
        self.ivars().host.borrow_mut().as_mut().map(|h| f(&mut **h))
    }

    pub fn stats(&self) -> std::cell::RefMut<'_, Stats> {
        self.ivars().stats.borrow_mut()
    }

    pub fn rect_check(&self, on: bool) {
        let iv = self.ivars();
        iv.rect_frames.set(0);
        iv.rect_bad.set(0);
        iv.check_rect.set(on);
    }

    pub fn rect_result(&self) -> (u32, u32) {
        (self.ivars().rect_frames.get(), self.ivars().rect_bad.get())
    }

    pub fn ivars_log(&self) -> std::cell::RefMut<'_, EventLog> {
        self.ivars().log.borrow_mut()
    }

    pub fn ticks(&self) -> u64 {
        self.ivars().ticks.get()
    }

    pub fn composing(&self) -> bool {
        self.ivars().marked.borrow().is_some()
    }

    /// The pointer position inside the view, from an event with a window or a screen location.
    fn point(&self, event: &NSEvent) -> (f32, f32) {
        let at = event.locationInWindow();
        let in_window = if event.window(self.mtm()).is_some() {
            at
        } else {
            self.window().map_or(at, |w| w.convertPointFromScreen(at))
        };
        let p = self.convertPoint_fromView(in_window, None);
        (p.x as f32, p.y as f32)
    }

    fn scale(&self) -> f32 {
        self.window().map_or(1.0, |w| w.backingScaleFactor() as f32)
    }

    /// Creates the host once the view is in a window with a size.
    fn attach(&self) {
        if self.window().is_none() || self.host_ready() {
            return;
        }
        let size = self.bounds().size;
        if size.width < 1.0 || size.height < 1.0 {
            return;
        }
        let layer: *mut AnyObject = unsafe { msg_send![self, layer] };
        let scale = self.scale();
        let _: () = unsafe { msg_send![layer, setContentsScale: scale as f64] };
        let transparent = self.ivars().mode != Mode::Beside;
        let spec = spec();
        // The view's mode, not creation order, picks the tree (`init` is a plain fn pointer and cannot capture it).
        let state = State { overlay: self.ivars().mode == Mode::Overlay, transparent, ..(spec.init)() };
        // SAFETY: `layer` is this view's CAMetalLayer; it lives as long as the view, which owns
        // the host, and this runs on the main thread.
        let made = unsafe {
            LayerHost::from_core_animation_layer(layer.cast::<c_void>(), &spec, state, (size.width as f32, size.height as f32), scale, true, Arc::new(|| {}))
        };
        match made {
            Ok(mut host) => {
                host.runtime_mut().transparent = transparent;
                *self.ivars().host.borrow_mut() = Some(Box::new(host));
            }
            Err(e) => {
                *self.ivars().last_error.borrow_mut() = Some(e);
                return;
            }
        }
        if transparent {
            let _: () = unsafe { msg_send![layer, setOpaque: false] };
        }
        let activation = Activation(self.ivars().ax_asked.clone());
        let actions = Actions(self.ivars().ax_actions.clone());
        // SAFETY: `self` is a live NSView; the adapter is dropped with the view's ivars.
        *self.ivars().a11y.borrow_mut() =
            Some(unsafe { accesskit_macos::SubclassingAdapter::new((self as *const Self as *mut Self).cast::<c_void>(), activation, actions) });
        let link: Retained<AnyObject> = unsafe { msg_send![self, displayLinkWithTarget: self, selector: sel!(tick:)] };
        unsafe {
            let run_loop: *mut AnyObject = msg_send![class!(NSRunLoop), mainRunLoop];
            let _: () = msg_send![&*link, addToRunLoop: run_loop, forMode: objc2_foundation::NSRunLoopCommonModes];
        }
        let hz: f32 = std::env::var("ABC_HZ").ok().and_then(|v| v.parse().ok()).unwrap_or(60.0);
        let range = FrameRateRange { minimum: hz, maximum: hz, preferred: hz };
        let _: () = unsafe { msg_send![&*link, setPreferredFrameRateRange: range] };
        *self.ivars().link.borrow_mut() = Some(link);
    }

    /// Follows the view's size and the window's scale.
    fn resync(&self) {
        if !self.host_ready() {
            self.attach();
            return;
        }
        let size = self.bounds().size;
        let scale = self.scale();
        let layer: *mut AnyObject = unsafe { msg_send![self, layer] };
        let _: () = unsafe { msg_send![layer, setContentsScale: scale as f64] };
        self.with_host(|h| h.resize((size.width as f32, size.height as f32), scale));
    }

    /// One display-link tick: the proof driver, one Rune frame, accessibility, statistics.
    fn step(&self, link: &AnyObject) {
        let started = Instant::now();
        let ts: f64 = unsafe { msg_send![link, timestamp] };
        let duration: f64 = unsafe { msg_send![link, duration] };
        let iv = self.ivars();
        let interval_ms = if iv.prev_ts.get() > 0.0 { (ts - iv.prev_ts.get()) * 1000.0 } else { 0.0 };
        iv.prev_ts.set(ts);
        iv.ticks.set(iv.ticks.get() + 1);
        if duration > 0.0 {
            iv.stats.borrow_mut().refresh_hz = 1.0 / duration;
        }
        let dark = {
            let name = self.effectiveAppearance().name().to_string();
            name.contains("Dark")
        };
        SYSTEM_DARK.with(|c| c.set(dark));
        if iv.check_rect.get() {
            // The layer rectangle must equal the view bounds, and the drawable the bounds in pixels, in every frame.
            let layer: *mut AnyObject = unsafe { msg_send![self, layer] };
            let lb: NSRect = unsafe { msg_send![layer, bounds] };
            let drawable: NSSize = unsafe { msg_send![layer, drawableSize] };
            let b = self.bounds().size;
            let scale = self.scale() as f64;
            let ok = (lb.size.width - b.width).abs() < 0.01
                && (lb.size.height - b.height).abs() < 0.01
                && (drawable.width - b.width * scale).abs() < 1.0
                && (drawable.height - b.height * scale).abs() < 1.0;
            iv.rect_frames.set(iv.rect_frames.get() + 1);
            if !ok {
                iv.rect_bad.set(iv.rect_bad.get() + 1);
            }
        }
        crate::proof::on_tick(self);
        let (drew, cpu_us) = {
            let mut guard = iv.host.borrow_mut();
            let Some(host) = guard.as_mut() else { return };
            if iv.mode != Mode::Beside {
                // wgpu marks a configured layer opaque; the layer must stay translucent.
                let layer: *mut AnyObject = unsafe { msg_send![self, layer] };
                let _: () = unsafe { msg_send![layer, setOpaque: false] };
            }
            let out = host.frame();
            if iv.ax_asked.swap(false, Ordering::Relaxed) {
                iv.a11y_link.borrow_mut().activate();
            }
            if let Some(update) = iv.a11y_link.borrow_mut().service(host.runtime_mut(), "Ria layer") {
                if let Some(adapter) = iv.a11y.borrow_mut().as_mut() {
                    if let Some(events) = adapter.update_if_active(|| update) {
                        events.raise();
                    }
                }
            }
            (out.drew, out.cpu_us)
        };
        if drew {
            iv.frames_drawn.set(iv.frames_drawn.get() + 1);
        }
        iv.stats.borrow_mut().push(Sample { interval_ms, callback_ms: started.elapsed().as_secs_f64() * 1000.0, cpu_us, drew });
    }

    pub fn ax_actions(&self) -> Vec<String> {
        self.ivars().ax_actions.lock().map(|l| l.clone()).unwrap_or_default()
    }

    pub fn last_error(&self) -> String {
        self.ivars().last_error.borrow().clone().unwrap_or_default()
    }

    /// Reads back state the proofs assert on.
    pub fn probe(&self) -> Probe {
        let guard = self.ivars().host.borrow();
        let Some(host) = guard.as_ref() else { return Probe::default() };
        let rt = host.runtime();
        Probe {
            presses: rt.state.presses,
            shown: rt.state.shown,
            hover: rt.state.hover,
            field: rt.state.field.text.clone(),
            composing: rt.state.field.is_composing(),
            preedit: rt.state.field.preedit().map(|(t, _)| t.to_owned()),
            pointer: rt.pointer(),
            focus: rt.has_focus(),
            scroll_y: SCROLL_Y.with(Cell::get),
            transparent: rt.transparent,
            regions: rt.hit_regions().iter().take(12).cloned().collect(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Probe {
    pub presses: u32,
    pub shown: u32,
    pub hover: bool,
    pub field: String,
    pub composing: bool,
    pub preedit: Option<String>,
    pub pointer: Option<(f32, f32)>,
    pub focus: bool,
    pub scroll_y: f32,
    pub transparent: bool,
    pub regions: Vec<[f32; 4]>,
}

/// Center of a keyed element in the view's own points, once laid out.
pub fn element_center(view: &RuneView, key: &str) -> Option<(f32, f32)> {
    view.with_host(|h| h.runtime().find(key)).flatten()
}
