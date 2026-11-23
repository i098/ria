//! The load case of plan section 6.7: 3 windows, 20 tabs in each on the fixture page, the sidebar open,
//! 1 tab that plays the fixture video. `--load 1` has 1 Rune layer per window (proof B),
//! `--load 2` has 2 (proof C). Three numbers: launch time to the first frame of the first window,
//! memory of all app processes after 60 s with no input, and the frame times of a 10 s sidebar open and close loop.

#![allow(unsafe_code)]

use std::cell::RefCell;
use std::process::Command;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSView, NSWindowOrderingMode};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::layer::{Mode, RuneView};
use crate::proof::{emit, t_ms};

const WINDOWS: usize = 3;
const TABS: usize = 20;
const SIDEBAR_W: f64 = 320.0;
const IDLE_SECS: u64 = 60;
const LOOP_SECS: f64 = 10.0;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Phase {
    WaitFirstFrame,
    Idle,
    Loop,
    Done,
}

struct Win {
    engine_window: Retained<AnyObject>,
    host_view: Retained<NSView>,
    tab_view: Retained<NSView>,
    strip: Retained<RuneView>,
    overlay: Option<Retained<RuneView>>,
}

struct Load {
    layers: u32,
    started: Instant,
    engine: Retained<AnyObject>,
    wins: Vec<Win>,
    phase: Phase,
    phase_at: Instant,
    ticks: u64,
}

thread_local! {
    static LOAD: RefCell<Option<Load>> = const { RefCell::new(None) };
}

fn ev(started: Instant, name: &str, fields: &str) {
    emit(&format!("{{\"t_ms\":{:.1},\"proof\":\"load\",\"event\":\"{name}\",{fields}}}", t_ms(started)));
}

pub fn begin(layers: u32, page_url: &str, started: Instant, engine: Retained<AnyObject>, profile: Retained<AnyObject>) {
    let mtm = MainThreadMarker::new().expect("main thread");
    let video_url = page_url.replace("page.html", "page-video.html");
    let window_class = AnyClass::get(c"ArdiaCoreWindow").expect("ArdiaCore is loaded");
    let grow = NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable;
    let mut wins = Vec::new();
    for w in 0..WINDOWS {
        let rect = NSRect::new(NSPoint::new(40.0 + 60.0 * w as f64, 60.0 + 40.0 * w as f64), NSSize::new(1040.0, 640.0));
        let alloc: Allocated<AnyObject> = unsafe { msg_send![window_class, alloc] };
        let window: Retained<AnyObject> = unsafe { msg_send![alloc, initWithContentRect: rect] };
        let mut first: Option<Retained<AnyObject>> = None;
        for t in 0..TABS {
            let url = NSString::from_str(if w == 0 && t == 0 { &video_url } else { page_url });
            let tab: Retained<AnyObject> = unsafe { msg_send![&*window, openTab: &*url, profile: &*profile, foreground: t == 0] };
            if t == 0 {
                first = Some(tab);
            }
        }
        let tab = first.expect("20 tabs");
        let host_view: Retained<NSView> = unsafe { msg_send![&*window, hostView] };
        let tab_view: Retained<NSView> = unsafe { msg_send![&*tab, view] };
        let bounds = host_view.bounds();
        tab_view.setFrame(NSRect::new(NSPoint::ZERO, NSSize::new(bounds.size.width - SIDEBAR_W, bounds.size.height)));
        tab_view.setAutoresizingMask(grow);
        host_view.addSubview(&tab_view);
        let _: () = unsafe { msg_send![&*window, activateTab: &*tab] };
        let _: () = unsafe { msg_send![&*window, showInactive] };
        let strip_frame = NSRect::new(NSPoint::new(bounds.size.width - SIDEBAR_W, 0.0), NSSize::new(SIDEBAR_W, bounds.size.height));
        let strip = RuneView::new(mtm, Mode::Beside, strip_frame);
        strip.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin | NSAutoresizingMaskOptions::ViewHeightSizable);
        host_view.addSubview(&strip);
        let overlay = (layers >= 2).then(|| {
            let frame = NSRect::new(NSPoint::new(24.0, 24.0), NSSize::new(340.0, 220.0));
            let v = RuneView::new(mtm, Mode::Above, frame);
            v.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMaxXMargin | NSAutoresizingMaskOptions::ViewMaxYMargin);
            host_view.addSubview_positioned_relativeTo(&v, NSWindowOrderingMode::Above, Some(&tab_view));
            v
        });
        wins.push(Win { engine_window: window, host_view, tab_view, strip, overlay });
    }
    ev(started, "windows_opened", &format!("\"windows\":{WINDOWS},\"tabs_each\":{TABS},\"layers\":{layers}"));
    LOAD.with(|l| {
        *l.borrow_mut() = Some(Load {
            layers,
            started,
            engine,
            wins,
            phase: Phase::WaitFirstFrame,
            phase_at: Instant::now(),
            ticks: 0,
        })
    });
    let block = RcBlock::new(|_: *mut AnyObject| on_timer());
    let timer: Retained<AnyObject> =
        unsafe { msg_send![objc2::class!(NSTimer), scheduledTimerWithTimeInterval: 0.1f64, repeats: true, block: &*block] };
    std::mem::forget(timer);
    std::mem::forget(block);
}

/// Memory of this process and all its descendants in MB: (resident sum, physical footprint sum, process count).
fn tree_memory() -> (f64, f64, usize) {
    let out = Command::new("ps").args(["-axo", "pid=,ppid=,rss="]).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let rows: Vec<(u32, u32, u64)> = out
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace().map(|x| x.parse::<u64>().ok());
            Some((f.next()??.try_into().ok()?, f.next()??.try_into().ok()?, f.next()??))
        })
        .collect();
    let mut set = vec![std::process::id()];
    loop {
        let before = set.len();
        for (pid, ppid, _) in &rows {
            if set.contains(ppid) && !set.contains(pid) {
                set.push(*pid);
            }
        }
        if set.len() == before {
            break;
        }
    }
    let kb: u64 = rows.iter().filter(|(p, _, _)| set.contains(p)).map(|r| r.2).sum();
    let mut cmd = Command::new("footprint");
    cmd.arg("--noCategories");
    for p in &set {
        cmd.args(["-p", &p.to_string()]);
    }
    let text = cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    // Header lines look like `name [pid]: 64-bit    Footprint: 123 MB (16384 bytes per page)`.
    let mut footprint = 0.0;
    for l in text.lines().filter(|l| l.contains("]:") && l.contains("Footprint:")) {
        let mut it = l.split("Footprint:").nth(1).unwrap_or("").split_whitespace();
        let n: f64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
        footprint += n * match it.next() {
            Some("GB") => 1024.0,
            Some("MB") => 1.0,
            Some("KB") => 1.0 / 1024.0,
            _ => 0.0,
        };
    }
    (kb as f64 / 1024.0, footprint, set.len())
}

fn on_timer() {
    LOAD.with(|l| {
        let Ok(mut guard) = l.try_borrow_mut() else { return };
        let Some(s) = guard.as_mut() else { return };
        match s.phase {
            Phase::WaitFirstFrame => {
                let v = &s.wins[0].strip;
                if v.host_ready() && v.ticks() > 0 {
                    ev(s.started, "first_frame", &format!("\"launch_ms\":{:.1}", t_ms(s.started)));
                    s.phase = Phase::Idle;
                    s.phase_at = Instant::now();
                } else if s.started.elapsed() > Duration::from_secs(60) {
                    ev(s.started, "error", "\"detail\":\"no first frame in 60 s\"");
                    s.phase = Phase::Done;
                    unsafe { let _: () = msg_send![&*s.engine, quit]; }
                }
            }
            Phase::Idle => {
                if s.phase_at.elapsed() >= Duration::from_secs(IDLE_SECS) {
                    let (mb, fp, n) = tree_memory();
                    let ticks: Vec<u64> = s.wins.iter().map(|w| w.strip.ticks()).collect();
                    ev(s.started, "memory", &format!("\"after_s\":{IDLE_SECS},\"rss_mb\":{mb:.1},\"footprint_mb\":{fp:.1},\"processes\":{n},\"layers\":{},\"strip_ticks\":{ticks:?}", s.layers));
                    s.wins[0].strip.stats().begin("sidebar_loop");
                    s.phase = Phase::Loop;
                    s.phase_at = Instant::now();
                }
            }
            Phase::Loop => {
                if s.phase_at.elapsed().as_secs_f64() >= LOOP_SECS {
                    let line = s.wins[0].strip.stats().end();
                    if let Some(line) = line {
                        emit(&line);
                    }
                    ev(s.started, "result", &format!("\"layers\":{}", s.layers));
                    s.phase = Phase::Done;
                    unsafe { let _: () = msg_send![&*s.engine, quit]; }
                }
            }
            Phase::Done => {}
        }
    });
}

/// Called on each display-link tick of any Rune view; the first window's strip drives the sidebar animation of all windows.
pub fn on_tick(view: &RuneView) {
    LOAD.with(|l| {
        let Ok(mut guard) = l.try_borrow_mut() else { return };
        let Some(s) = guard.as_mut() else { return };
        if s.phase != Phase::Loop || !std::ptr::eq(&*s.wins[0].strip, view) {
            return;
        }
        s.ticks += 1;
        // One open-and-close cycle takes 2 s: the width follows a cosine between 0 and the full sidebar.
        let t = s.phase_at.elapsed().as_secs_f64();
        let width = SIDEBAR_W * (0.5 + 0.5 * (std::f64::consts::TAU * t / 2.0).cos());
        for w in &s.wins {
            let b = w.host_view.bounds();
            w.tab_view.setFrame(NSRect::new(NSPoint::ZERO, NSSize::new(b.size.width - width, b.size.height)));
            w.strip.setFrame(NSRect::new(NSPoint::new(b.size.width - width, 0.0), NSSize::new(width.max(1.0), b.size.height)));
            let _ = (&w.engine_window, &w.overlay);
        }
    });
}


