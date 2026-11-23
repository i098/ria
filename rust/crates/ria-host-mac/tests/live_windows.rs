// Ria original, no Ardia source.

//! Run from the repository root on a Mac:
//! `(cd rust && ENGINE_DIR=/path/to/framework-parent blaze test -p ria-host-mac --test live_windows)`.
//!
//! Window-host Check L uses a temporary Chromium profile and an isolated AppKit defaults domain.
//! It does not activate the app. The check skips on Linux.
//! Physical display moves and foreground input are not part of this check.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("live_windows: needs macOS and the prebuilt engine");
}

#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("RIA_WINDOWS_DATA").is_some() {
        mac::child();
    } else {
        mac::parent();
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::{Cell, RefCell};
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::process::Command;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use block2::{DynBlock, RcBlock};
    use objc2::runtime::{AnyObject, Bool, Sel};
    use objc2::{
        ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, ffi,
        msg_send, rc::Retained, sel,
    };
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSColor, NSEvent, NSEventModifierFlags, NSEventType,
        NSMenu, NSMenuItem, NSScreen, NSTextField, NSTextView, NSUserInterfaceItemIdentification,
        NSView, NSWindow, NSWindowButton, NSWindowDidEnterFullScreenNotification,
        NSWindowDidExitFullScreenNotification, NSWindowStyleMask,
        NSWindowWillEnterFullScreenNotification, NSWindowWillExitFullScreenNotification,
    };
    use objc2_foundation::{
        NSBundle, NSDate, NSIntersectionRect, NSNotification, NSNotificationCenter, NSPoint,
        NSRect, NSRunLoop, NSSize, NSString, NSUserDefaults, ns_string,
    };
    use ria_engine::call::main_queue;
    use ria_host_mac::core::bindings::{CGImageRef, Engine, Profile, Tab, Window};
    use ria_host_mac::core::delegate::{EngineDelegate, Hooks};
    use ria_host_mac::core::host::State;
    use ria_host_mac::launch::host_launch::{self, LaunchEvent};
    use ria_host_mac::tabs::ardia_page_cover::PageCover;
    use ria_host_mac::windows::ardia_chrome_window_host::ChromeWindowHost;
    use ria_host_mac::windows::ardia_chrome_window_host_little_fit::little_frame_for_size;
    use ria_host_mac::windows::ardia_display_identity::frame_on_screen;
    use ria_host_mac::windows::ardia_interface_scale;
    use ria_host_mac::windows::ardia_popup_origin_strip::PopupOriginStrip;
    use ria_host_mac::windows::ardia_popup_window::Popups;
    use ria_host_mac::windows::ardia_reserved_keys::{
        KeyContext, is_reserved_binding, is_reserved_key_event,
    };
    use ria_host_mac::windows::ardia_traffic_lights::Placement;
    use ria_host_mac::windows::ardia_window_controller::{Controllers, WindowKind};
    use serde_json::Value;

    const FRAMEWORK: &str = "Chromium Framework.framework";

    pub fn parent() {
        let framework = PathBuf::from(
            std::env::var_os("ENGINE_DIR").expect("set ENGINE_DIR to the framework's parent"),
        )
        .join(FRAMEWORK);
        let root = std::env::temp_dir().join(format!("ria-windows-live-{}", std::process::id()));
        let bundle_id = format!("sh.ria.windows-live.{}", std::process::id());
        let contents = root.join("Live.app/Contents");
        let exe = contents.join("MacOS/live_windows");
        let data = root.join("data");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::create_dir(contents.join("Frameworks")).unwrap();
        std::fs::create_dir(&data).unwrap();
        std::fs::write(
            contents.join("Info.plist"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>live_windows</string>
<key>CFBundleIdentifier</key><string>{bundle_id}</string>
<key>CFBundleName</key><string>Ria Windows Live</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>0</string>
<key>NSPrincipalClass</key><string>BrowserCrApplication</string>
</dict></plist>"#
            ),
        )
        .unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        assert!(
            Command::new("cp")
                .arg("-cR")
                .arg(framework)
                .arg(contents.join("Frameworks"))
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("codesign")
                .args(["--force", "--sign", "-"])
                .arg(&exe)
                .status()
                .unwrap()
                .success()
        );
        let status = Command::new(&exe)
            .env("RIA_WINDOWS_DATA", "1")
            .env("RIA_DATA_DIR", &data)
            .env("RIA_BACKGROUND", "1")
            .env("RIA_TEST", "fakeFullScreenMs=500")
            .status();
        NSUserDefaults::standardUserDefaults()
            .removePersistentDomainForName(&NSString::from_str(&bundle_id));
        std::fs::remove_dir_all(root).unwrap();
        assert!(status.unwrap().success());
    }

    pub fn child() {
        let shutdown = Rc::new(Cell::new(false));
        let identifier = NSBundle::mainBundle().bundleIdentifier().unwrap();
        assert!(identifier.to_string().starts_with("sh.ria.windows-live."));
        NSUserDefaults::standardUserDefaults().setDouble_forKey(1., ns_string!("interfaceScale"));
        assert_eq!(ardia_interface_scale::scale(), 1.);
        let exited = shutdown.clone();
        let exit = host_launch::run(Rc::new(move |event| match event {
            LaunchEvent::WindowOpened { .. } => main_queue::post(Box::new(|| {
                check_windows(&Engine::shared().unwrap().default_profile());
            })),
            LaunchEvent::EngineWillShutDown => exited.set(true),
            LaunchEvent::Error(error) => panic!("{error}"),
            _ => {}
        }))
        .unwrap();
        assert_eq!(exit, 0);
        assert!(shutdown.get());
        println!("live_windows: engine shut down cleanly");
    }

    fn wait_for(message: &str, mut ready: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready() {
            assert!(Instant::now() < deadline, "{message}");
            NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.01));
        }
    }

    fn observe(
        window: &NSWindow,
        name: &NSString,
        callback: impl Fn() + 'static,
    ) -> Retained<AnyObject> {
        let block = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| callback());
        let token = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(name),
                Some(window),
                None,
                &block,
            )
        };
        token.into()
    }

    fn check_reserved_replies(app: &NSApplication, mtm: MainThreadMarker) {
        assert!(!app.isActive());
        let delegate = EngineDelegate::new(Rc::new(WindowHooks), State::new(), None);
        let menu = NSMenu::new(mtm);
        let modifiers = NSEventModifierFlags::Command;
        let bindings = [
            (
                ns_string!("w"),
                ns_string!("ardia.command.closeTab"),
                13,
                true,
            ),
            (
                ns_string!("b"),
                ns_string!("ria.test.nonreserved"),
                11,
                false,
            ),
        ];
        for (key, id, _, _) in bindings {
            let item = NSMenuItem::new(mtm);
            item.setTitle(id);
            item.setIdentifier(Some(id));
            item.setKeyEquivalent(key);
            item.setKeyEquivalentModifierMask(modifiers);
            menu.addItem(&item);
        }
        let previous_menu = app.mainMenu();
        app.setMainMenu(Some(&menu));
        let replies = bindings.map(|(key, _, key_code, expected)| {
            let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                NSEventType::KeyDown, NSPoint::ZERO, modifiers, 0., 0, None, key, key, false,
                key_code,
            )
            .expect("Test key event");
            let binding: Bool = unsafe {
                msg_send![delegate.as_object(), engineIsReservedKey: key, unshiftedKey: key, modifiers: modifiers.bits()]
            };
            let key_event: Bool =
                unsafe { msg_send![delegate.as_object(), engineIsReservedKeyEvent: &*event] };
            ((binding.as_bool(), key_event.as_bool()), (expected, expected))
        });
        app.setMainMenu(previous_menu.as_deref());
        assert!(!app.isActive());
        for (actual, expected) in replies {
            assert_eq!(actual, expected);
        }
    }

    fn check_native_close(mtm: MainThreadMarker) {
        let allow = Rc::new(Cell::new(false));
        let requests = Rc::new(Cell::new(0));
        let controllers = Controllers::new(mtm, true, true, {
            let allow = allow.clone();
            let requests = requests.clone();
            move |_| {
                requests.set(requests.get() + 1);
                allow.get()
            }
        });
        let controller = controllers.make_normal();
        let window = controller.core.window();
        window.orderFrontRegardless();
        window.performClose(None);
        wait_for(
            "The engine did not ask whether the window could close",
            || requests.get() > 0,
        );
        assert_eq!(requests.get(), 1);
        assert!(window.isVisible());
        let registered = controllers
            .for_window(&window)
            .expect("The vetoed window must stay registered");
        assert!(Rc::ptr_eq(&registered, &controller));
        assert_eq!(controllers.all().len(), 1);

        allow.set(true);
        controller.fullscreen.act_window_full_screen(true);
        assert!(controller.core.acted_full_screen());
        assert!(controller.fullscreen.is_full_screen());
        window.performClose(None);
        wait_for("The accepted close did not remove the window", || {
            controllers.for_window(&window).is_none()
        });
        assert_eq!(requests.get(), 2);
        assert!(controllers.all().is_empty());
        assert!(!controller.fullscreen.is_full_screen());
    }

    fn check_dev_actions(
        host: &ChromeWindowHost,
        window: &NSWindow,
        second: &NSWindow,
        app: &NSApplication,
    ) {
        let run = |action| host.dev_action(window, action, true).unwrap();
        for action in [
            "click",
            "drag",
            "mouse",
            "fakeActiveClicks",
            "postNotification",
            "subroles",
            "ax",
            "type",
        ] {
            assert_eq!(
                host.dev_action(window, action, false).as_deref(),
                Some("test instances only")
            );
        }
        check_native_mouse_delivery(host, app);
        window.setTitle(ns_string!("Native probe fixture"));
        let text = NSTextView::initWithFrame(
            window.mtm().alloc(),
            NSRect::new(NSPoint::ZERO, NSSize::new(180., 60.)),
        );
        window.contentView().unwrap().addSubview(&text);
        assert!(window.makeFirstResponder(Some(&text)));
        assert!(run("type:Native probe").starts_with("NSTextView"));
        assert_eq!(text.string().to_string(), "Native probe");
        window.orderFrontRegardless();
        let tree = run("ax");
        assert!(
            tree.lines().next().is_some_and(|line| {
                line.starts_with("AXWindow: ") && line.contains("Native probe fixture")
            }),
            "{tree}"
        );
        assert!(tree.lines().any(|line| line == " AXButton"), "{tree}");

        let received = Rc::new(Cell::new(0));
        let count = received.clone();
        let token = observe(window, ns_string!("RiaProbeNotification"), move || {
            count.set(count.get() + 1)
        });
        second.orderFrontRegardless();
        let received_second = Rc::new(Cell::new(0));
        let count = received_second.clone();
        let second_token = observe(second, ns_string!("RiaProbeNotification"), move || {
            count.set(count.get() + 1)
        });
        assert_eq!(
            run("postNotification:window:RiaProbeNotification"),
            "posted"
        );
        assert_eq!(received.get(), 1);
        assert_eq!(received_second.get(), 0);
        assert_eq!(
            run("postNotification:windows:RiaProbeNotification"),
            "posted"
        );
        assert_eq!(received.get(), 2);
        assert_eq!(received_second.get(), 1);
        // SAFETY: The token belongs to this notification center.
        unsafe {
            let center = NSNotificationCenter::defaultCenter();
            center.removeObserver(&token);
            center.removeObserver(&second_token);
        }
        second.orderOut(None);

        let windows: Vec<Value> = serde_json::from_str(&run("subroles")).unwrap();
        assert!(
            windows
                .iter()
                .any(|entry| entry["number"] == window.windowNumber()
                    && entry["title"] == "Native probe fixture"
                    && entry["subrole"] == "AXStandardWindow")
        );
        window.orderOut(None);
        text.removeFromSuperview();

        let frame = unsafe { window.contentView().unwrap().superview() }.unwrap();
        let expected_point = NSPoint::new(-20., frame.bounds().size.height + 20.);
        assert_eq!(run("mouse:otherdown:-20,-20,2,1048576,7"), "");
        let mouse = app
            .nextEventMatchingMask_untilDate_inMode_dequeue(
                objc2_app_kit::NSEventMask::OtherMouseDown,
                Some(&NSDate::distantPast()),
                // SAFETY: AppKit defines this static run loop mode.
                unsafe { objc2_foundation::NSDefaultRunLoopMode },
                true,
            )
            .expect("mouse action did not enqueue the event");
        let point = mouse.locationInWindow();
        assert!((point.x - expected_point.x).abs() < 0.01);
        assert!((point.y - expected_point.y).abs() < 0.01);
        assert_eq!(mouse.windowNumber(), window.windowNumber());
        assert_eq!(mouse.eventNumber(), 7);
        assert_eq!(mouse.buttonNumber(), 2);
        assert_eq!(mouse.clickCount(), 2);
        assert!(
            mouse
                .modifierFlags()
                .contains(NSEventModifierFlags::Command)
        );
        assert!(!app.isActive());
        println!(
            "live_windows: guarded developer actions, native text and mouse input, accessibility, and notifications checked"
        );
    }

    #[derive(Default)]
    struct MouseFixture {
        key: Cell<bool>,
        events: RefCell<Vec<(NSEventType, NSPoint)>>,
    }

    define_class!(
        #[unsafe(super(NSWindow))]
        #[name = "RiaLiveMouseWindow"]
        #[ivars = MouseFixture]
        struct MouseWindow;

        impl MouseWindow {
            #[unsafe(method(isKeyWindow))]
            fn is_key(&self) -> Bool {
                Bool::new(self.ivars().key.get())
            }

            #[unsafe(method(sendEvent:))]
            fn receive(&self, event: &NSEvent) {
                use NSEventType as Event;
                match event.r#type() {
                    Event::LeftMouseDown
                    | Event::LeftMouseUp
                    | Event::LeftMouseDragged
                    | Event::RightMouseDown
                    | Event::RightMouseUp => {
                        self.ivars()
                            .events
                            .borrow_mut()
                            .push((event.r#type(), event.locationInWindow()));
                    }
                    _ => unsafe { msg_send![super(self), sendEvent: event] },
                }
            }
        }
    );

    fn check_native_mouse_delivery(host: &ChromeWindowHost, app: &NSApplication) {
        use NSEventType as Event;
        let frame = NSRect::new(NSPoint::ZERO, NSSize::new(240., 160.));
        let fixture = app
            .mtm()
            .alloc::<MouseWindow>()
            .set_ivars(MouseFixture::default());
        let window: Retained<MouseWindow> = unsafe {
            msg_send![super(fixture), initWithContentRect: frame,
                styleMask: NSWindowStyleMask::Titled, backing: NSBackingStoreType::Buffered, defer: false]
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.center();
        let text = NSTextView::initWithFrame(app.mtm().alloc(), frame);
        text.setEditable(false);
        text.setSelectable(false);
        window.setContentView(Some(&text));
        let run = |action| host.dev_action(&window, action, true).unwrap();
        let first = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            Event::LeftMouseDown, NSPoint::new(20., 20.), NSEventModifierFlags::empty(),
            0., window.windowNumber(), None, 0, 1, 1.,
        )
        .unwrap();
        let accepts = || text.acceptsFirstMouse(Some(&first));
        let baseline = accepts();
        window.ivars().key.set(true);
        assert!(
            !accepts(),
            "The unpatched text view already accepts first mouse"
        );
        assert_eq!(run("fakeActiveClicks"), "1");
        assert!(accepts());
        window.ivars().key.set(false);
        assert_eq!(accepts(), baseline);

        window.orderFrontRegardless();
        let frame_height = unsafe { text.superview().unwrap() }.bounds().size.height;
        let left = NSPoint::new(20., frame_height - 60.);
        let right = NSPoint::new(35., frame_height - 75.);
        let end = NSPoint::new(80., frame_height - 90.);
        let left_hit = run("click:20,60");
        assert_eq!(left_hit.split(" < ").next(), Some("NSTextView"));
        assert_eq!(run("click:35,75,right"), left_hit);
        for malformed in [
            "drag:20,60;flags=0;80,90",
            "drag:flags=0;flags=0;20,60;80,90",
            "drag:20,60;80,90;flags=0",
        ] {
            assert!(run(malformed).starts_with("drag:[flags="));
        }
        assert_eq!(
            run("drag:flags=0;20,60;80,90"),
            format!("{left_hit} (14 events)")
        );
        let app = app.retain();
        main_queue::after(
            Duration::from_millis(500),
            Box::new(move || {
                let mut expected = vec![
                    (Event::LeftMouseDown, left),
                    (Event::LeftMouseUp, left),
                    (Event::RightMouseDown, right),
                    (Event::RightMouseUp, right),
                    (Event::LeftMouseDown, left),
                ];
                for step in 1..=12 {
                    expected.push((
                        Event::LeftMouseDragged,
                        NSPoint::new(
                            left.x + 5. * f64::from(step),
                            left.y - 2.5 * f64::from(step),
                        ),
                    ));
                }
                expected.push((Event::LeftMouseUp, end));
                let events = window.ivars().events.borrow();
                assert_eq!(events.len(), expected.len());
                for (&(kind, point), (expected_kind, expected_point)) in events.iter().zip(expected)
                {
                    assert_eq!(kind, expected_kind);
                    assert!((point.x - expected_point.x).abs() < 0.01);
                    assert!((point.y - expected_point.y).abs() < 0.01);
                }
                drop(events);
                assert!(!app.isActive());
                window.close();
            }),
        );
    }

    fn check_pending_fullscreen_snapshot(
        tab: &Tab,
        supersede: impl FnOnce() + 'static,
        verify: impl FnOnce() + 'static,
    ) {
        let view = tab.view();
        let page = std::ptr::from_ref(&*view);
        let parent = unsafe { view.superview() }.unwrap();
        let expected_parent = std::ptr::from_ref(&*parent);
        let checked_parent = Rc::new(Cell::new(false));
        let checked = checked_parent.clone();
        let superview = sel!(superview);
        let method = view.class().instance_method(superview).unwrap();
        let implementation = method.implementation();
        let original: unsafe extern "C-unwind" fn(*const NSView, Sel) -> *const NSView =
            unsafe { std::mem::transmute(implementation) };
        let lookup = RcBlock::new(move |view: *const NSView| {
            let parent = unsafe { original(view, superview) };
            if view == page && parent == expected_parent {
                checked.set(true);
            }
            parent
        });
        let lookup_imp = unsafe { ffi::imp_implementationWithBlock(RcBlock::as_ptr(&lookup).cast()) };
        let superview_method = method;
        let superview_imp = implementation;
        let attached = Rc::new(Cell::new(false));
        let attachment = attached.clone();
        let selector = sel!(viewWillMoveToSuperview:);
        let class = PageCover::class();
        let method = class.instance_method(selector).unwrap();
        let original: unsafe extern "C-unwind" fn(*const NSView, Sel, *const NSView) =
            unsafe { std::mem::transmute(method.implementation()) };
        let attach = RcBlock::new(move |view: *const NSView, parent: *const NSView| {
            if parent == page {
                attachment.set(true);
            }
            unsafe { original(view, selector, parent) };
        });
        unsafe {
            ffi::class_replaceMethod(
                std::ptr::from_ref(class).cast_mut(),
                selector,
                ffi::imp_implementationWithBlock(RcBlock::as_ptr(&attach).cast()),
                ffi::method_getTypeEncoding(method),
            );
        }

        let target = std::ptr::from_ref(tab);
        let selector = sel!(copyPicture:completion:);
        let method = tab.class().instance_method(selector).unwrap();
        let implementation = method.implementation();
        let original: unsafe extern "C-unwind" fn(*const Tab, Sel, f64, &DynBlock<dyn Fn(CGImageRef)>) =
            unsafe { std::mem::transmute(implementation) };
        let work = Cell::new(Some((supersede, verify)));
        let snapshot = RcBlock::new(
            move |tab: *const Tab, timeout: f64, completion: &DynBlock<dyn Fn(CGImageRef)>| {
                if tab == target && let Some((supersede, verify)) = work.take() {
                    unsafe { ffi::method_setImplementation(method, implementation) };
                    main_queue::post(Box::new(supersede));
                    let completion = completion.copy();
                    let verify = Cell::new(Some(verify));
                    let attached = attached.clone();
                    let checked_parent = checked_parent.clone();
                    let reply = RcBlock::new(move |image: CGImageRef| {
                        assert!(!image.0.is_null(), "The fullscreen snapshot must contain an image");
                        main_queue::post(Box::new(move || unsafe {
                            ffi::method_setImplementation(superview_method, lookup_imp);
                        }));
                        completion.call((image,));
                        let attached = attached.clone();
                        let checked_parent = checked_parent.clone();
                        let verify = verify.take().unwrap();
                        main_queue::post(Box::new(move || {
                            unsafe {
                                ffi::method_setImplementation(superview_method, superview_imp);
                                ffi::imp_removeBlock(lookup_imp);
                            }
                            assert!(
                                checked_parent.get(),
                                "The snapshot handler must validate the parent before attachment"
                            );
                            assert!(!attached.get(), "The stale cover must never attach");
                            verify();
                        }));
                    });
                    unsafe { original(tab, selector, timeout, &reply) };
                } else {
                    unsafe { original(tab, selector, timeout, completion) };
                }
            },
        );
        unsafe {
            ffi::method_setImplementation(
                method,
                ffi::imp_implementationWithBlock(RcBlock::as_ptr(&snapshot).cast()),
            );
        }
    }

    fn check_exit_settlement(
        host: Rc<ChromeWindowHost>,
        controllers: &Controllers,
        profile: &Profile,
    ) -> Rc<Cell<bool>> {
        let window = host.make_window(profile, None).unwrap();
        let controller = controllers.for_window(&window).unwrap();
        let core = controller.core.clone();
        let replacement = core.open_tab(ns_string!("about:blank"), profile, false);
        let tab = core.open_tab(ns_string!("about:blank"), profile, true);
        let delegate: Retained<AnyObject> = unsafe { msg_send![&*core, delegate] };
        let change = move |tab: &Tab, entering| {
            let _: () = unsafe {
                msg_send![&*delegate, window: &*core, tab: tab,
                    didChangeFullscreen: Bool::new(entering)]
            };
        };
        controller.core.show_inactive();
        change(&tab, true);
        let completed = Rc::new(Cell::new(false));
        let done = completed.clone();
        let observed_window = window.clone();
        let notification = unsafe { NSWindowDidEnterFullScreenNotification };
        let entered = Cell::new(Some(move || {
            let selector = sel!(exitFullscreen);
            let method = tab.class().instance_method(selector).unwrap();
            let implementation = method.implementation();
            let original: unsafe extern "C-unwind" fn(*const Tab, Sel) =
                unsafe { std::mem::transmute(implementation) };
            let calls = Rc::new(RefCell::new(Vec::new()));
            let received = calls.clone();
            let exit = RcBlock::new(move |tab: *const Tab| {
                received.borrow_mut().push(tab);
                unsafe { original(tab, selector) };
            });
            let imp = unsafe { ffi::imp_implementationWithBlock(RcBlock::as_ptr(&exit).cast()) };
            unsafe { ffi::method_setImplementation(method, imp) };
            change(&tab, false);
            for action in ["fullScreen", "fakeFullScreen"] {
                let state = host.dev_action(&window, action, true).unwrap();
                let state: Value = serde_json::from_str(&state).unwrap();
                assert_eq!(state["pageFullScreen"]["waitingForPage"], -1);
            }
            change(&tab, true);
            host.dev_action(&window, "fakeFullScreen:0", true).unwrap();
            assert!(calls.borrow().is_empty());
            change(&replacement, true);
            main_queue::post(Box::new(move || {
                unsafe {
                    ffi::method_setImplementation(method, implementation);
                    ffi::imp_removeBlock(imp);
                }
                assert_eq!(
                    calls.borrow().as_slice(),
                    &[std::ptr::from_ref(&*replacement)]
                );
                host.close_window(&window);
                done.set(true);
            }));
        }));
        let token = Rc::new(Cell::<Option<Retained<AnyObject>>>::new(None));
        let observer = token.clone();
        token.set(Some(observe(&observed_window, notification, move || {
            let token = observer.take().unwrap();
            unsafe { NSNotificationCenter::defaultCenter().removeObserver(&token) };
            main_queue::post(Box::new(entered.take().unwrap()));
        })));
        completed
    }

    fn check_windows(profile: &Profile) {
        let mtm = MainThreadMarker::new().unwrap();
        let controllers = Controllers::new(mtm, true, true, |_| true);
        let normal = controllers.make_normal();
        let chrome = controllers.make_chrome();
        let token = chrome.chrome_token().unwrap();
        assert_eq!(chrome.kind(), WindowKind::AwaitingApp);
        assert!(Rc::ptr_eq(
            &controllers.claim_chrome(token).unwrap(),
            &chrome
        ));
        assert_eq!(chrome.kind(), WindowKind::Normal);
        assert!(controllers.claim_chrome(token).is_none());
        let stray = controllers.make_stray();
        assert_eq!(stray.kind(), WindowKind::Stray);
        assert!(controllers.user_sees(&stray.core.window()));
        let standalone = controllers.standalone(profile);
        assert_eq!(standalone.kind(), WindowKind::Standalone);
        assert!(standalone.core.internal());
        assert!(Rc::ptr_eq(&standalone, &controllers.standalone(profile)));
        assert_eq!(controllers.all().len(), 4);

        let window = normal.core.window();
        let lights = normal.traffic_lights.as_ref().unwrap();
        assert!(window.titlebarAppearsTransparent());
        let sight_events = Rc::new(Cell::new(0));
        let tally = sight_events.clone();
        let sight_observer = observe(&window, ns_string!("ArdiaWindowSeenDidChange"), move || {
            tally.set(tally.get() + 1);
        });
        window.orderOut(None);
        controllers.reconsider_sight(&window);
        assert!(!controllers.user_sees(&window));
        let events = sight_events.get();
        let app = NSApplication::sharedApplication(mtm);
        assert!(
            !app.isActive(),
            "The visibility check requires an inactive app"
        );
        check_reserved_replies(&app, mtm);
        window.orderFrontRegardless();
        wait_for("No Window Server visibility reply and notification", || {
            assert!(
                !app.isActive(),
                "The app activated during the visibility check"
            );
            controllers.user_sees(&window) && sight_events.get() > events
        });
        let cover = chrome.core.window();
        cover.setFrame_display(window.frame(), true);
        cover.setBackgroundColor(Some(&NSColor::blackColor()));
        cover.setOpaque(true);
        cover.setAlphaValue(1.);
        for covered in [true, false] {
            let events = sight_events.get();
            if covered {
                cover.orderFrontRegardless();
            } else {
                cover.orderOut(None);
            }
            wait_for(
                "No inactive sight reply and notification after changing cover",
                || {
                    assert!(!app.isActive(), "The app activated during the cover check");
                    assert!(window.isVisible(), "The target must stay ordered in");
                    controllers.reconsider_sight(&window);
                    controllers.user_sees(&window) != covered && sight_events.get() > events
                },
            );
        }
        let placement = Placement {
            center: Some(NSPoint::new(50., 80.)),
            trailing: false,
        };
        lights.set_placement(placement);
        let close = window
            .standardWindowButton(NSWindowButton::CloseButton)
            .unwrap();
        let frame = unsafe { window.contentView().unwrap().superview().unwrap() };
        let rect = close.convertRect_toView(close.bounds(), Some(&frame));
        assert!((rect.origin.x + rect.size.width / 2. - 50.).abs() < 1.);
        let mid_y = rect.origin.y + rect.size.height / 2.;
        let y_from_top = if frame.isFlipped() {
            mid_y
        } else {
            frame.bounds().size.height - mid_y
        };
        assert!((y_from_top - 80.).abs() < 1.);
        let button_center = frame.convertPoint_toView(NSPoint::new(50., mid_y), None);
        assert!(
            frame
                .hitTest(button_center)
                .is_some_and(|hit| hit.isDescendantOf(&close))
        );

        let before_placement = window.frame();
        lights.set_placement(Placement {
            center: Some(NSPoint::new(120., 80.)),
            trailing: true,
        });
        let trailing = close.convertRect_toView(close.bounds(), Some(&frame));
        let trailing_x = trailing.origin.x + trailing.size.width / 2.;
        assert!((trailing_x - (before_placement.size.width - 120.)).abs() < 1.);
        assert!((trailing.origin.y + trailing.size.height / 2. - mid_y).abs() < 1.);
        assert_eq!(window.frame(), before_placement);

        let events = sight_events.get();
        window.orderOut(None);
        wait_for("No hidden sight reply and notification", || {
            !controllers.user_sees(&window) && sight_events.get() > events
        });
        assert!(!controllers.user_sees_only_through_window_server(&window));

        let screen = window
            .screen()
            .or_else(|| NSScreen::mainScreen(mtm))
            .expect("Acted fullscreen requires a screen");
        let screen_frame = screen.frame();
        let visible = screen.visibleFrame();
        let oversized = NSRect::new(
            screen_frame.origin,
            NSSize::new(visible.size.width + 100., visible.size.height + 100.),
        );
        assert_eq!(frame_on_screen(mtm, oversized, &screen), visible);
        println!("live_windows: AppKit placement fits an oversized frame to the display work area");
        window.setFrame_display(
            NSRect::new(
                NSPoint::new(screen_frame.origin.x + 40., screen_frame.origin.y + 40.),
                NSSize::new(600., 500.),
            ),
            false,
        );
        let before = window.frame();
        assert_ne!(before, screen_frame);
        let events = Rc::new(RefCell::new(Vec::new()));
        let names = unsafe {
            [
                NSWindowWillEnterFullScreenNotification,
                NSWindowDidEnterFullScreenNotification,
                NSWindowWillExitFullScreenNotification,
                NSWindowDidExitFullScreenNotification,
            ]
        };
        let observers = names.map(|name| {
            let events = events.clone();
            let observed_window = window.clone();
            observe(&window, name, move || {
                events.borrow_mut().push((name, observed_window.frame()));
            })
        });
        normal.fullscreen.act_window_full_screen(true);
        assert_eq!(
            events.borrow().as_slice(),
            &[(names[0], before), (names[1], screen_frame)]
        );
        assert_eq!(window.frame(), screen_frame);
        assert!(normal.core.acted_full_screen() && normal.fullscreen.is_full_screen());
        assert!(!normal.fullscreen.transitioning());
        normal.fullscreen.act_window_full_screen(false);
        assert_eq!(
            events.borrow().as_slice(),
            &[
                (names[0], before),
                (names[1], screen_frame),
                (names[2], screen_frame),
                (names[3], before),
            ]
        );
        assert!(!normal.core.acted_full_screen() && !normal.fullscreen.is_full_screen());
        assert_eq!(window.frame(), before);
        assert!(!normal.fullscreen.transitioning());
        let center = NSNotificationCenter::defaultCenter();
        assert!(!app.isActive());
        for observer in observers.into_iter().chain([sight_observer]) {
            unsafe { center.removeObserver(&observer) };
        }
        check_native_close(mtm);
        println!(
            "live_windows: reserved key replies, controller claims, titlebar hit testing, visibility, acted fullscreen, and native close checked"
        );
        println!(
            "live_windows: Chromium key dispatch to an AppKit menu action not checked; the harness does not activate the app or make a window key"
        );
        println!("live_windows: foreground AppKit fullscreen entry and exit not checked");
        println!(
            "live_windows: native sharing after trailing placement without resize not checked (indicator and child pill)"
        );
        let state = State::new();
        state.started.set(true);
        let controllers = Controllers::new(mtm, true, false, |_| true);
        let host = Rc::new(ChromeWindowHost::new(mtm, state, controllers.clone()));
        let made = host.make_window(profile, None).unwrap();
        assert_eq!(
            host.dev_action(&made, "fakeFullScreen:1", false).as_deref(),
            Some("test instances only")
        );
        assert!(!host.is_fullscreen(&made));
        let fresh = host.make_window(profile, None).unwrap();
        check_dev_actions(&host, &made, &fresh, &app);
        let probe = |host: &ChromeWindowHost, window: &NSWindow, action| {
            let reply = host.dev_action(window, action, true).unwrap();
            serde_json::from_str::<Value>(&reply).unwrap()
        };
        let before_sight = probe(&host, &fresh, "fakeFullScreen:1:50");
        assert_eq!(before_sight["fullScreen"], true);
        assert!(before_sight.get("pageFullScreen").is_none());
        assert!(controllers.user_sees(&fresh));
        let page = &probe(&host, &fresh, "fullScreen")["pageFullScreen"];
        assert_eq!(page["transitioning"], true);
        assert_eq!(page["settling"], true);
        host.close_window(&fresh);
        let occluded = host.make_window(profile, None).unwrap();
        occluded.orderOut(None);
        assert_eq!(
            host.dev_action(&occluded, "fakeOcclusion:hidden", true).as_deref(),
            Some("0")
        );
        assert!(
            probe(&host, &occluded, "fullScreen")
                .get("pageFullScreen")
                .is_none()
        );
        assert!(!controllers.user_sees(&occluded));
        assert_eq!(
            probe(&host, &occluded, "fakeFullScreen")["pageFullScreen"]["seen"],
            false
        );
        host.close_window(&occluded);
        let exit_checked = check_exit_settlement(host.clone(), &controllers, profile);

        let page_window = host.make_window(profile, None).unwrap();
        let controller = controllers.for_window(&page_window).unwrap();
        let tab = controller
            .core
            .open_tab(ns_string!("about:blank"), profile, true);
        let delegate: Retained<AnyObject> = unsafe { msg_send![&*controller.core, delegate] };
        let change_page = {
            let core = controller.core.clone();
            let tab = tab.clone();
            move |entering| {
                let _: () = unsafe {
                    msg_send![&*delegate, window: &*core, tab: &*tab,
                        didChangeFullscreen: Bool::new(entering)]
                };
            }
        };
        assert!(
            probe(&host, &page_window, "fullScreen")
                .get("pageFullScreen")
                .is_none()
        );
        change_page(true);
        let page = &probe(&host, &page_window, "fakeFullScreen")["pageFullScreen"];
        assert_eq!(page["page"], tab.browser_id());
        assert_eq!(page["telling"], 1);
        for action in ["fullScreen", "fakeFullScreen"] {
            assert_eq!(
                probe(&host, &page_window, action)["pageFullScreen"]["waitingForPage"],
                1
            );
        }
        change_page(false);
        let page = &probe(&host, &page_window, "fullScreen")["pageFullScreen"];
        assert!(page["page"].is_null());
        assert_eq!(page["telling"], 2);
        controller.core.show_inactive();
        assert!(tab.view().window().is_some());
        assert!(!tab.view().isHiddenOrHasHiddenAncestor());
        change_page(true);
        let page_host = host.clone();
        let stale_checked = Rc::new(Cell::new(false));
        let completed = stale_checked.clone();
        main_queue::post(Box::new(move || {
            main_queue::after(
                Duration::from_millis(425),
                Box::new(move || {
                    let before = probe(&page_host, &page_window, "fullScreen");
                    assert_eq!(before["pageFullScreen"]["waitingForPage"], 1);
                    let after = probe(&page_host, &page_window, "fakeFullScreen:1");
                    assert_eq!(after["fullScreen"], true);
                    assert_eq!(after["pageFullScreen"]["waitingForPage"], 0);
                    assert_eq!(after["pageFullScreen"]["picturing"], false);
                    change_page(false);
                    probe(&page_host, &page_window, "fakeFullScreen:0");
                    let snapshot_host = page_host.clone();
                    let snapshot_window = page_window.clone();
                    let superseded = Rc::new(Cell::new(None));
                    let result = superseded.clone();
                    check_pending_fullscreen_snapshot(
                        &tab,
                        move || {
                            let before = probe(&snapshot_host, &snapshot_window, "fullScreen");
                            assert_eq!(before["pageFullScreen"]["picturing"], true);
                            let after = probe(&snapshot_host, &snapshot_window, "fakeFullScreen:1");
                            assert_eq!(after["fullScreen"], true);
                            assert_eq!(after["pageFullScreen"]["picturing"], false);
                            assert_eq!(after["pageFullScreen"]["covered"], false);
                            result.set(Some(after));
                        },
                        move || {
                            let after = superseded.take().unwrap();
                            assert_eq!(probe(&page_host, &page_window, "fullScreen"), after);
                            assert_eq!(probe(&page_host, &page_window, "fakeFullScreen"), after);
                            page_host.close_window(&page_window);
                            completed.set(true);
                        },
                    );
                    change_page(true);
                }),
            );
        }));
        let idle = serde_json::json!({
            "fullScreen": false,
            "styleMask": false,
            "acted": false,
            "chromeCounts": false,
        });
        assert_eq!(probe(&host, &made, "fullScreen"), idle);
        assert_eq!(probe(&host, &made, "fakeFullScreen"), idle);
        assert!(controllers.user_sees(&made));
        made.orderOut(None);
        for (action, expected, seen) in [
            ("fakeOcclusion:visible", "1", false),
            ("fakeOcclusion:hidden", "0", false),
            ("fakeOcclusion:off", "0", true),
        ] {
            assert_eq!(
                host.dev_action(&made, action, true).as_deref(),
                Some(expected)
            );
            assert_eq!(
                probe(&host, &made, "fullScreen")["pageFullScreen"]["seen"],
                seen
            );
        }
        let notes = Rc::new(RefCell::new(Vec::new()));
        let tokens = names.map(|name| {
            let notes = notes.clone();
            observe(&made, name, move || notes.borrow_mut().push(name))
        });
        let fullscreen_state = |full_screen, transitioning| {
            serde_json::json!({
                "fullScreen": full_screen,
                "styleMask": false,
                "acted": full_screen,
                "chromeCounts": full_screen,
                "pageFullScreen": {
                    "page": null,
                    "entered": false,
                    "asked": 0,
                    "waitingForPage": 0,
                    "telling": 0,
                    "picturing": false,
                    "covered": false,
                    "transitioning": transitioning,
                    "ours": false,
                    "settling": transitioning,
                    "seen": true,
                    "fullScreen": full_screen,
                },
            })
        };
        assert_eq!(
            probe(&host, &made, "fullScreen"),
            fullscreen_state(false, false)
        );
        assert_eq!(
            probe(&host, &made, "fakeFullScreen"),
            fullscreen_state(false, false)
        );
        assert_eq!(
            probe(&host, &made, "fakeFullScreen:1"),
            fullscreen_state(true, false)
        );
        assert_eq!(notes.borrow().as_slice(), &names[..2]);
        assert_eq!(
            probe(&host, &made, "fakeFullScreen:0"),
            fullscreen_state(false, false)
        );
        assert_eq!(notes.borrow().as_slice(), &names);

        let root = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        host.embed_root(&made, &root);
        assert!(
            host.root_view(&made)
                .is_some_and(|view| std::ptr::eq(&*view, &*root))
        );
        let minimum = NSSize::new(410., 230.);
        host.set_interface_min_size(minimum, &made);
        assert_eq!(made.minSize(), minimum);
        ardia_interface_scale::set_scale(1.2);
        let point = NSPoint::new(25., 50.);
        let in_window = ardia_interface_scale::window_point_from_root(point, &made);
        let origin = root.convertPoint_toView(NSPoint::ZERO, None);
        assert!((in_window.x - (origin.x + 30.)).abs() < 0.001);
        assert!((in_window.y - (origin.y + 60.)).abs() < 0.001);
        let logical = ardia_interface_scale::root_point_from_window(
            NSPoint::new(origin.x + 30., origin.y + 60.),
            &made,
        );
        assert!((logical.x - 25.).abs() < 0.001 && (logical.y - 50.).abs() < 0.001);
        assert_eq!(made.minSize(), NSSize::new(492., 276.));
        ardia_interface_scale::set_scale(1.);
        host.remove_root(&made);
        assert!(host.root_view(&made).is_none());
        let fitted = little_frame_for_size(
            NSSize::new(900., 700.),
            NSSize::new(100., 100.),
            NSRect::new(NSPoint::ZERO, NSSize::new(1000., 800.)),
        );
        assert_eq!(
            fitted,
            NSRect::new(NSPoint::new(100., 80.), NSSize::new(800., 640.))
        );
        println!(
            "live_windows: isolated scale, scaled minimum, and absolute root coordinates checked"
        );
        host.set_traffic_lights(
            &window,
            Placement {
                center: Some(NSPoint::new(-100., 80.)),
                trailing: true,
            },
            true,
        );
        assert!(close.isHidden());
        host.set_traffic_lights(
            &window,
            Placement {
                center: Some(NSPoint::new(160., 80.)),
                trailing: true,
            },
            true,
        );
        let profile = profile.retain();
        notes.borrow_mut().clear();
        let entered_at = Rc::new(Cell::new(None));
        let enter_time = entered_at.clone();
        let start = Instant::now();
        let enter_token = observe(&made, names[1], move || {
            enter_time.set(Some(start.elapsed()))
        });
        assert_eq!(
            probe(&host, &made, "fakeFullScreen:1: 50"),
            fullscreen_state(true, true)
        );
        assert_eq!(notes.borrow().as_slice(), &names[..1]);
        main_queue::after(
            Duration::from_millis(100),
            Box::new(move || {
                assert!(entered_at.get().unwrap() >= Duration::from_millis(50));
                unsafe { center.removeObserver(&enter_token) };
                assert_eq!(notes.borrow().as_slice(), &names[..2]);
                assert_eq!(
                    probe(&host, &made, "fullScreen"),
                    fullscreen_state(true, false)
                );
                let start = Instant::now();
                let exit_at = Rc::new(Cell::new(None));
                let exit_time = exit_at.clone();
                let exit_token = observe(&made, names[3], move || {
                    exit_time.set(Some(start.elapsed()))
                });
                controllers.for_window(&made).unwrap().fullscreen.toggle();
                assert_eq!(notes.borrow().as_slice(), &names[..3]);
                assert_eq!(
                    probe(&host, &made, "fakeFullScreen"),
                    fullscreen_state(true, true)
                );
                let restored = host.make_window(&profile, None).unwrap();
                let overridden = host.make_window(&profile, None).unwrap();
                for window in [&restored, &overridden] {
                    controllers.for_window(window).unwrap().core.show_inactive();
                    assert!(window.isVisible());
                    assert_eq!(
                        host.dev_action(window, "fakeWindowServer:shows", true).as_deref(),
                        Some("ok")
                    );
                }
                assert_eq!(
                    host.dev_action(&restored, "fakeFullScreenOcclusionMs: 500", true)
                        .as_deref(),
                    Some("500")
                );
                for window in [&restored, &overridden] {
                    assert_eq!(
                        probe(&host, window, "fakeFullScreen:1:-500")["fullScreen"],
                        true
                    );
                    assert!(controllers.user_sees_only_through_window_server(window));
                }
                assert_eq!(
                    host.dev_action(&restored, "fakeFullScreenOcclusionMs:-1", true)
                        .as_deref(),
                    Some("-1")
                );
                for window in [&restored, &overridden] {
                    assert_eq!(probe(&host, window, "fakeFullScreen:0")["fullScreen"], false);
                    assert!(controllers.user_sees_only_through_window_server(window));
                }
                assert_eq!(
                    host.dev_action(&overridden, "fakeOcclusion:hidden", true).as_deref(),
                    Some("0")
                );
                let restored_before_gap = restored.clone();
                let controllers_before_gap = controllers.clone();
                main_queue::after(
                    Duration::from_millis(100),
                    Box::new(move || {
                        assert!(
                            !controllers_before_gap
                                .user_sees_only_through_window_server(&restored_before_gap)
                        );
                    }),
                );
                main_queue::after(
                    Duration::from_millis(1700),
                    Box::new(move || {
                        assert!(stale_checked.get());
                        assert!(exit_checked.get());
                        assert!(!controllers.user_sees_only_through_window_server(&restored));
                        assert!(controllers.user_sees_only_through_window_server(&overridden));
                        host.close_window(&restored);
                        host.close_window(&overridden);
                        assert!(exit_at.get().unwrap() >= Duration::from_millis(500));
                        assert_eq!(notes.borrow().as_slice(), &names);
                        assert_eq!(
                            probe(&host, &made, "fullScreen"),
                            fullscreen_state(false, false)
                        );
                        assert_eq!(
                            host.dev_action(&made, "fakeFullScreenMs: 50", true).as_deref(),
                            Some("50")
                        );
                        assert_eq!(
                            host.dev_action(&made, "fakeFullScreenMs:0", true)
                                .as_deref(),
                            Some("0")
                        );
                        for token in tokens.into_iter().chain([exit_token]) {
                            // SAFETY: Each token belongs to this notification center.
                            unsafe { center.removeObserver(&token) };
                        }
                        let controller = controllers.for_window(&made).unwrap();
                        controller.fullscreen.toggle();
                        assert!(host.is_fullscreen(&made));
                        assert!(!controller.fullscreen.transitioning());
                        controller.fullscreen.toggle();
                        assert!(!host.is_fullscreen(&made));
                        host.close_window(&made);
                        println!(
                            "live_windows: fullscreen reply fields, explicit and background timing, and occlusion override checked"
                        );
                        let rect = close.convertRect_toView(close.bounds(), Some(&frame));
                        let expected = window.frame().size.width - 160.;
                        assert!((rect.origin.x + rect.size.width / 2. - expected).abs() < 1.);

                        check_popup(&window, mtm, &profile);
                        for controller in [&normal, &chrome, &stray, &standalone] {
                            controller.core.close();
                        }
                    }),
                );
            }),
        );
    }

    fn check_popup(window: &NSWindow, mtm: MainThreadMarker, profile: &Profile) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let url = format!("{origin}/private?q=secret");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            for line in BufReader::new(&stream).lines() {
                if line.unwrap().is_empty() {
                    break;
                }
            }
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 20\r\nConnection: close\r\n\r\n<title>Probe</title>").unwrap();
        });
        let popups = Popups::new(mtm, true);
        let popup = popups.for_chrome(Some(window), false);
        assert_eq!(popups.count(), 1);
        assert!(
            popup
                .window()
                .styleMask()
                .contains(objc2_app_kit::NSWindowStyleMask::Resizable)
        );
        let tab = popup.open_tab(&NSString::from_str(&url), profile, false);
        check_popup_origin(
            popups,
            popup,
            tab,
            url,
            origin,
            Instant::now() + Duration::from_secs(10),
        );
    }

    fn check_popup_origin(
        popups: Rc<Popups>,
        popup: Retained<Window>,
        tab: Retained<Tab>,
        url: String,
        origin: String,
        deadline: Instant,
    ) {
        if tab.url().to_string() != url || tab.title().to_string() != "Probe" {
            assert!(Instant::now() < deadline, "Popup did not load {url}");
            main_queue::after(
                Duration::from_millis(25),
                Box::new(move || {
                    check_popup_origin(popups, popup, tab, url, origin, deadline);
                }),
            );
            return;
        }
        let window = popup.window();
        assert!(window.isVisible(), "The popup must be ordered in");
        assert!(!NSApplication::sharedApplication(window.mtm()).isActive());
        let host = popup.host_view();
        let page = tab.view();
        assert!(
            page.window()
                .is_some_and(|owner| std::ptr::eq(&*owner, &*window))
        );
        let strip = host
            .subviews()
            .iter()
            .find_map(|view| view.downcast::<PopupOriginStrip>().ok())
            .unwrap();
        let label = strip
            .subviews()
            .iter()
            .find_map(|view| view.downcast::<NSTextField>().ok())
            .unwrap();
        assert!(unsafe { page.superview() }.is_some_and(|parent| std::ptr::eq(&*parent, &*host)));
        let bounds = host.bounds();
        let mut strip_frame = bounds;
        strip_frame.size.height = 28.;
        let mut page_frame = bounds;
        page_frame.size.height -= 28.;
        if host.isFlipped() {
            page_frame.origin.y += 28.;
        } else {
            strip_frame.origin.y += page_frame.size.height;
        }
        assert_eq!(strip.frame(), strip_frame);
        assert_eq!(page.frame(), page_frame);
        assert!(!label.isHiddenOrHasHiddenAncestor());
        let visible_label = NSIntersectionRect(
            strip.visibleRect(),
            label.convertRect_toView(label.visibleRect(), Some(&strip)),
        );
        assert!(visible_label.size.width > 0. && visible_label.size.height > 0.);
        assert_eq!(label.stringValue().to_string(), origin);
        assert!(window.alphaValue() > 0. && strip.alphaValue() > 0. && label.alphaValue() > 0.);
        let region = strip.convertRect_toView(visible_label, Some(&host));
        let capture = || {
            let bitmap = host.bitmapImageRepForCachingDisplayInRect(region).unwrap();
            host.cacheDisplayInRect_toBitmapImageRep(region, &bitmap);
            bitmap.TIFFRepresentation().unwrap()
        };
        let with_text = capture();
        label.setStringValue(ns_string!(""));
        let without_text = capture();
        label.setStringValue(&NSString::from_str(&origin));
        assert!(
            !with_text.isEqualToData(&without_text),
            "The origin text must change the rendered pixels"
        );
        println!("live_windows: popup rendered its 28-point origin strip with {origin}");
        popup.close();
        check_popup_closed(popups, Instant::now() + Duration::from_secs(5));
    }

    fn check_popup_closed(popups: Rc<Popups>, deadline: Instant) {
        if popups.count() == 0 {
            println!(
                "live_windows: window claims, shortcuts, fullscreen, root scaling, display fit, and popup tab placement checked"
            );
            let _: () = Engine::shared().unwrap().quit();
            return;
        }
        assert!(Instant::now() < deadline, "engine did not close the popup");
        main_queue::after(
            Duration::from_millis(25),
            Box::new(move || check_popup_closed(popups, deadline)),
        );
    }

    struct WindowHooks;

    impl Hooks for WindowHooks {
        fn engine_started(&self) {}
        fn engine_shut_down(&self) {}
        fn emit(&self, _: &str, _: Value) {}
        fn listens(&self, _: &str) -> bool {
            false
        }
        fn app_tab_id_value(&self, _: Option<&Tab>) -> Value {
            Value::Null
        }
        fn profile_name(&self, _: &Profile) -> String {
            "Default".into()
        }
        fn names_of_loaded_profile(&self, _: &Profile) -> Vec<String> {
            vec!["Default".into()]
        }
        fn ask_permission_in_popup_window(&self, _: &Tab, _: &Value) -> bool {
            false
        }
        fn hold_until_hosted(&self, _: &Tab, _: &str, _: Value) {}
        fn drop_popup_window_permission(&self, _: &str) {}
        fn drop_held_event(&self, _: &str, _: &str) {}
        fn is_reserved_binding(&self, key: &str, unshifted: &str, modifiers: usize) -> bool {
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            is_reserved_binding(
                &app,
                &NSString::from_str(key),
                &NSString::from_str(unshifted),
                NSEventModifierFlags::from_bits_retain(modifiers),
            )
        }
        fn is_reserved_key_event(&self, event: &NSEvent) -> bool {
            is_reserved_key_event(
                MainThreadMarker::new().unwrap(),
                event,
                KeyContext::Windowed,
            )
        }
        fn window_for_new_browser(&self, _: &Profile, _: &str) -> Option<Retained<Window>> {
            None
        }
    }
}
