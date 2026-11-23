// Ria original, no Ardia source.

//! Run from the repository root on a Mac:
//! `ENGINE_DIR=/path/to/framework-parent cargo test --manifest-path rust/Cargo.toml -p ria-host-mac --test live_windows`.

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
    use std::ffi::CString;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use block2::RcBlock;
    use objc2::runtime::{AnyObject, Bool};
    use objc2::{MainThreadMarker, Message, msg_send, rc::Retained};
    use objc2_app_kit::{
        NSApplication, NSColor, NSEvent, NSEventModifierFlags, NSEventType, NSMenu, NSMenuItem,
        NSScreen, NSUserInterfaceItemIdentification, NSView, NSWindow, NSWindowButton,
        NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification,
        NSWindowWillEnterFullScreenNotification, NSWindowWillExitFullScreenNotification,
    };
    use objc2_foundation::{
        NSDate, NSNotification, NSNotificationCenter, NSPoint, NSRect, NSRunLoop, NSSize, NSString,
        ns_string,
    };
    use ria_engine::call::main_queue;
    use ria_host_mac::core::bindings::{Engine, Profile, Tab, Window};
    use ria_host_mac::core::delegate::{EngineDelegate, Hooks};
    use ria_host_mac::core::host::State;
    use ria_host_mac::windows::ardia_chrome_window_host::ChromeWindowHost;
    use ria_host_mac::windows::ardia_chrome_window_host_little_fit::little_frame_for_size;
    use ria_host_mac::windows::ardia_interface_scale;
    use ria_host_mac::windows::ardia_popup_origin_strip::popup_origin_text;
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
        let contents = root.join("Live.app/Contents");
        let exe = contents.join("MacOS/live_windows");
        let data = root.join("data");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::create_dir(contents.join("Frameworks")).unwrap();
        std::fs::create_dir(&data).unwrap();
        std::fs::write(
            contents.join("Info.plist"),
            br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>live_windows</string>
<key>CFBundleIdentifier</key><string>sh.ria.windows-live</string>
<key>CFBundleName</key><string>Ria Windows Live</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>0</string>
<key>NSPrincipalClass</key><string>BrowserCrApplication</string>
</dict></plist>"#,
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
        let status = Command::new(&exe).env("RIA_WINDOWS_DATA", data).status();
        std::fs::remove_dir_all(root).unwrap();
        assert!(status.unwrap().success());
    }

    pub fn child() {
        let executable = std::env::current_exe().unwrap();
        let contents = executable.parent().and_then(Path::parent).unwrap();
        let library = contents
            .join("Frameworks")
            .join(FRAMEWORK)
            .join("Versions/Current/Chromium Framework");
        let library = CString::new(library.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: Load the framework and register its Objective-C classes before using the bindings.
        assert!(
            !unsafe { libc::dlopen(library.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) }
                .is_null()
        );
        let data = std::env::var("RIA_WINDOWS_DATA").unwrap();
        let argv: Vec<_> = [
            std::env::args().next().unwrap(),
            format!("--user-data-dir={data}/Chromium"),
            "--use-mock-keychain".into(),
            "--profile-directory=Default".into(),
            "--no-first-run".into(),
            "--disable-stack-profiler".into(),
        ]
        .into_iter()
        .map(|arg| CString::new(arg).unwrap())
        .collect();
        let hooks = Rc::new(WindowHooks::default());
        let delegate = EngineDelegate::new(
            hooks.clone(),
            State::new(),
            Some("/* window check */".into()),
        );
        // SAFETY: This is the main thread and the framework has been loaded.
        let exit = unsafe { delegate.run(&argv) };
        assert_eq!(exit, 0);
        assert!(hooks.shutdown.get());
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
        let delegate = EngineDelegate::new(Rc::new(WindowHooks::default()), State::new(), None);
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
        normal.fullscreen.toggle();
        assert_eq!(window.frame(), screen_frame);
        normal.fullscreen.act_window_full_screen(true);
        assert_eq!(
            events.borrow().as_slice(),
            &[(names[0], before), (names[1], screen_frame)]
        );
        assert_eq!(window.frame(), screen_frame);
        assert!(normal.core.acted_full_screen() && normal.fullscreen.is_full_screen());
        assert!(!normal.fullscreen.transitioning());
        normal.fullscreen.toggle();
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
        let host = ChromeWindowHost::new(mtm, state, controllers.clone());
        let made = host
            .make_window(profile, None, Some(ns_string!("ria:live")))
            .unwrap();
        assert_eq!(
            host.dev_action(&made, "fakeFullScreen:1", false).as_deref(),
            Some("test instances only")
        );
        assert!(!host.is_fullscreen(&made));
        for (command, expected) in [("fakeFullScreen:1:0", true), ("fakeFullScreen:0:0", false)] {
            let reply = host.dev_action(&made, command, true).unwrap();
            let reply: Value = serde_json::from_str(&reply).unwrap();
            assert_eq!(reply["fullScreen"], expected);
            assert_eq!(host.is_fullscreen(&made), expected);
        }
        let delayed = host.dev_action(&made, "fakeFullScreen:1:50", true).unwrap();
        let delayed: Value = serde_json::from_str(&delayed).unwrap();
        assert_eq!(delayed["transitioning"], true);
        let root = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        host.embed_root(&made, &root);
        assert!(
            host.root_view(&made)
                .is_some_and(|view| std::ptr::eq(&*view, &*root))
        );
        let point = NSPoint::new(17., 49.);
        let in_window = ardia_interface_scale::window_point_from_root(point, &made);
        assert_eq!(
            ardia_interface_scale::root_point_from_window(in_window, &made),
            point
        );
        let minimum = NSSize::new(410., 230.);
        host.set_interface_min_size(minimum, &made);
        let previous = ardia_interface_scale::scale();
        let updated = if previous == 1.2 { 1.0 } else { 1.2 };
        ardia_interface_scale::set_scale(updated);
        let minimum_at_updated_scale = made.minSize();
        ardia_interface_scale::set_scale(previous);
        assert_eq!(
            minimum_at_updated_scale,
            NSSize::new((410. * updated).round(), (230. * updated).round())
        );
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
        assert_eq!(
            popup_origin_text("https://example.test:8443/private?q=secret"),
            "example.test:8443"
        );
        host.set_traffic_lights(
            &window,
            Placement {
                center: Some(NSPoint::new(160., 80.)),
                trailing: true,
            },
            true,
        );
        let profile = profile.retain();
        main_queue::after(
            Duration::from_millis(250),
            Box::new(move || {
                let reply = host.dev_action(&made, "fullScreen", true).unwrap();
                let reply: Value = serde_json::from_str(&reply).unwrap();
                assert_eq!(reply["transitioning"], false);
                assert!(host.is_fullscreen(&made));
                host.dev_action(&made, "fakeFullScreen:0:0", true).unwrap();
                host.close_window(&made);
                let rect = close.convertRect_toView(close.bounds(), Some(&frame));
                let expected = window.frame().size.width - 160.;
                assert!((rect.origin.x + rect.size.width / 2. - expected).abs() < 1.);

                let popups = Popups::new(mtm, true);
                let browser_popup = popups.for_chrome(Some(&window), false);
                assert_eq!(popups.count(), 1);
                assert!(
                    browser_popup
                        .window()
                        .styleMask()
                        .contains(objc2_app_kit::NSWindowStyleMask::Resizable)
                );
                let tab = browser_popup.open_tab(ns_string!("about:blank"), &profile, false);
                main_queue::post(Box::new(move || {
                    let window = browser_popup.window();
                    let page = tab.view();
                    assert!(
                        page.window()
                            .is_some_and(|owner| std::ptr::eq(&*owner, &*window))
                    );
                    assert!(
                        page.frame().size.height < browser_popup.host_view().bounds().size.height
                    );
                    browser_popup.close();
                    check_popup_closed(popups, Instant::now() + Duration::from_secs(5));
                }));
                for controller in [&normal, &chrome, &stray, &standalone] {
                    controller.core.close();
                }
            }),
        );
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

    #[derive(Default)]
    struct WindowHooks {
        shutdown: Cell<bool>,
    }

    impl Hooks for WindowHooks {
        fn engine_started(&self) {
            main_queue::post(Box::new(|| {
                let engine = Engine::shared().unwrap();
                engine.load_profile(ns_string!("Default"), move |profile| {
                    main_queue::post(Box::new(move || {
                        check_windows(&profile.expect("Default profile"));
                    }));
                });
            }));
        }
        fn engine_shut_down(&self) {
            self.shutdown.set(true);
        }
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
