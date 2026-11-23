// Ported from ardia packages/ardiacore/ios/ArdiaPageCursor.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::rc::{Rc, Weak as RcWeak};

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, NSObjectProtocol, Sel};
use objc2::{ClassType, MainThreadMarker, Message, sel};
use objc2_app_kit::{
    NSBitmapImageRep, NSClipView, NSCompositingOperation, NSCursor, NSDeviceRGBColorSpace, NSEvent,
    NSEventMask, NSGraphicsContext, NSImage, NSScrollView, NSTextField, NSTextView, NSView,
    NSWindow,
};
use objc2_foundation::{NSBundle, NSPoint, NSRect, NSSize};
use ria_engine::call::main_queue;

pub enum SurfaceCursor {
    Inherit,
    Owned,
    Set(Retained<NSCursor>),
}

#[derive(Default)]
pub struct CursorStats {
    pub restores: usize,
    pub hidden_restores: usize,
    pub covered_page_cursors_held: usize,
    pub last_restore: Option<String>,
}

type WindowQuery = dyn Fn(&NSWindow) -> bool;
type CursorQuery = dyn Fn(&NSView) -> SurfaceCursor;
struct Watch {
    last_page: RefCell<Option<Weak<NSCursor>>>,
    visibility: RefCell<Option<(Weak<NSCursor>, bool)>>,
    stats: RefCell<CursorStats>,
    background: bool,
    is_surface: Box<WindowQuery>,
    is_hidden_page: Box<WindowQuery>,
    surface_cursor: Box<CursorQuery>,
    restored: Rc<dyn Fn(bool)>,
}

thread_local! {
    static ACTIVE: RefCell<RcWeak<Watch>> = const { RefCell::new(RcWeak::new()) };
    static ORIGINALS: std::cell::Cell<Option<(Imp, Imp)>> = const { std::cell::Cell::new(None) };
}

pub struct PageCursor {
    watch: Rc<Watch>,
    token: Retained<AnyObject>,
}

impl PageCursor {
    pub fn install(
        _mtm: MainThreadMarker,
        background: bool,
        is_surface: impl Fn(&NSWindow) -> bool + 'static,
        is_hidden_page: impl Fn(&NSWindow) -> bool + 'static,
        surface_cursor: impl Fn(&NSView) -> SurfaceCursor + 'static,
        restored: impl Fn(bool) + 'static,
    ) -> Option<Self> {
        if ACTIVE.with(|active| active.borrow().upgrade().is_some()) {
            return None;
        }
        install_hooks()?;
        let watch = Rc::new(Watch {
            last_page: RefCell::default(),
            visibility: RefCell::default(),
            stats: RefCell::default(),
            background,
            is_surface: Box::new(is_surface),
            is_hidden_page: Box::new(is_hidden_page),
            surface_cursor: Box::new(surface_cursor),
            restored: Rc::new(restored),
        });
        let weak = Rc::downgrade(&watch);
        // SAFETY: AppKit supplies a non-null event for the monitor and retains the block.
        let token = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                NSEventMask::MouseMoved,
                &RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| {
                    let deferred = weak.clone();
                    // SAFETY: AppKit lends the event for the callback; retain it until the deferred work runs.
                    let retained = event.as_ref().retain();
                    main_queue::post(Box::new(move || {
                        if let Some(watch) = deferred.upgrade() {
                            watch.restore(&retained);
                        }
                    }));
                    event.as_ptr()
                }),
            )
        }?;
        ACTIVE.with(|active| *active.borrow_mut() = Rc::downgrade(&watch));
        Some(Self { watch, token })
    }

    pub fn diagnostics(&self, argument: &str) -> String {
        if argument == "hide" {
            let image = NSImage::initWithSize(objc2::AnyThread::alloc(), NSSize::new(1.0, 1.0));
            NSCursor::initWithImage_hotSpot(objc2::AnyThread::alloc(), &image, NSPoint::ZERO).set();
        } else if argument == "reset" {
            *self.watch.stats.borrow_mut() = CursorStats::default();
        }
        let current = cursor_name(&NSCursor::currentCursor());
        let stats = self.watch.stats.borrow();
        format!(
            "cursor {current}, restores {} (hidden {}), covered page cursors held {}, last restore {}",
            stats.restores,
            stats.hidden_restores,
            stats.covered_page_cursors_held,
            stats.last_restore.as_deref().unwrap_or("none")
        )
    }
}

impl Drop for PageCursor {
    fn drop(&mut self) {
        // SAFETY: AppKit returned this token from its local monitor registration.
        unsafe { NSEvent::removeMonitor(&self.token) };
    }
}

impl Watch {
    fn is_surface(&self, window: &NSWindow, hit: &NSView) -> bool {
        if (self.is_surface)(window) {
            return true;
        }
        let main = NSBundle::mainBundle();
        let mut view = Some(hit.retain());
        while let Some(v) = view {
            // SAFETY: The view supplies its registered Objective-C class.
            let bundle = unsafe { NSBundle::bundleForClass(v.class()) };
            if std::ptr::eq(&*bundle, &*main) {
                return true;
            }
            view = super::superview(&v);
        }
        false
    }

    fn restore(&self, event: &NSEvent) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let current = NSCursor::currentCursor();
        if current == NSCursor::arrowCursor() {
            return;
        }
        let Some(window) = event
            .window(mtm)
            .filter(|window| !(self.is_hidden_page)(window))
        else {
            return;
        };
        if !self.background
            && NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(
                NSEvent::mouseLocation(),
                0,
                mtm,
            ) != window.windowNumber()
        {
            return;
        }
        let Some(content) = window.contentView() else {
            return;
        };
        let Some(hit) = view_at(&window, event.locationInWindow()) else {
            return;
        };
        if enclosing_page(&hit) || !self.is_surface(&window, &hit) {
            return;
        }
        let cached = self
            .visibility
            .borrow()
            .as_ref()
            .and_then(|(cursor, hidden)| {
                cursor
                    .load()
                    .filter(|cursor| std::ptr::eq(&**cursor, &*current))
                    .map(|_| *hidden)
            });
        let hidden = cached.unwrap_or_else(|| {
            let hidden = invisible(&current.image());
            *self.visibility.borrow_mut() = Some((Weak::new(&*current), hidden));
            hidden
        });
        if !hidden
            && self
                .last_page
                .borrow()
                .as_ref()
                .and_then(Weak::load)
                .as_deref()
                != Some(&*current)
        {
            return;
        }
        let wanted = match self.cursor_of(&hit, &content) {
            SurfaceCursor::Owned if !hidden => return,
            SurfaceCursor::Set(cursor) => cursor,
            _ => NSCursor::arrowCursor(),
        };
        if wanted == current {
            return;
        }
        wanted.set();
        let description = format!(
            "{} -> {} over {}",
            cursor_name(&current),
            cursor_name(&wanted),
            hit.class().name().to_string_lossy()
        );
        {
            let mut stats = self.stats.borrow_mut();
            stats.restores += 1;
            stats.hidden_restores += usize::from(hidden);
            stats.last_restore = Some(description);
        }
        (self.restored)(hidden);
    }

    fn cursor_of(&self, hit: &NSView, content: &NSView) -> SurfaceCursor {
        let mut view = Some(hit.retain());
        while let Some(v) = view {
            if std::ptr::eq(&*v, content) {
                break;
            }
            match (self.surface_cursor)(&v) {
                SurfaceCursor::Inherit => {}
                cursor => return cursor,
            }
            if let Some(text) = v.downcast_ref::<NSTextView>() {
                if text.isSelectable() {
                    return SurfaceCursor::Set(NSCursor::IBeamCursor());
                }
            } else if let Some(field) = v.downcast_ref::<NSTextField>() {
                if field.isEditable() || field.isSelectable() {
                    return SurfaceCursor::Set(NSCursor::IBeamCursor());
                }
            } else if let Some(scroll) = v.downcast_ref::<NSScrollView>() {
                if let Some(cursor) = scroll.documentCursor() {
                    return SurfaceCursor::Set(cursor);
                }
            } else if let Some(clip) = v.downcast_ref::<NSClipView>() {
                if let Some(cursor) = clip.documentCursor() {
                    return SurfaceCursor::Set(cursor);
                }
            } else if [sel!(resetCursorRects), sel!(cursorUpdate:)]
                .into_iter()
                .any(|method| {
                    match (
                        v.class().instance_method(method),
                        NSView::class().instance_method(method),
                    ) {
                        (Some(actual), Some(base)) => {
                            !std::ptr::fn_addr_eq(actual.implementation(), base.implementation())
                        }
                        (None, None) => false,
                        _ => true,
                    }
                })
            {
                return SurfaceCursor::Owned;
            }
            view = super::superview(&v);
        }
        SurfaceCursor::Inherit
    }
}

fn view_at(window: &NSWindow, point: NSPoint) -> Option<Retained<NSView>> {
    let root = window.contentView()?;
    let parent = super::superview(&root)?;
    root.hitTest(parent.convertPoint_fromView(point, None))
}

fn page_class() -> Option<&'static AnyClass> {
    AnyClass::get(c"RenderWidgetHostViewCocoa")
}

fn enclosing_page(hit: &NSView) -> bool {
    let Some(class) = page_class() else {
        return false;
    };
    let mut view = Some(hit.retain());
    while let Some(v) = view {
        if v.isKindOfClass(class) {
            return true;
        }
        view = super::superview(&v);
    }
    false
}

fn invisible(image: &NSImage) -> bool {
    let size = image.size();
    if size.width < 1.0 || size.height < 1.0 || image.representations().count() == 0 {
        return true;
    }
    const SIDE: isize = 64;
    // SAFETY: Null planes ask AppKit to allocate the requested packed RGBA bitmap.
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
        objc2::AnyThread::alloc(), std::ptr::null_mut(), SIDE, SIDE, 8, 4, true, false, NSDeviceRGBColorSpace, SIDE * 4, 32,
    )
    };
    let Some(bitmap) = bitmap else { return false };
    let Some(context) = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap) else {
        return false;
    };
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));
    image.drawInRect_fromRect_operation_fraction(
        NSRect::new(NSPoint::ZERO, NSSize::new(SIDE as f64, SIDE as f64)),
        NSRect::ZERO,
        NSCompositingOperation::Copy,
        1.0,
    );
    NSGraphicsContext::restoreGraphicsState_class();
    // SAFETY: The live bitmap contains 64 rows with 256 bytes each.
    let data = bitmap.bitmapData();
    !data.is_null()
        && unsafe { std::slice::from_raw_parts(data, (SIDE * SIDE * 4) as usize) }
            .chunks_exact(4)
            .all(|pixel| pixel[3] == 0)
}

fn cursor_name(cursor: &NSCursor) -> String {
    for (name, known) in [
        ("arrow", NSCursor::arrowCursor()),
        ("iBeam", NSCursor::IBeamCursor()),
        ("pointingHand", NSCursor::pointingHandCursor()),
        ("crosshair", NSCursor::crosshairCursor()),
        ("openHand", NSCursor::openHandCursor()),
        ("closedHand", NSCursor::closedHandCursor()),
        ("operationNotAllowed", NSCursor::operationNotAllowedCursor()),
    ] {
        if std::ptr::eq(cursor, &*known) {
            return name.into();
        }
    }
    let image = cursor.image();
    if invisible(&image) {
        return "hidden".into();
    }
    let size = image.size();
    format!("other {:.0}x{:.0}", size.width, size.height)
}

fn install_hooks() -> Option<()> {
    ORIGINALS.with(|originals| {
        if originals.get().is_some() {
            return Some(());
        }
        let class = page_class()?;
        let update = class.instance_method(sel!(updateCursor:))?;
        let should_change = class.instance_method(sel!(shouldChangeCurrentCursor))?;
        // SAFETY: These replacement functions match the two Chromium methods' Objective-C ABI.
        unsafe {
            let a = update.set_implementation(std::mem::transmute::<
                unsafe extern "C-unwind" fn(&NSView, Sel, Option<&NSCursor>),
                Imp,
            >(update_cursor));
            let b = should_change.set_implementation(std::mem::transmute::<
                unsafe extern "C-unwind" fn(&NSView, Sel) -> Bool,
                Imp,
            >(should_change_cursor));
            originals.set(Some((a, b)));
        }
        Some(())
    })
}

unsafe extern "C-unwind" fn update_cursor(page: &NSView, sel: Sel, cursor: Option<&NSCursor>) {
    if let Some((original, _)) = ORIGINALS.with(std::cell::Cell::get) {
        // SAFETY: This is the previous implementation of this exact selector.
        let original = unsafe {
            std::mem::transmute::<Imp, unsafe extern "C-unwind" fn(&NSView, Sel, Option<&NSCursor>)>(
                original,
            )
        };
        unsafe { original(page, sel, cursor) };
    }
    if let Some(cursor) = cursor
        && std::ptr::eq(cursor, &*NSCursor::currentCursor())
    {
        ACTIVE.with(|active| {
            if let Some(active) = active.borrow().upgrade() {
                *active.last_page.borrow_mut() = Some(Weak::new(cursor));
            }
        });
    }
}

unsafe extern "C-unwind" fn should_change_cursor(page: &NSView, sel: Sel) -> Bool {
    if let Some((_, original)) = ORIGINALS.with(std::cell::Cell::get) {
        // SAFETY: This is the previous implementation of this exact selector.
        let original = unsafe {
            std::mem::transmute::<Imp, unsafe extern "C-unwind" fn(&NSView, Sel) -> Bool>(original)
        };
        if !unsafe { original(page, sel) }.as_bool() {
            return Bool::NO;
        }
    }
    let active = ACTIVE.with(|active| active.borrow().upgrade());
    let Some(active) = active else {
        return Bool::YES;
    };
    let Some(window) = page.window() else {
        return Bool::YES;
    };
    let Some(hit) = view_at(
        &window,
        window.convertPointFromScreen(NSEvent::mouseLocation()),
    ) else {
        return Bool::YES;
    };
    if hit.isDescendantOf(page) || !active.is_surface(&window, &hit) {
        return Bool::YES;
    }
    active.stats.borrow_mut().covered_page_cursors_held += 1;
    Bool::NO
}
