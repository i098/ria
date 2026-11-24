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
    use objc2::{MainThreadMarker, MainThreadOnly, class, define_class, msg_send};
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSApplication, NSApplicationActivationPolicy,
        NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType, NSScreen, NSView, NSWindow,
        NSWindowOcclusionState, NSWindowStyleMask,
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
        col().w_full().h_full().child(button)
    }

    fn key(state: &mut State, event: &KeyEvent, _: &mut Tx) -> bool {
        if event.key == Key::Right {
            state.borrow_mut().keys += 1;
        }
        true
    }

    define_class!(
        /// A test window that answers visible without activation.
        ///
        /// The Window Server grants `NSWindowOcclusionStateVisible` only to an app the user
        /// activated, and this test must not activate. It still composites the window and serves
        /// drawables, so the window answers visible itself, as Rune's `PresumeShown` does for its
        /// own windows.
        #[unsafe(super(NSWindow))]
        #[thread_kind = MainThreadOnly]
        #[name = "RiaSurfaceTestWindow"]
        struct TestWindow;

        impl TestWindow {
            #[unsafe(method(occlusionState))]
            fn occlusion_state(&self) -> NSWindowOcclusionState {
                NSWindowOcclusionState::Visible
            }
        }
    );

    pub fn run() {
        let mtm = MainThreadMarker::new().expect("test runs on the AppKit thread");
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.finishLaunching();
        let size = NSSize::new(360.0, 240.0);
        // Top left of the screen, clear of the Dock, so the Window Server can report it visible.
        let screen = NSScreen::mainScreen(mtm).expect("the test needs a screen");
        let frame = NSRect::new(
            NSPoint::new(0.0, screen.frame().size.height - size.height),
            size,
        );
        // SAFETY: The test owns its AppKit window and never activates the app. `TestWindow` has
        // no ivars, and the `NSWindow` initializer fills the superclass.
        let window: Retained<TestWindow> = unsafe {
            msg_send![
                mtm.alloc::<TestWindow>(),
                initWithContentRect: frame,
                styleMask: NSWindowStyleMask::Titled,
                backing: NSBackingStoreType::Buffered,
                defer: false
            ]
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
        window.orderFrontRegardless();
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

        let parent_point = views
            .objectAtIndex(3)
            .convertPoint_toView(NSPoint::new(20.0, 20.0), Some(&root));
        let overlay_hit = |views: &NSArray<NSView>| -> *mut NSView {
            // SAFETY: The overlay answers hitTest: for a point in the surface container's space.
            unsafe { msg_send![&*views.objectAtIndex(3), hitTest: parent_point] }
        };
        // The test needs a real desktop: Rune lays out only on a presented frame, and wgpu asks
        // the Window Server for a drawable (the window answers visible itself, see `TestWindow`).
        assert!(
            window.isVisible(),
            "the window must be ordered in: run this test in the console session, not over ssh"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while (host.presentation_time().is_none()
            || page_size(&views) == NSSize::ZERO
            || overlay_hit(&views).is_null())
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
        // SAFETY: Query the laid-out native hierarchy before injecting input.
        let hit: *mut NSView = unsafe { msg_send![&*overlay, hitTest: parent_point] };
        assert_eq!(
            hit.cast_const(),
            Retained::as_ptr(&overlay),
            "the overlay must answer the hit test at {at:?}"
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
            // SAFETY: AccessKit's subclass answers these selectors on the view and its nodes.
            match unsafe { find_role(overlay, ns_string!("AXButton")) } {
                Ok(button) => break button,
                Err(roles) => assert!(
                    Instant::now() < deadline,
                    "AccessKit did not expose the Rune button; roles: {roles:?}"
                ),
            }
            NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
            queue.drain();
        };
        // SAFETY: The queried object is an accessible button from the adapter.
        unsafe {
            let title: Option<Retained<NSString>> = msg_send![&*button, accessibilityTitle];
            assert_eq!(title.as_deref(), Some(ns_string!("Change page width")));
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

    /// Finds the first accessibility descendant of `view` whose role is `role`, or lists the roles
    /// it saw. AppKit answers `accessibilityChildren` on the view and on every node AccessKit
    /// exposes, so this walks the tree the way an assistive client does.
    ///
    /// # Safety
    /// `view` is an AccessKit adaptor view and the nodes are its own accessible objects.
    unsafe fn find_role(
        view: &NSView,
        role: &NSString,
    ) -> Result<Retained<AnyObject>, Vec<String>> {
        let mut roles = Vec::new();
        // SAFETY: The view is an accessible object; the same pointer reads as `AnyObject`.
        let root: &AnyObject = unsafe { &*(view as *const NSView).cast::<AnyObject>() };
        let mut pending = unsafe { ax_children(root) };
        while let Some(node) = pending.pop() {
            // SAFETY: An accessible node answers both properties.
            let node_role: Option<Retained<NSString>> =
                unsafe { msg_send![&*node, accessibilityRole] };
            if node_role.as_deref() == Some(role) {
                return Ok(node);
            }
            roles.push(
                node_role
                    .as_deref()
                    .map_or_else(String::new, |role| role.to_string()),
            );
            assert!(roles.len() <= 300, "the accessibility tree is too large");
            pending.extend(unsafe { ax_children(&node) });
        }
        Err(roles)
    }

    /// # Safety
    /// As `find_role`.
    unsafe fn ax_children(node: &AnyObject) -> Vec<Retained<AnyObject>> {
        // SAFETY: An accessible object answers `accessibilityChildren`.
        let children: Option<Retained<NSArray<AnyObject>>> =
            unsafe { msg_send![node, accessibilityChildren] };
        children.map_or_else(Vec::new, |children| {
            (0..children.count())
                .map(|i| children.objectAtIndex(i))
                .collect()
        })
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
                    CFRelease(cg as *const c_void);
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

    /// Opaque `CGEventRef`; the encoding must match what AppKit expects, not `*mut c_void`.
    #[repr(C)]
    struct CGEvent {
        _private: [u8; 0],
    }

    // SAFETY: The encoding matches the C header's `__CGEvent`, so AppKit accepts the pointer.
    unsafe impl objc2::RefEncode for CGEvent {
        const ENCODING_REF: objc2::Encoding =
            objc2::Encoding::Pointer(&objc2::Encoding::Struct("__CGEvent", &[]));
    }

    unsafe extern "C" {
        fn CGEventCreateScrollWheelEvent2(
            source: *const c_void,
            units: u32,
            wheels: u32,
            x: i32,
            y: i32,
            z: i32,
        ) -> *mut CGEvent;
        fn CGEventSetLocation(event: *mut CGEvent, location: NSPoint);
        fn CFRelease(value: *const c_void);
    }

    fn page_size(views: &NSArray<NSView>) -> NSSize {
        views.objectAtIndex(2).frame().size
    }
}
