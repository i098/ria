// Ria original, no Ardia source.

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    mac::run();
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::ptr;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{MainThreadMarker, class, msg_send};
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSApplication, NSBackingStoreType, NSEvent,
        NSEventModifierFlags, NSEventType, NSScreen, NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{
        NSArray, NSDate, NSNotFound, NSPoint, NSRange, NSRect, NSRunLoop, NSSize, NSString,
        ns_string,
    };
    use ria_engine::call::ActionQueue;
    use ria_surface::WindowHost;
    use ria_surface::WindowSpec;
    use rune::{AppSpec, Color, Cx, El, Key, KeyEvent, Tx, col, el};

    #[derive(Default)]
    struct Log {
        presses: u32,
        moves: Vec<(f32, f32)>,
        wheels: Vec<(f32, f32)>,
        keys: usize,
    }

    type State = Rc<RefCell<Log>>;

    fn chrome(state: &State, cx: &mut Cx<State>) -> El {
        let width =
            200.0 + state.borrow().presses as f32 + if cx.system_dark() { 10.0 } else { 0.0 };
        col()
            .w_full()
            .h_full()
            .child(el().key("page").w(width).h(120.0))
    }

    fn overlay(_: &State, _: &mut Cx<State>) -> El {
        let button = el()
            .key("overlay")
            .w(60.0)
            .h(60.0)
            .fill(Color::WHITE)
            .role("button")
            .focusable(true)
            .label("Change page width")
            .on_press(|state: &mut State| state.borrow_mut().presses += 1)
            .on_pointer_move(|state: &mut State, event| {
                state.borrow_mut().moves.push((event.x, event.y))
            })
            .on_wheel(|state: &mut State, event, _| {
                state.borrow_mut().wheels.push((event.x, event.y));
                true
            });
        col().w(60.0).h(60.0).child(button)
    }

    fn key(state: &mut State, event: &KeyEvent, _: &mut Tx) -> bool {
        if event.key == Key::Right {
            state.borrow_mut().keys += 1;
        }
        true
    }

    pub fn run() {
        let mtm = MainThreadMarker::new().expect("test runs on the AppKit thread");
        let app = NSApplication::sharedApplication(mtm);
        app.finishLaunching();
        let size = NSSize::new(360.0, 240.0);
        let frame = NSRect::new(NSPoint::ZERO, size);
        // SAFETY: The test owns its AppKit window and never activates the app.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                frame,
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: Retained owns the NSWindow; close must not release it again.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setAppearance(
            NSAppearance::appearanceNamed(ns_string!("NSAppearanceNameAqua")).as_deref(),
        );
        let root = window.contentView().expect("window has a content view");
        let state = State::default();
        let mut overlay_spec = AppSpec::new("Overlay", (360.0, 240.0), State::default, overlay);
        overlay_spec.on_key = Some(key);
        let spec = WindowSpec {
            chrome: AppSpec::new("Chrome", (360.0, 240.0), State::default, chrome),
            overlay: overlay_spec,
            page_slot: "page",
            text_snapshot: |_| None,
        };
        let queue = ActionQueue::new();
        let host = WindowHost::new(&root, state.clone(), spec, queue.clone())
            .expect("Rune Metal surfaces");
        let page = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        host.set_page(Some(&page));
        host.set_overlay_visible(true);
        window.orderBack(None);
        window.display();
        assert!(!app.isActive(), "the native test must not activate the app");

        let surface = root.subviews().objectAtIndex(0);
        let views = surface.subviews();
        let chrome = views.objectAtIndex(1);
        let overlay = views.objectAtIndex(3);
        let overlay_for_query = overlay.clone();
        let core_rect = page.frame();
        let expected_none = NSRange::new(NSNotFound as usize, 0);
        let q = queue.clone();
        queue.push(Box::new(move || {
            // SAFETY: Both views implement these synchronous AppKit selectors.
            unsafe {
                let hit: *mut NSView = msg_send![&*overlay_for_query, hitTest: NSPoint::new(350.0, 235.0)];
                assert!(hit.is_null(), "a bare overlay point must not intercept the page");
                let marked: NSRange = msg_send![&*chrome, markedRange];
                let selected: NSRange = msg_send![&*chrome, selectedRange];
                let caret: NSRect = msg_send![&*chrome, firstRectForCharacterRange: expected_none, actualRange: ptr::null_mut::<NSRange>()];
                assert_eq!(marked, expected_none);
                assert_eq!(selected, expected_none);
                assert_eq!(caret, NSRect::ZERO);
                let children: Retained<NSArray<AnyObject>> = msg_send![&*surface, accessibilityChildren];
                assert_eq!(children.count(), 3);
                assert_eq!(Retained::as_ptr(&children.objectAtIndex(1)), Retained::as_ptr(&page).cast());
                let _: () = msg_send![&*chrome, setFrameSize: NSSize::new(280.0, 180.0)];
                assert_eq!(chrome.bounds().size, NSSize::new(280.0, 180.0));
                let _: () = msg_send![&*chrome, viewDidChangeBackingProperties];
            }
            assert_eq!(page.frame(), core_rect, "a size callback only records the viewport");
            assert_eq!(q.drain(), 0, "direct callbacks must not enqueue answers");
        }));
        queue.drain();
        assert_eq!(host.reentry_miss(), 0);

        let deadline = Instant::now() + Duration::from_secs(10);
        while (host.presentation_time().is_none() || page_size(&views) == NSSize::ZERO)
            && Instant::now() < deadline
        {
            NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
            queue.drain();
        }
        assert!(
            host.presentation_time().is_some(),
            "display link must post a presentation timestamp"
        );
        assert_eq!(page_size(&views), NSSize::new(200.0, 120.0));
        let at = overlay.convertPoint_toView(NSPoint::new(20.0, 20.0), None);
        let parent_point = overlay.convertPoint_toView(NSPoint::new(20.0, 20.0), Some(&root));
        // SAFETY: Query the laid-out native hierarchy before injecting input.
        let hit: *mut NSView = unsafe { msg_send![&*overlay, hitTest: parent_point] };
        assert_eq!(
            hit.cast_const(),
            Retained::as_ptr(&overlay),
            "overlay hit test at {at:?}"
        );
        for kind in [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp] {
            let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                kind, at, NSEventModifierFlags::empty(), 0.0, window.windowNumber(), None, 0, 1, 1.0,
            ).expect("mouse event");
            // SAFETY: Both selectors receive the NSEvent made for this window.
            unsafe {
                if kind == NSEventType::LeftMouseDown {
                    let _: () = msg_send![&*overlay, mouseDown: &*event];
                } else {
                    let _: () = msg_send![&*overlay, mouseUp: &*event];
                }
            }
        }
        assert_eq!(
            state.borrow().presses,
            0,
            "the AppKit input entries must defer work"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.borrow().presses == 0 && Instant::now() < deadline {
            NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
            queue.drain();
        }
        assert_eq!(
            state.borrow().presses,
            1,
            "the overlay must handle the click once"
        );
        host.mark_dirty(true, false);
        let deadline = Instant::now() + Duration::from_secs(10);
        while page_size(&views).width != 201.0 && Instant::now() < deadline {
            NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
            queue.drain();
        }
        assert_eq!(
            page_size(&views).width,
            201.0,
            "chrome must read the overlay's change"
        );
        check_input(&window, &overlay, &queue, &state);
        let dark = NSAppearance::appearanceNamed(ns_string!("NSAppearanceNameDarkAqua"))
            .expect("AppKit dark appearance");
        window.setAppearance(Some(&dark));
        let deadline = Instant::now() + Duration::from_secs(10);
        while page_size(&views).width != 211.0 && Instant::now() < deadline {
            NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
            queue.drain();
        }
        assert_eq!(
            page_size(&views).width,
            211.0,
            "Rune must use the new AppKit appearance"
        );
        check_accessibility(&overlay, &queue, &state);
        println!(
            "surface: native queries, input counts and coordinates, shared state, and appearance passed"
        );
        check_frames(&window, &views, &host, &queue);
        assert_eq!(host.reentry_miss(), 0);
        drop(host);
        window.close();
        println!(
            "surface: native queries, input counts and coordinates, shared state, appearance, and display frames passed"
        );
    }

    fn check_accessibility(overlay: &NSView, queue: &ActionQueue, state: &State) {
        let deadline = Instant::now() + Duration::from_secs(10);
        let button = loop {
            // SAFETY: AccessKit adds these selectors to the view and its elements.
            let button = unsafe {
                let children: Option<Retained<NSArray<AnyObject>>> =
                    msg_send![overlay, accessibilityChildren];
                children.and_then(|children| {
                    (0..children.count()).find_map(|i| {
                        let child = children.objectAtIndex(i);
                        let role: Option<Retained<NSString>> =
                            msg_send![&*child, accessibilityRole];
                        (role.as_deref() == Some(ns_string!("AXButton"))).then_some(child)
                    })
                })
            };
            if let Some(button) = button {
                break button;
            }
            assert!(
                Instant::now() < deadline,
                "AccessKit did not expose the Rune button"
            );
            NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
            queue.drain();
        };
        // SAFETY: The queried object is an accessible button from the adapter.
        unsafe {
            let label: Option<Retained<NSString>> = msg_send![&*button, accessibilityLabel];
            assert_eq!(label.as_deref(), Some(ns_string!("Change page width")));
            let accepted: bool = msg_send![&*button, accessibilityPerformPress];
            assert!(accepted);
        }
        assert_eq!(
            state.borrow().presses,
            1,
            "accessibility actions must defer work"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.borrow().presses == 1 && Instant::now() < deadline {
            NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
            queue.drain();
        }
        assert_eq!(state.borrow().presses, 2);
    }

    fn check_frames(
        window: &NSWindow,
        views: &NSArray<NSView>,
        host: &WindowHost<Log>,
        queue: &ActionQueue,
    ) {
        let chrome = views.objectAtIndex(1);
        let overlay = views.objectAtIndex(3);
        let mut previous = host.presentation_time().unwrap();
        let mut late = 0;
        let deadline = Instant::now() + Duration::from_secs(25);
        let mut sample = 0;
        while sample < 600 && Instant::now() < deadline {
            if sample < 300 {
                let width = 300.0 + f64::from(sample % 60);
                window.setContentSize(NSSize::new(width, 240.0));
            }
            host.mark_dirty(true, true);
            loop {
                NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.001));
                queue.drain();
                let time = host.presentation_time().unwrap();
                if time != previous {
                    late += usize::from((time - previous) * 1000.0 > 20.7);
                    previous = time;
                    break;
                }
                assert!(Instant::now() < deadline, "the display link stopped");
            }
            for view in [&chrome, &overlay] {
                let layer = view.layer().expect("Rune backing layer");
                assert_eq!(layer.frame(), view.bounds());
                assert_eq!(layer.contentsScale(), window.backingScaleFactor());
            }
            sample += 1;
        }
        assert_eq!(sample, 600, "display frame samples");
        println!("display frames: {sample}, intervals over 20.7 ms: {late}");
        assert!(late <= 6, "too many late display frames: {late}");
    }

    fn check_input(window: &NSWindow, view: &NSView, queue: &ActionQueue, state: &State) {
        state.borrow_mut().moves.clear();
        let chars = ns_string!("\u{f703}");
        let mtm = MainThreadMarker::new().unwrap();
        let screen_height = NSScreen::screens(mtm)
            .firstObject()
            .expect("screen")
            .frame()
            .size
            .height;
        let deadline = Instant::now() + Duration::from_secs(10);
        for i in 0..100 {
            let x = 10.0 + f64::from(i % 40);
            let at = view.convertPoint_toView(NSPoint::new(x, 20.0), None);
            let mouse = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                NSEventType::MouseMoved, at, NSEventModifierFlags::empty(), 0.0, window.windowNumber(), None, 0, 0, 0.0,
            ).expect("mouse event");
            // SAFETY: Send native events to the AppKit input view for this window.
            unsafe {
                let _: () = msg_send![view, mouseMoved: &*mouse];
                if i < 20 {
                    let screen = window.convertPointToScreen(at);
                    let cg = CGEventCreateScrollWheelEvent2(ptr::null(), 1, 2, 2, 0, 0);
                    assert!(!cg.is_null());
                    CGEventSetLocation(cg, NSPoint::new(screen.x, screen_height - screen.y));
                    let scroll: Option<Retained<NSEvent>> =
                        msg_send![class!(NSEvent), eventWithCGEvent: cg];
                    CFRelease(cg);
                    let _: () = msg_send![view, scrollWheel: &*scroll.expect("wheel event")];
                }
                if i < 50 {
                    let key = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                        NSEventType::KeyDown, NSPoint::ZERO, NSEventModifierFlags::empty(), 0.0,
                        window.windowNumber(), None, chars, chars, false, 124,
                    ).expect("key event");
                    let _: () = msg_send![view, keyDown: &*key];
                }
            }
            while state.borrow().moves.len() <= i as usize && Instant::now() < deadline {
                NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
                queue.drain();
            }
        }
        let log = state.borrow();
        assert_eq!(log.moves.len(), 100);
        assert_eq!(log.wheels.len(), 20);
        assert_eq!(log.keys, 50);
        for (i, &(x, y)) in log.moves.iter().enumerate() {
            assert!((x - (10 + i % 40) as f32).abs() <= 1.0 && (y - 20.0).abs() <= 1.0);
        }
        for (i, &(x, y)) in log.wheels.iter().enumerate() {
            assert!((x - (10 + i) as f32).abs() <= 1.0 && (y - 20.0).abs() <= 1.0);
        }
    }

    unsafe extern "C" {
        fn CGEventCreateScrollWheelEvent2(
            source: *const c_void,
            units: u32,
            wheels: u32,
            x: i32,
            y: i32,
            z: i32,
        ) -> *mut c_void;
        fn CGEventSetLocation(event: *mut c_void, location: NSPoint);
        fn CFRelease(value: *const c_void);
    }

    fn page_size(views: &NSArray<NSView>) -> NSSize {
        views.objectAtIndex(2).frame().size
    }
}
