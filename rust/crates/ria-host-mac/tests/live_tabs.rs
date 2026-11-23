// Ria original, no Ardia source.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("live_tabs: skipped, the prebuilt engine runs on macOS only");
}

#[cfg(target_os = "macos")]
fn main() {
    mac::run();
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::{Cell, RefCell};
    use std::ffi::CString;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{Duration, Instant};

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject};
    use objc2::{
        AllocAnyThread, DefinedClass, MainThreadMarker, Message, class, define_class, msg_send,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use ria_engine::call::main_queue;
    use ria_engine::types::{Number, TabStripCommand, TabStripTransaction};
    use ria_host_mac::core::bindings::{Engine, Profile, Tab, Window};
    use ria_host_mac::core::convert::{from_json, to_json};
    use ria_host_mac::tabs::ardia_page_draw_wait::after_page_draw;
    use ria_host_mac::tabs::ardia_renderer_scroll_observer::RendererScrollObserver;
    use ria_host_mac::tabs::ardia_tab_parking::TabParking;
    use ria_host_mac::tabs::ardia_tab_registry::TabRegistry;
    use ria_host_mac::tabs::ardia_tab_strip::{StripCommandResult, StripWindow, TabStrips};
    use ria_host_mac::tabs::ardia_web_view_context_menu::{
        ContextMenuDelegate, ElementPick, MenuEvent,
    };
    use serde_json::{Value, json};

    struct Checks {
        passed: Cell<bool>,
        shutdown: Cell<bool>,
    }

    thread_local! {
        static RESULT: Cell<bool> = const { Cell::new(false) };
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "RiaTabsLiveEngineDelegate"]
        #[ivars = Checks]
        struct Delegate;

        impl Delegate {
            #[unsafe(method(engineDidStart))]
            fn started(&self) {
                self.ivars().passed.set(true);
                main_queue::post(Box::new(start));
            }

            #[unsafe(method(engineWillShutDown))]
            fn shutdown(&self) {
                self.ivars().shutdown.set(true);
            }
        }
    );

    pub fn run() {
        if let Some(profile) = std::env::var_os("RIA_TABS_LIVE_PROFILE") {
            child(Path::new(&profile));
        } else {
            parent();
        }
    }

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn parent() {
        let engine = PathBuf::from(
            std::env::var_os("ENGINE_DIR").expect("set ENGINE_DIR to the prebuilt engine folder"),
        );
        let root =
            Scratch(std::env::temp_dir().join(format!("ria-live-tabs-{}", std::process::id())));
        std::fs::create_dir(&root.0).unwrap();
        let contents = root.0.join("RiaTabs.app/Contents");
        let macos = contents.join("MacOS");
        let frameworks = contents.join("Frameworks");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::create_dir(&frameworks).unwrap();
        std::fs::write(
            contents.join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>live_tabs</string>
<key>CFBundleIdentifier</key><string>sh.ria.tabs-live</string>
<key>CFBundleName</key><string>Ria Tabs Live</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>0</string>
<key>NSPrincipalClass</key><string>BrowserCrApplication</string>
<key>LSBackgroundOnly</key><true/>
</dict></plist>"#,
        )
        .unwrap();
        let exe = macos.join("live_tabs");
        std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        assert!(
            Command::new("cp")
                .arg("-cR")
                .arg(engine.join("Chromium Framework.framework"))
                .arg(&frameworks)
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
        let profile = root.0.join("profile");
        std::fs::create_dir(&profile).unwrap();
        let mut child = Command::new(exe)
            .env("RIA_TABS_LIVE_PROFILE", profile)
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "live_tabs child failed: {status}");
                break;
            }
            if start.elapsed() > Duration::from_secs(50) {
                let _ = child.kill();
                let _ = child.wait();
                panic!("live_tabs child timed out");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn child(profile: &Path) {
        let executable = std::env::current_exe().unwrap();
        let contents = executable.parent().and_then(Path::parent).unwrap();
        let library = contents
            .join("Frameworks/Chromium Framework.framework/Versions/Current/Chromium Framework");
        let library = CString::new(library.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: This loads the prebuilt engine before we use its Objective-C classes.
        assert!(
            !unsafe { libc::dlopen(library.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) }
                .is_null()
        );
        let args = [
            executable.to_string_lossy().into_owned(),
            format!("--user-data-dir={}", profile.display()),
            "--use-mock-keychain".into(),
            "--profile-directory=Default".into(),
            "--no-first-run".into(),
            "--disable-stack-profiler".into(),
        ]
        .map(|arg| CString::new(arg).unwrap());
        let pointers = args
            .iter()
            .map(|arg| arg.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect::<Vec<_>>();
        let delegate = Delegate::alloc().set_ivars(Checks {
            passed: Cell::new(false),
            shutdown: Cell::new(false),
        });
        // SAFETY: NSObject initializes the allocated delegate; argv and the delegate live until the engine quits.
        let delegate: Retained<Delegate> = unsafe { msg_send![super(delegate), init] };
        let code = unsafe { Engine::run(args.len() as i32, pointers.as_ptr(), &delegate) };
        assert_eq!(code, 0);
        assert!(
            delegate.ivars().passed.get(),
            "engineDidStart did not arrive"
        );
        assert!(
            delegate.ivars().shutdown.get(),
            "engineWillShutDown did not arrive"
        );
        assert!(RESULT.with(Cell::get), "a live tab check failed");
    }

    fn start() {
        let Some(engine) = Engine::shared() else {
            return;
        };
        let profile = engine.default_profile();
        if profile.path().to_string().is_empty() {
            engine.load_profile(&NSString::from_str("Default"), move |profile| {
                if let Some(profile) = profile {
                    main_queue::post(Box::new(move || open_fixture(&profile)));
                }
            });
        } else {
            open_fixture(&profile);
        }
    }

    fn open_fixture(profile: &Profile) {
        let rect = NSRect::new(NSPoint::new(-3000.0, -3000.0), NSSize::new(420.0, 300.0));
        // SAFETY: The engine registered this class and this call is on the main thread.
        let window = unsafe {
            Window::init_with_content_rect(msg_send![class!(ArdiaCoreWindow), alloc], rect)
        };
        window.set_internal(true);
        window.prepare_profile(profile);
        let tab = window.open_tab(
            &NSString::from_str("data:text/html,<title>Ria%20tab%20fixture</title>"),
            profile,
            false,
        );
        window.activate_tab(&tab);
        loaded(
            window,
            tab,
            profile.retain(),
            Instant::now() + Duration::from_secs(30),
        );
    }

    fn loaded(
        window: Retained<Window>,
        tab: Retained<Tab>,
        profile: Retained<Profile>,
        deadline: Instant,
    ) {
        if tab.title().to_string() != "Ria tab fixture" {
            if Instant::now() >= deadline {
                shutdown(&window, &tab, false);
                return;
            }
            main_queue::after(
                Duration::from_millis(50),
                Box::new(move || loaded(window, tab, profile, deadline)),
            );
            return;
        }
        let mtm = MainThreadMarker::new().unwrap();
        let view = tab.view();
        let mut registry = TabRegistry::default();
        registry.set_view(&tab, Some((&view, "fixture".into())));
        let mut pass = registry
            .tab_with_app_id("fixture")
            .is_some_and(|found| std::ptr::eq(&*found, &*tab));
        let id = registry.offer(tab.clone(), "fixture", Instant::now());
        pass &= registry
            .take_offered(&id)
            .is_some_and(|found| std::ptr::eq(&*found, &*tab));
        registry.expect_first_show_focus(&tab, true);
        pass &= registry.take_first_show_focus(&tab) && !registry.take_first_show_focus(&tab);
        registry.hold_until_hosted(&tab, "test".into(), json!({"id":"held"}));
        pass &= registry.release_held_events(&tab)
            == [("test".into(), json!({"id":"held", "tabId":"fixture"}))];

        let mut parking = TabParking::new(mtm);
        parking.park("fixture".into(), tab.clone());
        pass &= parking
            .parked_tab("fixture")
            .is_some_and(|found| std::ptr::eq(found, &*tab));
        pass &= parking.unpark("fixture").is_some();
        window.host_view().addSubview(&view);

        let scopes = [StripWindow {
            window: &window,
            profile: &profile,
            profile_name: "Default",
            app_window: Some("fixture"),
            host_changes: false,
            standalone: false,
        }];
        let mut strips = TabStrips::default();
        let strip = Number::from(window.chrome_window_id_for_profile(&profile));
        let result = strips.command(
            Number(7.0),
            TabStripCommand::Arrange {
                strip,
                keys: vec!["fixture".into()],
                pinned: Number(1.0),
            },
            &scopes,
            &mut registry,
        );
        pass &= match result {
            StripCommandResult::Done(tx) => serde_json::to_value(&tx)
                .ok()
                .and_then(|value| serde_json::from_value::<TabStripTransaction>(value).ok())
                .is_some_and(|parsed| {
                    parsed.cmd == Some(Number(7.0))
                        && parsed
                            .strips
                            .first()
                            .and_then(|strip| strip.tabs.first())
                            .is_some_and(|tab| tab.pinned && tab.key.as_deref() == Some("fixture"))
                }),
            StripCommandResult::Group { .. } => false,
        };

        let urls = ["about:blank#dismissed".into(), "about:blank#adopt".into()];
        parking.hold_link_tabs(&urls, "Default", Instant::now(), |url| {
            Some(window.open_tab(&NSString::from_str(url), &profile, false))
        });
        if let Some(dismissed) = parking.first_link_tab().map(Message::retain) {
            parking.dismiss_link_tab(&dismissed, |_| {});
            let route = parking.route_link_tabs(&urls);
            pass &= route.profile == Some("Default")
                && route.dismissed == [0]
                && route.adopt_ids == [None, Some("link:1".into())];
            if let Some(adopted) = parking.park_link_tab("link:1", "adopted") {
                parking.discard_tab(&adopted, "adopted", |_| {});
            } else {
                pass = false;
            }
        } else {
            pass = false;
        }

        let mut pick = ElementPick::default();
        let armed = pick.arm(&tab, 1);
        pass &= !pick.take(&tab, 1, armed);
        let armed = pick.arm(&tab, 1);
        pick.note_input(&tab);
        pass &= !pick.take(&tab, 2, armed);
        let armed = pick.arm(&tab, 2);
        pick.note_input(&tab);
        pass &= pick.take(&tab, 2, armed) && !pick.take(&tab, 2, armed);

        let observer = RendererScrollObserver::install(mtm, |_| {});
        let _ = observer.renderer_scroll_for(&view);
        let events = std::rc::Rc::new(RefCell::new(Vec::new()));
        let delegate = ContextMenuDelegate::new(mtm, {
            let events = events.clone();
            move |event| {
                if let MenuEvent::Command(value) = event {
                    events.borrow_mut().push(value);
                }
            }
        });
        delegate.set_test_pick("Copy\t8".into());
        let menu = from_json(&json!({"items": [{"label": "Copy", "id": 42}]}));
        // SAFETY: The delegate implements these exact selectors and all arguments are live Foundation objects.
        let (selection, picked) = unsafe {
            let text = NSString::from_str(" fixture   text ");
            let items: Retained<AnyObject> =
                msg_send![&*delegate, tab: &*tab, contextMenuItemsForSelection: &*text];
            let search = NSString::from_str("search");
            let _: () = msg_send![&*delegate, tab: &*tab, contextMenuCommand: &*search, text: &*text, modifiers: Option::<&AnyObject>::None];
            let picked: Option<Retained<AnyObject>> =
                msg_send![&*delegate, tab: &*tab, runContextMenu: &*menu];
            let second: Option<Retained<AnyObject>> =
                msg_send![&*delegate, tab: &*tab, runContextMenu: &*menu];
            pass &= second.is_none() && events.borrow().is_empty();
            (to_json(&items), picked.map(|picked| to_json(&picked)))
        };
        pass &= selection[0]["title"].as_str() == Some("Search Google for “fixture text”");
        pass &= picked == Some(json!({"command": 42, "path": [0], "flags": 8}));
        after_page_draw(
            &tab.clone(),
            |_, _| true,
            move || {
                let pass = pass
                    && events
                        .borrow()
                        .first()
                        .and_then(|v: &Value| v["command"].as_str())
                        == Some("search");
                drop(observer);
                shutdown(&window, &tab, pass);
            },
        );
    }

    fn shutdown(window: &Window, tab: &Tab, pass: bool) {
        println!("live_tabs: title, registry, parking, typed strip and context menu: {pass}");
        RESULT.with(|result| result.set(pass));
        tab.close();
        window.close();
        if let Some(engine) = Engine::shared() {
            engine.quit();
        }
    }
}
