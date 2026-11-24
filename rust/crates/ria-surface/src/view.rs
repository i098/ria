// Ria original, no Ardia source.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::rc::{Rc, Weak};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSTextInputClient, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSPoint, NSRange, NSRect, NSSize, NSString,
};
use objc2_quartz_core::CAMetalLayer;
use ria_engine::call::ActionQueue;
use rune::{Clause, Ime, Key, KeyEvent, Mods};

use crate::input::{
    Input, NO_RANGE, TextInputSnapshot, byte_range, command, modifiers, native_key,
};

pub(crate) fn boundary<T>(operation: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(panic) => {
            use std::io::Write;
            let message = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic");
            let path =
                std::env::temp_dir().join(format!("ria-surface-panic-{}.txt", std::process::id()));
            if let Ok(mut report) = std::fs::File::create_new(path) {
                let _ = writeln!(
                    report,
                    "{message}\n{}",
                    std::backtrace::Backtrace::force_capture()
                );
            }
            std::process::abort();
        }
    }
}

pub(crate) trait ViewHost {
    fn hit(&self, x: f32, y: f32) -> bool;
    fn caret(&self) -> Option<[f32; 4]>;
    fn snapshot(&self) -> Option<Rc<TextInputSnapshot>>;
    fn handle(&self, input: Input);
}

#[derive(Clone, Copy)]
pub(crate) struct Viewport {
    pub bounds: NSRect,
    pub scale: f64,
}

struct MarkedText {
    text: String,
    range: NSRange,
    selected: NSRange,
}

pub(crate) struct Ivars {
    host: Weak<dyn ViewHost>,
    queue: Rc<ActionQueue>,
    overlay: bool,
    viewport: Cell<Option<Viewport>>,
    marked: RefCell<Option<MarkedText>>,
    key: Cell<(Mods, bool)>,
    tracking: RefCell<Option<Retained<NSTrackingArea>>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaSurfaceView"]
    #[ivars = Ivars]
    pub(crate) struct RuneView;

    impl RuneView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }

        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool { false }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _: Option<&NSEvent>) -> bool { true }

        #[unsafe(method(makeBackingLayer))]
        fn make_backing_layer(&self) -> *mut AnyObject {
            boundary(|| {
                // SAFETY: AppKit retains the returned layer.
                let layer: Retained<CAMetalLayer> = unsafe { msg_send![objc2::class!(CAMetalLayer), new] };
                Retained::autorelease_return(layer).cast()
            })
        }

        #[unsafe(method(wantsUpdateLayer))]
        fn wants_update_layer(&self) -> bool { true }

        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {}

        #[unsafe(method(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> *mut NSView {
            boundary(|| {
                if self.ivars().overlay {
                    // SAFETY: This view and its AppKit hierarchy stay on the main thread.
                    let parent = unsafe { self.superview() };
                    let local = self.convertPoint_fromView(point, parent.as_deref());
                    if !self.host().is_some_and(|host| host.hit(local.x as f32, local.y as f32)) {
                        return ptr::null_mut();
                    }
                }
                // SAFETY: NSView's hitTest: expects a point in the superview's coordinates.
                unsafe { msg_send![super(self), hitTest: point] }
            })
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            boundary(|| {
                // SAFETY: Preserve NSView's synchronous size change before we capture its bounds.
                let _: () = unsafe { msg_send![super(self), setFrameSize: size] };
                self.capture_viewport();
            });
        }

        #[unsafe(method(viewDidChangeBackingProperties))]
        fn view_did_change_backing_properties(&self) {
            boundary(|| {
                // SAFETY: Preserve NSView's backing-property notification.
                let _: () = unsafe { msg_send![super(self), viewDidChangeBackingProperties] };
                self.capture_viewport();
            });
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            boundary(|| {
                // SAFETY: This is an NSView, and the view owns the tracking area.
                let _: () = unsafe { msg_send![super(self), updateTrackingAreas] };
                let old = self.ivars().tracking.take();
                if let Some(old) = old { self.removeTrackingArea(&old); }
                let options = NSTrackingAreaOptions::MouseMoved
                    | NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::ActiveInKeyWindow
                    | NSTrackingAreaOptions::InVisibleRect;
                let area = unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(
                    self.mtm().alloc(), NSRect::ZERO, options, Some(self), None,
                ) };
                self.addTrackingArea(&area);
                self.ivars().tracking.replace(Some(area));
            });
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            boundary(|| {
                let (x, y) = self.event_point(event);
                if !self.ivars().overlay || self.host().is_some_and(|host| host.hit(x, y)) {
                    self.enqueue(Input::Move(x, y));
                } else {
                    self.enqueue(Input::Leave);
                }
            });
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            boundary(|| {
                let (x, y) = self.event_point(event);
                self.enqueue(Input::Move(x, y));
            });
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _: &NSEvent) { boundary(|| self.enqueue(Input::Leave)); }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            boundary(|| {
                if let Some(window) = self.window() { window.makeFirstResponder(Some(self)); }
                self.button(event, true, false);
            });
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) { boundary(|| self.button(event, false, false)); }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) { boundary(|| self.button(event, true, true)); }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &NSEvent) { boundary(|| self.button(event, false, true)); }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            boundary(|| {
                let (x, y) = self.event_point(event);
                self.enqueue(Input::Wheel { x, y, dx: event.scrollingDeltaX() as f32, dy: event.scrollingDeltaY() as f32 });
            });
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            boundary(|| {
                let mods = modifiers(event);
                let repeat = event.isARepeat();
                self.ivars().key.set((mods, repeat));
                if mods.cmd || mods.ctrl || !self.host().is_some_and(|host| host.caret().is_some()) {
                    if let Some(key) = native_key(event, mods) { self.enqueue(Input::Key(KeyEvent { key, mods }, repeat)); }
                } else {
                    self.interpretKeyEvents(&NSArray::from_slice(&[event]));
                }
            });
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) {
            boundary(|| {
                let mods = modifiers(event);
                if let Some(key) = native_key(event, mods) { self.enqueue(Input::KeyUp(KeyEvent { key, mods })); }
            });
        }

        #[unsafe(method(doCommandBySelector:))]
        fn do_command_by_selector(&self, selector: Sel) {
            boundary(|| {
                let (mods, repeat) = self.ivars().key.get();
                if let Some(event) = command(selector, mods) { self.enqueue(Input::Key(event, repeat)); }
            });
        }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            boundary(|| { self.enqueue(Input::Focus(true)); true })
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            boundary(|| { self.enqueue(Input::Focus(false)); true })
        }
    }

    unsafe impl NSObjectProtocol for RuneView {}

    unsafe impl NSTextInputClient for RuneView {
        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool { boundary(|| self.ivars().marked.borrow().is_some()) }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            boundary(|| self.ivars().marked.borrow().as_ref().map_or(NO_RANGE, |marked| marked.range))
        }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            boundary(|| self.selection_range())
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn set_marked_text(&self, string: &AnyObject, selected: NSRange, replacement: NSRange) {
            boundary(|| {
                let (text, mut clauses) = marked_text(string);
                let snapshot = self.host().and_then(|host| host.snapshot());
                let replace = snapshot.as_ref().and_then(|s| s.byte_range(replacement));
                let previous = self.ivars().marked.take();
                let cursor = byte_range(&text, selected).map(|range| {
                    if !range.is_empty() { clauses.push(Clause::new(range.clone()).target()); }
                    (range.start, range.end)
                });
                if !text.is_empty() {
                    let location = if replace.is_some() {
                        replacement.location
                    } else if let Some(previous) = previous {
                        previous.range.location
                    } else {
                        snapshot.map_or(0, |s| s.selected_range().location)
                    };
                    let length = text.encode_utf16().count();
                    let selection = if cursor.is_some() {
                        NSRange::new(location + selected.location, selected.length)
                    } else {
                        NSRange::new(location + length, 0)
                    };
                    self.ivars().marked.replace(Some(MarkedText {
                        text: text.clone(), range: NSRange::new(location, length), selected: selection,
                    }));
                }
                self.enqueue(Input::Ime { change: Ime::Preedit(text, cursor, clauses), replacement: replace });
            });
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            boundary(|| {
                if let Some(marked) = self.ivars().marked.take() {
                    self.enqueue(Input::Ime { change: Ime::Commit(marked.text), replacement: None });
                }
            });
        }

        #[unsafe(method(validAttributesForMarkedText))]
        fn valid_attributes_for_marked_text(&self) -> *mut NSArray<NSAttributedStringKey> {
            boundary(|| {
                // SAFETY: This is AppKit's static attributed-string key.
                let key = unsafe { objc2_app_kit::NSMarkedClauseSegmentAttributeName };
                Retained::autorelease_return(NSArray::from_slice(&[key]))
            })
        }

        #[unsafe(method(attributedSubstringForProposedRange:actualRange:))]
        fn attributed_substring(&self, range: NSRange, actual: *mut NSRange) -> *mut NSAttributedString {
            boundary(|| {
                let Some(snapshot) = self.host().and_then(|host| host.snapshot()) else { return ptr::null_mut() };
                let Some(text) = snapshot.substring(range) else { return ptr::null_mut() };
                let text = NSString::from_str(text);
                let output = NSAttributedString::initWithString(self.mtm().alloc(), &text);
                write_range(actual, range);
                Retained::autorelease_return(output)
            })
        }

        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, string: &AnyObject, range: NSRange) {
            boundary(|| {
                let text = plain_string(string);
                let replacement = self.host().and_then(|host| host.snapshot()).and_then(|s| s.byte_range(range));
                if self.ivars().marked.take().is_some() || replacement.is_some() {
                    self.enqueue(Input::Ime { change: Ime::Commit(text), replacement });
                } else {
                    let (mods, repeat) = self.ivars().key.get();
                    self.enqueue(Input::Key(KeyEvent { key: Key::Text(text), mods }, repeat));
                }
            });
        }

        #[unsafe(method(characterIndexForPoint:))]
        fn character_index_for_point(&self, point: NSPoint) -> usize {
            boundary(|| {
                let caret = self.screen_caret();
                if point.x >= caret.origin.x && point.x <= caret.origin.x + caret.size.width
                    && point.y >= caret.origin.y && point.y <= caret.origin.y + caret.size.height
                {
                    self.selection_range().location
                } else {
                    NO_RANGE.location
                }
            })
        }

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect(&self, _: NSRange, actual: *mut NSRange) -> NSRect {
            boundary(|| {
                write_range(actual, self.selection_range());
                self.screen_caret()
            })
        }
    }
);

impl RuneView {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        overlay: bool,
        host: Weak<dyn ViewHost>,
        queue: Rc<ActionQueue>,
    ) -> Retained<Self> {
        let view = mtm.alloc::<Self>().set_ivars(Ivars {
            host,
            queue,
            overlay,
            viewport: Cell::new(None),
            marked: RefCell::new(None),
            key: Cell::new((Mods::default(), false)),
            tracking: RefCell::new(None),
        });
        // SAFETY: All ivars are initialized, and frame is in superview points.
        let view: Retained<Self> = unsafe { msg_send![super(view), initWithFrame: frame] };
        view.setWantsLayer(true);
        view
    }

    pub(crate) fn metal_layer(&self) -> Retained<CAMetalLayer> {
        // SAFETY: makeBackingLayer supplies a retained CAMetalLayer whenever the view has a layer.
        unsafe { msg_send![self, layer] }
    }

    pub(crate) fn capture_viewport(&self) {
        self.ivars().viewport.set(Some(Viewport {
            bounds: self.bounds(),
            scale: self
                .window()
                .map_or(1.0, |window| window.backingScaleFactor()),
        }));
    }

    pub(crate) fn take_viewport(&self) -> Option<Viewport> {
        self.ivars().viewport.take()
    }

    fn host(&self) -> Option<Rc<dyn ViewHost>> {
        self.ivars().host.upgrade()
    }

    fn selection_range(&self) -> NSRange {
        if let Some(marked) = self.ivars().marked.borrow().as_ref() {
            return marked.selected;
        }
        self.host()
            .and_then(|host| host.snapshot())
            .map_or(NO_RANGE, |snapshot| snapshot.selected_range())
    }

    fn enqueue(&self, input: Input) {
        let host = self.ivars().host.clone();
        self.ivars().queue.push(Box::new(move || {
            if let Some(host) = host.upgrade() {
                host.handle(input);
            }
        }));
    }

    fn button(&self, event: &NSEvent, down: bool, secondary: bool) {
        let (x, y) = self.event_point(event);
        self.enqueue(Input::Button {
            x,
            y,
            down,
            secondary,
        });
    }

    fn event_point(&self, event: &NSEvent) -> (f32, f32) {
        let point = event.locationInWindow();
        let point = if event.window(self.mtm()).is_some() {
            point
        } else {
            self.window()
                .map_or(point, |w| w.convertPointFromScreen(point))
        };
        let point = self.convertPoint_fromView(point, None);
        (point.x as f32, point.y as f32)
    }

    fn screen_caret(&self) -> NSRect {
        let Some([x, y, width, height]) = self.host().and_then(|host| host.caret()) else {
            return NSRect::ZERO;
        };
        let rect = NSRect::new(
            NSPoint::new(x.into(), y.into()),
            NSSize::new(width.into(), height.into()),
        );
        let rect = self.convertRect_toView(rect, None);
        self.window()
            .map_or(rect, |window| window.convertRectToScreen(rect))
    }
}

fn write_range(output: *mut NSRange, range: NSRange) {
    if !output.is_null() {
        // SAFETY: NSTextInputClient gives either a null pointer or writable storage for one NSRange.
        unsafe { output.write(range) };
    }
}

fn plain_string(object: &AnyObject) -> String {
    // SAFETY: NSTextInputClient gives an NSString or NSAttributedString for text input.
    let is_attributed: bool =
        unsafe { msg_send![object, isKindOfClass: objc2::class!(NSAttributedString)] };
    let string: Retained<NSString> = if is_attributed {
        unsafe { msg_send![object, string] }
    } else {
        unsafe { msg_send![object, self] }
    };
    string.to_string()
}

fn marked_text(object: &AnyObject) -> (String, Vec<Clause>) {
    let text = plain_string(object);
    // SAFETY: The class check precedes the attributed string access.
    let attributed: bool =
        unsafe { msg_send![object, isKindOfClass: objc2::class!(NSAttributedString)] };
    let mut clauses = Vec::new();
    if attributed {
        let units = text.encode_utf16().count();
        let mut index = 0;
        while index < units {
            let mut range = NO_RANGE;
            // SAFETY: `index` is within the string and `range` is valid out storage.
            let value: *mut c_void = unsafe {
                msg_send![object, attribute: objc2_app_kit::NSMarkedClauseSegmentAttributeName, atIndex: index, effectiveRange: &mut range as *mut NSRange]
            };
            let Some(end) = range
                .location
                .checked_add(range.length)
                .filter(|end| *end > index)
            else {
                break;
            };
            if !value.is_null()
                && let Some(bytes) = byte_range(&text, range)
            {
                clauses.push(Clause::new(bytes));
            }
            index = end;
        }
    }
    (text, clauses)
}
