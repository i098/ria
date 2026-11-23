//! The proof scenarios. Each step prints one JSON line; checks print `pass` and a detail.
//!
//! `--proof a`: engine, one window, one tab, one page, clean quit.
//! `--proof b`: A plus a Rune layer beside the page view: pointer, wheel, keys, IME, resize.
//! `--proof c`: A plus a Rune layer above the page view with a glass view under it, and the
//! accessibility tree a screen reader reads.

#![allow(unsafe_code)]

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{MainThreadMarker, class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSEvent, NSEventModifierFlags, NSEventType, NSView, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowOrderingMode,
};
use objc2_foundation::{NSNotFound, NSPoint, NSProcessInfo, NSRange, NSRect, NSSize, NSString};

use objc2::runtime::NSObject;
use objc2::{ClassType, define_class};

use crate::layer::{Mode, RuneView, element_center};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    A,
    B,
    C,
}

thread_local! {
    /// The last title the tab delegate reported, with the time it arrived.
    static TAB_TITLE: RefCell<Option<(f64, String)>> = const { RefCell::new(None) };
    static T0: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "HalyardTabDelegate"]
    struct TabDelegate;

    impl TabDelegate {
        #[unsafe(method(tabDidChangeTitle:))]
        fn tab_did_change_title(&self, tab: &AnyObject) {
            let title: Option<Retained<NSString>> = unsafe { msg_send![tab, title] };
            let title = title.map(|t| t.to_string()).unwrap_or_default();
            let ms = T0.with(|t| t.get()).map_or(0.0, t_ms);
            TAB_TITLE.with(|c| *c.borrow_mut() = Some((ms, title)));
        }
    }
);

const PAGE_TITLE: &str = "Halyard proof page";
const STRIP_W: f64 = 320.0;
const GLASS: (f64, f64) = (340.0, 220.0);

pub fn emit(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn json(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// A step the scenario takes next.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    WaitLoad,
    WaitRune,
    Steady,
    SteadyEnd,
    PointerMove,
    Click,
    Wheel,
    FocusField,
    TypeKeys,
    EditKeys,
    Ime,
    Resize,
    ResizeEnd,
    Transparency,
    GlassShot,
    ClickAbove,
    AxSelfCheck,
    AxHold,
    Shot,
    Finish,
    Quit,
    Done,
}

struct Ctx {
    kind: Kind,
    started: Instant,
    engine: Retained<AnyObject>,
    window: Retained<AnyObject>,
    nswindow: Retained<NSWindow>,
    tab: Retained<AnyObject>,
    profile: Retained<AnyObject>,
    host_view: Retained<NSView>,
    tab_view: Retained<NSView>,
    rune: Option<Retained<RuneView>>,
    glass: Option<Retained<NSView>>,
    step: Step,
    until: Instant,
    sub: u32,
    checks: u32,
    fails: u32,
    js: Rc<RefCell<Option<String>>>,
    js_inflight: bool,
    base_frame: NSRect,
    page_widths: Vec<f64>,
    quit_at: Option<Instant>,
    page_loaded_ms: Option<f64>,
}

thread_local! {
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

fn t_ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

impl Ctx {
    fn ev(&self, name: &str, fields: &str) {
        let sep = if fields.is_empty() { "" } else { "," };
        emit(&format!("{{\"t_ms\":{:.1},\"proof\":\"{:?}\",\"event\":\"{name}\"{sep}{fields}}}", t_ms(self.started), self.kind));
    }

    fn check(&mut self, name: &str, pass: bool, detail: &str) {
        self.checks += 1;
        if !pass {
            self.fails += 1;
        }
        self.ev("check", &format!("\"name\":{},\"pass\":{pass},\"detail\":{}", json(name), json(detail)));
    }

    fn wait(&mut self, next: Step, secs: f64) {
        self.step = next;
        self.until = Instant::now() + Duration::from_secs_f64(secs);
    }

    fn ready(&self) -> bool {
        Instant::now() >= self.until
    }

    fn js(&mut self, code: &str) {
        if self.js_inflight {
            return;
        }
        self.js_inflight = true;
        let slot = self.js.clone();
        let block = RcBlock::new(move |json: *mut AnyObject| {
            let text = if json.is_null() { "null".to_owned() } else { unsafe { (*(json as *mut NSString)).to_string() } };
            *slot.borrow_mut() = Some(text);
        });
        let code = NSString::from_str(code);
        let _: () = unsafe { msg_send![&*self.tab, evaluate: &*code, completion: &*block] };
    }

    fn js_result(&mut self) -> Option<serde_json::Value> {
        let text = self.js.borrow_mut().take()?;
        self.js_inflight = false;
        // `evaluate:` hands back the JSON of the value; a JSON string holds our JSON object.
        let value: serde_json::Value = serde_json::from_str(&text).ok()?;
        match value {
            serde_json::Value::String(s) => serde_json::from_str(&s).ok(),
            other => Some(other),
        }
    }

    fn quit(&mut self) {
        self.ev("quit_requested", &format!("\"checks\":{},\"fails\":{}", self.checks, self.fails));
        let _: () = unsafe { msg_send![&*self.engine, quit] };
        self.quit_at = Some(Instant::now());
        self.step = Step::Done;
    }

    fn view(&self) -> Option<&Retained<RuneView>> {
        self.rune.as_ref()
    }
}

const STATE_JS: &str = "JSON.stringify({title:document.title,raf:window.__raf||0,clicks:window.__clicks||0,w:innerWidth,h:innerHeight})";

/// Builds the window and the first tab. Returns false when the engine has no profile yet.
pub fn begin(kind: Kind, page_url: &str, started: Instant, engine: Retained<AnyObject>, profile: Retained<AnyObject>) {
    let mtm = MainThreadMarker::new().expect("main thread");
    let version: Retained<NSString> = unsafe { msg_send![&*engine, chromiumVersion] };
    let pid = std::process::id();
    emit(&format!(
        "{{\"t_ms\":{:.1},\"proof\":\"{kind:?}\",\"event\":\"engine_started\",\"chromium\":{},\"pid\":{pid}}}",
        t_ms(started),
        json(&version.to_string())
    ));

    let window_class = AnyClass::get(c"ArdiaCoreWindow").expect("ArdiaCore is loaded");
    let rect = NSRect::new(NSPoint::new(60.0, 80.0), NSSize::new(1040.0, 640.0));
    let alloc: Allocated<AnyObject> = unsafe { msg_send![window_class, alloc] };
    let window: Retained<AnyObject> = unsafe { msg_send![alloc, initWithContentRect: rect] };
    let url = NSString::from_str(page_url);
    let tab: Retained<AnyObject> = unsafe { msg_send![&*window, openTab: &*url, profile: &*profile, foreground: true] };
    let nswindow: Retained<NSWindow> = unsafe { msg_send![&*window, window] };
    let host_view: Retained<NSView> = unsafe { msg_send![&*window, hostView] };
    let tab_view: Retained<NSView> = unsafe { msg_send![&*tab, view] };
    nswindow.setAcceptsMouseMovedEvents(true);
    T0.with(|t| t.set(Some(started)));
    let tab_delegate: Retained<AnyObject> = unsafe { msg_send![TabDelegate::class(), new] };
    let _: () = unsafe { msg_send![&*tab, setDelegate: &*tab_delegate] };
    std::mem::forget(tab_delegate);

    let bounds = host_view.bounds();
    let grow = NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable;
    let mut rune = None;
    let mut glass = None;
    match kind {
        Kind::A | Kind::C => tab_view.setFrame(bounds),
        Kind::B => tab_view.setFrame(NSRect::new(NSPoint::ZERO, NSSize::new(bounds.size.width - STRIP_W, bounds.size.height))),
    }
    tab_view.setAutoresizingMask(grow);
    host_view.addSubview(&tab_view);
    let _: () = unsafe { msg_send![&*window, activateTab: &*tab] };
    let _: () = unsafe { msg_send![&*window, showInactive] };

    match kind {
        Kind::A => {}
        Kind::B => {
            let frame = NSRect::new(NSPoint::new(bounds.size.width - STRIP_W, 0.0), NSSize::new(STRIP_W, bounds.size.height));
            let view = RuneView::new(mtm, Mode::Beside, frame);
            view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin | NSAutoresizingMaskOptions::ViewHeightSizable);
            host_view.addSubview(&view);
            rune = Some(view);
        }
        Kind::C => {
            let frame = NSRect::new(
                NSPoint::new(bounds.size.width - GLASS.0 - 24.0, 24.0),
                NSSize::new(GLASS.0, GLASS.1),
            );
            let margins = NSAutoresizingMaskOptions::ViewMinXMargin | NSAutoresizingMaskOptions::ViewMaxYMargin;
            let g = make_glass(mtm, frame);
            g.setAutoresizingMask(margins);
            host_view.addSubview_positioned_relativeTo(&g, NSWindowOrderingMode::Above, Some(&tab_view));
            let view = RuneView::new(mtm, Mode::Above, frame);
            view.setAutoresizingMask(margins);
            host_view.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Above, Some(&g));
            glass = Some(g);
            rune = Some(view);
        }
    }

    let wn = nswindow.windowNumber();
    emit(&format!(
        "{{\"t_ms\":{:.1},\"proof\":\"{kind:?}\",\"event\":\"window_opened\",\"window_number\":{wn},\"frame\":{},\"host_view\":[{},{}]}}",
        t_ms(started),
        json(&format!("{:?}", nswindow.frame())),
        bounds.size.width,
        bounds.size.height
    ));
    let ctx = Ctx {
        kind,
        started,
        engine,
        window,
        nswindow,
        tab,
        profile,
        host_view,
        tab_view,
        rune,
        glass,
        step: Step::WaitLoad,
        until: Instant::now(),
        sub: 0,
        checks: 0,
        fails: 0,
        js: Rc::new(RefCell::new(None)),
        js_inflight: false,
        base_frame: NSRect::ZERO,
        page_widths: Vec::new(),
        quit_at: None,
        page_loaded_ms: None,
    };
    CTX.with(|c| *c.borrow_mut() = Some(ctx));
    start_timer();
}

pub struct Config {
    pub kind: Kind,
    pub page_url: String,
    pub started: Instant,
}

thread_local! {
    static CONFIG: RefCell<Option<Config>> = const { RefCell::new(None) };
}

pub fn set_config(config: Config) {
    CONFIG.with(|c| *c.borrow_mut() = Some(config));
}

/// The engine delegate calls this on `engineDidStart`: take the default profile, loading it first when needed.
pub fn engine_started() {
    let engine: Retained<AnyObject> = unsafe { msg_send![class!(ArdiaCoreEngine), sharedEngine] };
    let loaded: Option<Retained<AnyObject>> = unsafe { msg_send![&*engine, defaultProfile] };
    if let Some(profile) = loaded {
        go(engine, profile);
        return;
    }
    let again = engine.clone();
    let block = RcBlock::new(move |profile: *mut AnyObject| {
        match unsafe { Retained::retain(profile) } {
            Some(profile) => go(again.clone(), profile),
            None => {
                emit("{\"event\":\"error\",\"detail\":\"loadProfile returned nil\"}");
                std::process::exit(2);
            }
        }
    });
    let name = NSString::from_str("Default");
    let _: () = unsafe { msg_send![&*engine, loadProfile: &*name, completion: &*block] };
}

fn go(engine: Retained<AnyObject>, profile: Retained<AnyObject>) {
    let (kind, url, started) = CONFIG.with(|c| {
        let c = c.borrow();
        let c = c.as_ref().expect("config set before the engine runs");
        (c.kind, c.page_url.clone(), c.started)
    });
    begin(kind, &url, started, engine, profile);
}

/// The engine asked to shut down: say so, as the proof's "quit clean" evidence.
pub fn engine_will_shut_down() {
    CTX.with(|c| {
        if let Ok(guard) = c.try_borrow() {
            if let Some(ctx) = guard.as_ref() {
                ctx.ev("engine_will_shut_down", &format!("\"since_quit_request_ms\":{:.1}", ctx.quit_at.map_or(-1.0, |t| t_ms(t))));
            }
        }
    });
}

/// NSGlassEffectView (macOS 26) when the system has it, else a blurring NSVisualEffectView.
fn make_glass(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSView> {
    if let Some(class) = AnyClass::get(c"NSGlassEffectView") {
        // The class is an NSView subclass, so `initWithFrame:` reaches its own init.
        let alloc: Allocated<NSView> = unsafe { msg_send![class, alloc] };
        let glass = NSView::initWithFrame(alloc, frame);
        let _: () = unsafe { msg_send![&*glass, setCornerRadius: 22.0f64] };
        emit("{\"event\":\"glass\",\"class\":\"NSGlassEffectView\"}");
        return glass;
    }
    let v = NSVisualEffectView::initWithFrame(mtm.alloc(), frame);
    v.setMaterial(NSVisualEffectMaterial::HUDWindow);
    v.setBlendingMode(NSVisualEffectBlendingMode::WithinWindow);
    v.setState(NSVisualEffectState::Active);
    v.setWantsLayer(true);
    emit("{\"event\":\"glass\",\"class\":\"NSVisualEffectView\"}");
    Retained::into_super(v)
}

fn start_timer() {
    let block = RcBlock::new(|_: *mut AnyObject| on_timer());
    let timer: Retained<AnyObject> =
        unsafe { msg_send![class!(NSTimer), scheduledTimerWithTimeInterval: 0.05f64, repeats: true, block: &*block] };
    std::mem::forget(timer);
    std::mem::forget(block);
}

fn on_timer() {
    CTX.with(|c| {
        let Ok(mut guard) = c.try_borrow_mut() else { return };
        let Some(ctx) = guard.as_mut() else { return };
        if let Some(at) = ctx.quit_at {
            if at.elapsed() > Duration::from_secs(20) {
                ctx.ev("quit_timeout", "");
                std::process::exit(3);
            }
            return;
        }
        if ctx.started.elapsed() > Duration::from_secs(150) {
            ctx.check("watchdog", false, "scenario did not finish in 150 s");
            ctx.quit();
            return;
        }
        // Without a Rune view the timer drives the steps; with one, its display link does.
        if ctx.rune.is_none() || matches!(ctx.step, Step::WaitLoad | Step::Quit | Step::Finish) {
            ctx.advance();
        }
    });
}

/// Called by the Rune view on each display-link tick, before it draws.
pub fn on_tick(_view: &RuneView) {
    CTX.with(|c| {
        let Ok(mut guard) = c.try_borrow_mut() else { return };
        if let Some(ctx) = guard.as_mut() {
            ctx.advance();
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Synthetic events: real NSEvent objects, delivered the way AppKit would.

fn uptime() -> f64 {
    NSProcessInfo::processInfo().systemUptime()
}

fn mouse(win: &NSWindow, kind: NSEventType, at_window: NSPoint, clicks: isize) -> Retained<NSEvent> {
    NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        kind,
        at_window,
        NSEventModifierFlags::empty(),
        uptime(),
        win.windowNumber(),
        None,
        0,
        clicks,
        1.0,
    )
    .expect("mouse event")
}

fn key(win: &NSWindow, down: bool, chars: &str, code: u16, flags: NSEventModifierFlags) -> Retained<NSEvent> {
    let s = NSString::from_str(chars);
    NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        if down { NSEventType::KeyDown } else { NSEventType::KeyUp },
        NSPoint::ZERO,
        flags,
        uptime(),
        win.windowNumber(),
        None,
        &s,
        &s,
        false,
        code,
    )
    .expect("key event")
}

unsafe extern "C" {
    fn CGEventCreateScrollWheelEvent2(source: *const std::ffi::c_void, units: u32, count: u32, w1: i32, w2: i32, w3: i32) -> *mut std::ffi::c_void;
    fn CGEventSetLocation(event: *mut std::ffi::c_void, at: NSPoint);
    fn CFRelease(object: *const std::ffi::c_void);
}

/// A pixel-precise scroll event at a screen point, as a trackpad makes one.
fn scroll(screen: NSPoint, dy: i32) -> Option<Retained<NSEvent>> {
    let primary_height = objc2_app_kit::NSScreen::screens(MainThreadMarker::new()?).firstObject()?.frame().size.height;
    // SAFETY: a fresh CGEvent, handed to NSEvent (which retains it) and then released.
    unsafe {
        let cg = CGEventCreateScrollWheelEvent2(std::ptr::null(), 1, 2, dy, 0, 0);
        if cg.is_null() {
            return None;
        }
        CGEventSetLocation(cg, NSPoint::new(screen.x, primary_height - screen.y));
        let event: Option<Retained<NSEvent>> = msg_send![class!(NSEvent), eventWithCGEvent: cg];
        CFRelease(cg);
        event
    }
}

impl Ctx {
    fn rune(&self) -> &RuneView {
        self.rune.as_ref().expect("rune view")
    }

    /// A point in the Rune view's coordinates, as window coordinates.
    fn to_window(&self, p: (f32, f32)) -> NSPoint {
        self.rune().convertPoint_toView(NSPoint::new(p.0 as f64, p.1 as f64), None)
    }

    fn center(&self, key: &str) -> Option<(f32, f32)> {
        element_center(self.rune(), key)
    }



    fn advance(&mut self) {
        match self.step {
            Step::WaitLoad => self.wait_load(),
            Step::WaitRune => {
                let v = self.rune();
                if v.host_ready() && v.ticks() > 20 && v.probe().pointer.is_none() && element_center(v, "inc").is_some() {
                    let hz = v.stats().refresh_hz;
                    let err = v.last_error();
                    self.ev("rune_ready", &format!("\"display_hz\":{hz:.1},\"ticks\":{},\"error\":{}", self.rune().ticks(), json(&err)));
                    self.check("rune_first_frame", true, "layer host drew a frame");
                    self.step = if self.kind == Kind::B { Step::Steady } else { Step::Transparency };
                } else if self.started.elapsed() > Duration::from_secs(40) {
                    let err = self.rune().last_error();
                    self.check("rune_first_frame", false, &format!("no frame in 40 s: {err}"));
                    self.step = Step::Finish;
                }
            }
            Step::Steady => {
                if self.sub == 0 {
                    self.rune().stats().begin("steady_page_and_rune_animating");
                    self.sub = 1;
                    self.until = Instant::now() + Duration::from_secs(5);
                } else if self.ready() {
                    self.step = Step::SteadyEnd;
                }
            }
            Step::SteadyEnd => {
                if let Some(line) = self.rune().stats().end() {
                    emit(&line);
                }
                self.sub = 0;
                self.step = Step::PointerMove;
            }
            Step::PointerMove => self.pointer_move(),
            Step::Click => self.click(),
            Step::Wheel => self.wheel(),
            Step::FocusField => self.focus_field(),
            Step::TypeKeys => self.type_keys(),
            Step::EditKeys => self.edit_keys(),
            Step::Ime => self.ime(),
            Step::Resize => self.resize(),
            Step::ResizeEnd => self.resize_end(),
            Step::Transparency => self.transparency(),
            Step::GlassShot => self.shot(Step::ClickAbove, "c-glass-under-rune-layer"),
            Step::ClickAbove => self.click_above(),
            Step::AxSelfCheck => self.ax_self_check(),
            Step::AxHold => {
                if self.sub == 0 {
                    self.ev("ax_hold", &format!("\"pid\":{}", std::process::id()));
                    self.sub = 1;
                    self.until = Instant::now() + Duration::from_secs(12);
                } else if self.ready() {
                    self.sub = 0;
                    self.step = Step::Finish;
                }
            }
            Step::Shot => self.shot(Step::Finish, "b-rune-beside-page"),
            Step::Finish => {
                if self.rune.is_some() && self.ready() || self.rune.is_none() {
                    self.step = Step::Quit;
                }
            }
            Step::Quit => self.finish(),
            Step::Done => {}
        }
    }

    fn wait_load(&mut self) {
        let loading: bool = unsafe { msg_send![&*self.tab, loading] };
        let Some((delegate_ms, title)) = TAB_TITLE.with(|t| t.borrow().clone()) else { return };
        if loading || title != PAGE_TITLE {
            return;
        }
        let ms = t_ms(self.started);
        self.page_loaded_ms = Some(ms);
        let url: Retained<NSString> = unsafe { msg_send![&*self.tab, url] };
        let tabs: Retained<AnyObject> = unsafe { msg_send![&*self.window, tabsForProfile: &*self.profile] };
        let count: usize = unsafe { msg_send![&*tabs, count] };
        let app = NSApplication::sharedApplication(MainThreadMarker::new().expect("main"));
        let windows = app.windows();
        let visible = windows.iter().filter(|w| w.isVisible()).count();
        self.ev(
            "page_loaded",
            &format!(
                "\"url\":{},\"title\":{},\"delegate_title_ms\":{delegate_ms:.1},\"tabs\":{count},\"app_windows\":{},\"visible_windows\":{visible}",
                json(&url.to_string()),
                json(&title),
                windows.len()
            ),
        );
        self.check("one_tab", count == 1, &format!("tabs={count}"));
        self.check("delegate_title", true, &format!("tabDidChangeTitle delivered {title:?} at {delegate_ms:.1} ms"));
        self.step = if self.kind == Kind::A { Step::Finish } else { Step::WaitRune };
        self.until = Instant::now();
    }

    fn pointer_move(&mut self) {
        if self.sub == 0 {
            let Some(c) = self.center("inc") else { return };
            let at = self.to_window(c);
            let ev = mouse(&self.nswindow, NSEventType::MouseMoved, at, 0);
            self.rune().mouseMoved(&ev);
            self.sub = 1;
            self.until = Instant::now() + Duration::from_millis(300);
        } else if self.ready() {
            let p = self.rune().probe();
            let c = self.center("inc").unwrap_or((0.0, 0.0));
            let near = p.pointer.is_some_and(|q| (q.0 - c.0).abs() < 1.5 && (q.1 - c.1).abs() < 1.5);
            self.check("pointer_move", near && p.hover, &format!("runtime pointer={:?} button hover={} expected {:?}", p.pointer, p.hover, c));
            self.sub = 0;
            self.step = Step::Click;
        }
    }

    fn click(&mut self) {
        if self.sub == 0 {
            let Some(c) = self.center("inc") else { return };
            let at = self.to_window(c);
            self.nswindow.sendEvent(&mouse(&self.nswindow, NSEventType::LeftMouseDown, at, 1));
            self.nswindow.sendEvent(&mouse(&self.nswindow, NSEventType::LeftMouseUp, at, 1));
            self.sub = 1;
            self.until = Instant::now() + Duration::from_millis(300);
        } else if self.ready() {
            let p = self.rune().probe();
            self.check("pointer_click", p.presses == 1, &format!("button press count={}", p.presses));
            self.sub = 0;
            self.step = Step::Wheel;
        }
    }

    fn wheel(&mut self) {
        let Some(list) = self.center("scroll") else { return };
        if self.sub == 0 {
            self.sub = 1;
            self.until = Instant::now();
        }
        if self.sub <= 6 && self.ready() {
            let at = self.to_window(list);
            let screen = self.nswindow.convertPointToScreen(at);
            if let Some(ev) = scroll(screen, -40) {
                self.rune().scrollWheel(&ev);
            }
            self.sub += 1;
            self.until = Instant::now() + Duration::from_millis(30);
        } else if self.sub > 6 && self.ready() {
            let y = self.rune().probe().scroll_y;
            self.check("wheel_scroll", y > 100.0, &format!("list scroll offset {y:.1} points after 6 x -40 px wheel events"));
            self.sub = 0;
            self.step = Step::FocusField;
        }
    }

    fn focus_field(&mut self) {
        if self.sub == 0 {
            let Some(c) = self.center("field") else { return };
            let at = self.to_window(c);
            self.nswindow.sendEvent(&mouse(&self.nswindow, NSEventType::LeftMouseDown, at, 1));
            self.nswindow.sendEvent(&mouse(&self.nswindow, NSEventType::LeftMouseUp, at, 1));
            self.sub = 1;
            self.until = Instant::now() + Duration::from_millis(300);
        } else if self.ready() {
            let p = self.rune().probe();
            let first: bool = self.nswindow.firstResponder().is_some_and(|r| std::ptr::eq(&*r as *const _ as *const u8, &**self.rune() as *const _ as *const u8));
            self.check("text_field_focus", p.focus && first, &format!("runtime focus={} view is first responder={first}", p.focus));
            self.sub = 0;
            self.step = Step::TypeKeys;
        }
    }

    fn type_keys(&mut self) {
        if self.sub == 0 {
            for (c, code) in [("h", 4u16), ("i", 34)] {
                self.nswindow.sendEvent(&key(&self.nswindow, true, c, code, NSEventModifierFlags::empty()));
                self.nswindow.sendEvent(&key(&self.nswindow, false, c, code, NSEventModifierFlags::empty()));
            }
            self.sub = 1;
            self.until = Instant::now() + Duration::from_millis(300);
        } else if self.ready() {
            let p = self.rune().probe();
            self.check("key_text", p.field == "hi", &format!("field text {:?}, preedit {:?}", p.field, p.preedit));
            self.sub = 0;
            self.step = Step::EditKeys;
        }
    }

    fn edit_keys(&mut self) {
        let none = NSEventModifierFlags::empty();
        match self.sub {
            0 => {
                self.nswindow.sendEvent(&key(&self.nswindow, true, "\u{7f}", 51, none));
                self.nswindow.sendEvent(&key(&self.nswindow, false, "\u{7f}", 51, none));
                self.sub = 1;
                self.until = Instant::now() + Duration::from_millis(250);
            }
            1 if self.ready() => {
                let p = self.rune().probe();
                self.check("key_backspace", p.field == "h", &format!("field text {:?} after Backspace", p.field));
                let cmd = NSEventModifierFlags::Command;
                self.nswindow.sendEvent(&key(&self.nswindow, true, "a", 0, cmd));
                self.nswindow.sendEvent(&key(&self.nswindow, false, "a", 0, cmd));
                self.nswindow.sendEvent(&key(&self.nswindow, true, "x", 7, none));
                self.nswindow.sendEvent(&key(&self.nswindow, false, "x", 7, none));
                self.sub = 2;
                self.until = Instant::now() + Duration::from_millis(250);
            }
            2 if self.ready() => {
                let p = self.rune().probe();
                self.check("key_select_all_replace", p.field == "x", &format!("field text {:?} after Cmd+A then x", p.field));
                self.nswindow.sendEvent(&key(&self.nswindow, true, "\u{f702}", 123, none));
                self.nswindow.sendEvent(&key(&self.nswindow, false, "\u{f702}", 123, none));
                self.nswindow.sendEvent(&key(&self.nswindow, true, "y", 16, none));
                self.nswindow.sendEvent(&key(&self.nswindow, false, "y", 16, none));
                self.sub = 3;
                self.until = Instant::now() + Duration::from_millis(250);
            }
            3 if self.ready() => {
                let p = self.rune().probe();
                self.check("key_arrow_left", p.field == "yx", &format!("field text {:?} after Left then y", p.field));
                self.sub = 0;
                self.step = Step::Ime;
            }
            _ => {}
        }
    }

    /// The calls an input method makes on the view's NSTextInputClient: marked text, then commit.
    fn ime(&mut self) {
        let none_range = NSRange { location: NSNotFound as usize, length: 0 };
        let v = self.rune.clone().expect("rune view");
        match self.sub {
            0 => {
                let s = NSString::from_str("ni");
                let _: () = unsafe { msg_send![&*v, setMarkedText: &*s, selectedRange: NSRange { location: 2, length: 0 }, replacementRange: none_range] };
                self.sub = 1;
                self.until = Instant::now() + Duration::from_millis(250);
            }
            1 if self.ready() => {
                let p = v.probe();
                let marked: bool = unsafe { msg_send![&*v, hasMarkedText] };
                self.check("ime_preedit", marked && p.composing && p.preedit.as_deref() == Some("ni"), &format!("hasMarkedText={marked} runtime preedit {:?}", p.preedit));
                let s = NSString::from_str("你");
                let _: () = unsafe { msg_send![&*v, setMarkedText: &*s, selectedRange: NSRange { location: 1, length: 0 }, replacementRange: none_range] };
                self.sub = 2;
                self.until = Instant::now() + Duration::from_millis(250);
            }
            2 if self.ready() => {
                let p = v.probe();
                self.check("ime_preedit_replaced", p.preedit.as_deref() == Some("你"), &format!("runtime preedit {:?}", p.preedit));
                let s = NSString::from_str("你好");
                let _: () = unsafe { msg_send![&*v, insertText: &*s, replacementRange: none_range] };
                self.sub = 3;
                self.until = Instant::now() + Duration::from_millis(250);
            }
            3 if self.ready() => {
                let p = v.probe();
                let rect: NSRect = unsafe { msg_send![&*v, firstRectForCharacterRange: none_range, actualRange: std::ptr::null_mut::<NSRange>()] };
                self.check("ime_commit", p.field.contains("你好") && !p.composing, &format!("field text {:?} composing={}", p.field, p.composing));
                self.check("ime_candidate_rect", rect.size.height > 0.0, &format!("firstRectForCharacterRange {rect:?}"));
                self.sub = 0;
                self.step = Step::Resize;
            }
            _ => {}
        }
    }

    /// Resizes the window like a live drag: the page view follows the window, the strip keeps its width.
    fn resize(&mut self) {
        if self.sub == 0 {
            self.base_frame = self.nswindow.frame();
            self.page_widths.clear();
            self.rune().stats().begin("window_resize_live");
            self.sub = 1;
            self.until = Instant::now();
        }

        let k = self.sub as f64;
        // 150 ticks of a smooth width swing: +260 points and back.
        let swing = (k / 150.0 * std::f64::consts::PI).sin() * 260.0;
        let mut f = self.base_frame;
        f.size.width += swing;
        self.nswindow.setFrame_display(f, true);
        self.page_widths.push(self.tab_view.frame().size.width);
        self.sub += 1;
        if self.sub > 150 {
            self.step = Step::ResizeEnd;
            self.sub = 0;
        }
    }

    fn resize_end(&mut self) {
        if let Some(line) = self.rune().stats().end() {
            emit(&line);
        }
        let (lo, hi) = self.page_widths.iter().fold((f64::MAX, f64::MIN), |(a, b), w| (a.min(*w), b.max(*w)));
        let strip = self.rune().frame().size;
        let layer: *mut AnyObject = unsafe { msg_send![&**self.rune(), layer] };
        let drawable: objc2_foundation::NSSize = unsafe { msg_send![layer, drawableSize] };
        let scale = self.nswindow.backingScaleFactor();
        let want = (strip.width * scale, strip.height * scale);
        self.check("resize_page_follows", hi - lo > 200.0, &format!("page view width ranged {lo:.0}..{hi:.0}"));
        self.check("resize_strip_fixed", (strip.width - STRIP_W).abs() < 0.5, &format!("strip width {:.1}", strip.width));
        self.check(
            "resize_drawable_matches",
            (drawable.width - want.0).abs() < 1.5 && (drawable.height - want.1).abs() < 1.5,
            &format!("drawableSize {:.0}x{:.0}, view x scale {:.0}x{:.0}", drawable.width, drawable.height, want.0, want.1),
        );
        self.wait(Step::Shot, 0.5);
    }

    fn transparency(&mut self) {
        let layer: *mut AnyObject = unsafe { msg_send![&**self.rune(), layer] };
        let opaque: bool = unsafe { msg_send![layer, isOpaque] };
        let p = self.rune().probe();
        let (g_view, r_view) = (self.glass.clone(), self.rune.clone());
        let order = {
            let subs = self.host_view.subviews();
            let idx = |v: &NSView| subs.iter().position(|s| std::ptr::eq(&*s as *const NSView, v as *const NSView));
            (idx(&self.tab_view), g_view.as_deref().and_then(idx), r_view.as_deref().map(|r| &**r).and_then(idx))
        };
        self.check("layer_transparent", !opaque && p.transparent, &format!("CAMetalLayer.isOpaque={opaque} runtime.transparent={}", p.transparent));
        let ordered = matches!(order, (Some(a), Some(b), Some(c)) if a < b && b < c);
        self.check("z_order_page_glass_rune", ordered, &format!("subview indices page/glass/rune = {order:?}"));
        self.wait(Step::GlassShot, 1.0);
    }

    fn shot(&mut self, next: Step, name: &str) {
        if self.sub == 0 {
            self.ev("shot", &format!("\"name\":{},\"window_id\":{}", json(name), self.nswindow.windowNumber()));
            self.sub = 1;
            self.until = Instant::now() + Duration::from_millis(2500);
        } else if self.ready() {
            self.sub = 0;
            self.step = next;
        }
    }

    /// A click inside the Rune layer reaches Rune and not the page under it.
    fn click_above(&mut self) {
        if self.sub == 0 {
            self.js_inflight = false;
            let Some(c) = self.center("inc") else { return };
            let at = self.to_window(c);
            self.nswindow.sendEvent(&mouse(&self.nswindow, NSEventType::LeftMouseDown, at, 1));
            self.nswindow.sendEvent(&mouse(&self.nswindow, NSEventType::LeftMouseUp, at, 1));
            self.sub = 1;
            self.until = Instant::now() + Duration::from_millis(300);
        } else if self.sub == 1 && self.ready() {
            let p = self.rune().probe();
            self.check("above_click_reaches_rune", p.presses == 1, &format!("button press count={}", p.presses));
            self.js("JSON.stringify({clicks:window.__clicks||0})");
            self.sub = 2;
        } else if self.sub == 2 {
            if let Some(v) = self.js_result() {
                let clicks = v["clicks"].as_u64().unwrap_or(99);
                self.check("above_click_not_in_page", clicks == 0, &format!("page mousedown count={clicks}"));
                self.sub = 0;
                self.step = Step::AxSelfCheck;
            }
        }
    }

    /// Walks the NSAccessibility tree of the view as an assistive client does.
    fn ax_self_check(&mut self) {
        let v = self.rune.clone().expect("rune view");
        let mut lines = Vec::new();
        ax_walk(&v, 0, &mut lines);
        match self.sub {
            0 => {
                // The first query makes AccessKit ask Rune for the whole tree.
                self.sub = 1;
                self.until = Instant::now() + Duration::from_millis(600);
            }
            _ if self.ready() => {
                let button = lines.iter().any(|l| l.contains("AXButton") && l.contains("Increment counter"));
                for l in &lines {
                    self.ev("ax_node", &format!("\"line\":{}", json(l)));
                }
                self.check("ax_button_with_label", button, &format!("{} nodes; AXButton labelled 'Increment counter' present={button}", lines.len()));
                self.sub = 0;
                self.step = Step::AxHold;
            }
            _ => {}
        }
    }

    fn finish(&mut self) {
        if self.rune.is_some() {
            let actions = self.rune().ax_actions();
            self.ev("ax_actions_received", &format!("\"count\":{}", actions.len()));
        }
        if self.kind == Kind::A {
            self.check("window_and_tab_stayed", true, "window opened and tab loaded before quit");
        }
        let pass = self.fails == 0;
        self.ev("result", &format!("\"pass\":{pass},\"checks\":{},\"fails\":{}", self.checks, self.fails));
        self.quit();
    }
}

fn ostr(obj: &AnyObject, selector: objc2::runtime::Sel) -> Option<String> {
    let responds: bool = unsafe { msg_send![obj, respondsToSelector: selector] };
    if !responds {
        return None;
    }
    let s: Option<Retained<AnyObject>> = unsafe { msg_send![obj, performSelector: selector] };
    let s = s?;
    let is_string: bool = unsafe { msg_send![&*s, isKindOfClass: class!(NSString)] };
    is_string.then(|| unsafe { (*(&*s as *const AnyObject as *const NSString)).to_string() })
}

fn ax_walk(obj: &AnyObject, depth: usize, out: &mut Vec<String>) {
    if depth > 6 {
        return;
    }
    let role = ostr(obj, sel!(accessibilityRole)).unwrap_or_default();
    let label = ostr(obj, sel!(accessibilityLabel)).unwrap_or_default();
    let title = ostr(obj, sel!(accessibilityTitle)).unwrap_or_default();
    if depth > 0 {
        out.push(format!("{}{role} label={label:?} title={title:?}", "  ".repeat(depth - 1)));
    }
    let children: Option<Retained<objc2_foundation::NSArray<AnyObject>>> = unsafe { msg_send![obj, accessibilityChildren] };
    if let Some(children) = children {
        for child in children.iter() {
            ax_walk(&child, depth + 1, out);
        }
    }
}
