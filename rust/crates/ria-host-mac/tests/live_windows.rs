// Ria original, no Ardia source.

//! Run on a Mac: `ENGINE_DIR=<framework-parent> blaze test -p ria-host-mac --test live_windows`.

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
    use std::cell::Cell;
    use std::ffi::CString;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use block2::RcBlock;
    use objc2::{MainThreadMarker, rc::Retained, runtime::AnyObject, sel};
    use objc2_app_kit::{
        NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSMenu, NSMenuItem,
        NSScreen, NSUserInterfaceItemIdentification, NSView, NSWindow, NSWindowButton,
        NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification,
    };
    use objc2_foundation::{
        NSDate, NSNotification, NSNotificationCenter, NSPoint, NSRect, NSRunLoop, NSSize, NSString,
        ns_string,
    };
    use ria_engine::call::main_queue;
    use ria_host_mac::core::bindings::{Engine, Profile, Tab, Window};
    use ria_host_mac::core::delegate::{EngineDelegate, Hooks};
    use ria_host_mac::core::host::State;
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

    fn observe(window: &NSWindow, name: &NSString) -> (Retained<AnyObject>, Rc<Cell<usize>>) {
        let count = Rc::new(Cell::new(0));
        let tally = count.clone();
        let callback = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
            tally.set(tally.get() + 1);
        });
        let token = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(name),
                Some(window),
                None,
                &callback,
            )
        };
        (token.into(), count)
    }

    fn check_shortcut(core: &Window, profile: &Profile, mtm: MainThreadMarker) {
        let window = core.window();
        let tab = core.open_tab(ns_string!("about:blank"), profile, true);
        let host = core.host_view();
        let page = tab.view();
        page.setFrame(host.bounds());
        host.addSubview(&page);
        let app = NSApplication::sharedApplication(mtm);
        app.activate();
        window.makeKeyAndOrderFront(None);
        wait_for("The page did not finish loading", || {
            !tab.loading() && tab.has_live_frame(ns_string!(""))
        });
        tab.focus();
        let responder = window
            .firstResponder()
            .and_then(|responder| responder.downcast::<NSView>().ok())
            .filter(|responder| responder.isDescendantOf(&page))
            .expect("The engine page must have keyboard focus");

        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        let item = NSMenuItem::new(mtm);
        let f8 = ns_string!("\u{f70b}");
        item.setTitle(ns_string!("Hide test window"));
        item.setIdentifier(Some(ns_string!("ardia.command.closeTab")));
        item.setKeyEquivalent(f8);
        item.setKeyEquivalentModifierMask(NSEventModifierFlags::empty());
        unsafe {
            item.setAction(Some(sel!(orderOut:)));
            item.setTarget(Some(&window));
        }
        menu.addItem(&item);
        let previous_menu = app.mainMenu();
        app.setMainMenu(Some(&menu));

        let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown,
            NSPoint::ZERO,
            NSEventModifierFlags::empty(),
            0.,
            window.windowNumber(),
            None,
            f8,
            f8,
            false,
            100,
        )
        .expect("F8 key event");
        assert!(window.isVisible());
        responder.keyDown(&event);
        wait_for("The engine did not dispatch F8 to the menu action", || {
            !window.isVisible()
        });
        app.setMainMenu(previous_menu.as_deref());
    }

    fn check_native_close(mtm: MainThreadMarker) {
        let allow = Rc::new(Cell::new(false));
        let requests = Rc::new(Cell::new(0));
        let controllers = Controllers::new(
            mtm,
            true,
            true,
            {
                let allow = allow.clone();
                let requests = requests.clone();
                move |_| {
                    requests.set(requests.get() + 1);
                    allow.get()
                }
            },
        );
        let controller = controllers.make_normal();
        let window = controller.core.window();
        window.orderFrontRegardless();
        window.performClose(None);
        wait_for("The engine did not ask whether the window could close", || {
            requests.get() > 0
        });
        assert_eq!(requests.get(), 1);
        assert!(window.isVisible());
        let registered = controllers
            .for_window(&window)
            .expect("The vetoed window must stay registered");
        assert!(Rc::ptr_eq(&registered, &controller));
        assert_eq!(controllers.all().len(), 1);

        allow.set(true);
        window.performClose(None);
        wait_for("The accepted close did not remove the window", || {
            controllers.for_window(&window).is_none()
        });
        assert_eq!(requests.get(), 2);
        assert!(controllers.all().is_empty());
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
        let (sight_observer, sight_events) = observe(&window, ns_string!("ArdiaWindowSeenDidChange"));
        window.orderOut(None);
        controllers.reconsider_sight(&window);
        assert!(!controllers.user_sees(&window));
        let events = sight_events.get();
        window.orderFrontRegardless();
        wait_for("No visible sight reply and notification", || {
            controllers.user_sees(&window) && sight_events.get() > events
        });
        let placement = Placement {
            center: Some(NSPoint::new(50., 80.)),
            trailing: false,
        };
        lights.set_placement(placement);
        let close = window
            .standardWindowButton(NSWindowButton::CloseButton)
            .unwrap();
        let frame = ria_host_mac::windows::ardia_traffic_lights::frame_view(&window).unwrap();
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

        let (enter_observer, enter_events) =
            observe(&window, unsafe { NSWindowDidEnterFullScreenNotification });
        let (exit_observer, exit_events) =
            observe(&window, unsafe { NSWindowDidExitFullScreenNotification });
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
        normal.fullscreen.toggle();
        assert_eq!(window.frame(), screen_frame);
        normal.fullscreen.act_window_full_screen(true);
        assert_eq!((enter_events.get(), exit_events.get()), (1, 0));
        assert_eq!(window.frame(), screen_frame);
        assert!(normal.core.acted_full_screen() && normal.fullscreen.is_full_screen());
        assert!(!normal.fullscreen.transitioning());
        normal.fullscreen.toggle();
        normal.fullscreen.act_window_full_screen(false);
        assert_eq!((enter_events.get(), exit_events.get()), (1, 1));
        assert!(!normal.core.acted_full_screen() && !normal.fullscreen.is_full_screen());
        assert_eq!(window.frame(), before);
        assert!(!normal.fullscreen.transitioning());
        let center = NSNotificationCenter::defaultCenter();
        for observer in [sight_observer, enter_observer, exit_observer] {
            unsafe { center.removeObserver(&observer) };
        }
        check_shortcut(&normal.core, profile, mtm);
        for controller in [&normal, &chrome, &stray, &standalone] {
            controller.core.close();
        }
        check_native_close(mtm);
        println!(
            "live_windows: engine shortcut, controller claims, titlebar hit testing, visibility, acted fullscreen, and native close checked"
        );
        println!(
            "live_windows: native sharing after trailing placement without resize not checked (indicator and child pill)"
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
                        main_queue::post(Box::new(|| {
                            let _: () = Engine::shared().unwrap().quit();
                        }));
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
            is_reserved_key_event(MainThreadMarker::new().unwrap(), event, KeyContext::Windowed)
        }
        fn window_for_new_browser(&self, _: &Profile, _: &str) -> Option<Retained<Window>> {
            None
        }
    }
}
