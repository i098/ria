// Ria original, no Ardia source.

// This fixture does not check mouse or keyboard input delivery.

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
    use std::rc::Rc;
    use std::time::{Duration, Instant, SystemTime};

    use objc2::rc::Retained;
    use objc2::runtime::NSObject;
    use objc2::{
        AllocAnyThread, DefinedClass, MainThreadMarker, Message, class, define_class, msg_send,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use ria_engine::call::{ActionQueue, Engine as CallEngine, main_queue};
    use ria_engine::types::{Number, TabStripCommand, TabStripTransaction};
    use ria_host_mac::core::bindings::{Adoption, Engine, Profile, Tab, Window};
    use ria_host_mac::tabs::ardia_tab_parking::TabParking;
    use ria_host_mac::tabs::ardia_tab_registry::TabRegistry;
    use ria_host_mac::tabs::ardia_tab_strip::{StripCommandResult, StripWindow, TabStrips};
    use ria_host_mac::tabs::ardia_web_view_navigation::NavigationReports;
    use ria_host_mac::tabs::ardia_web_view_page_commands as page_commands;
    use ria_host_mac::tabs::ardia_web_view_recording::recording_shows_page;
    use ria_host_mac::tabs::ardia_web_view_site_data::site_data;
    use ria_host_mac::tabs::ardia_web_view_tab::{Attach, AttachStep, AttachedTab, StashedReports};
    use serde_json::json;

    const FIXTURE_URL: &str = "data:text/html,%3Ctitle%3ERia%20tab%20fixture%3C/title%3E";
    const FIXTURE_TITLE: &str = "Ria tab fixture";

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

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "RiaTabsLiveRemovalObserver"]
        #[ivars = Cell<*const Tab>]
        struct RemovalObserver;

        impl RemovalObserver {
            #[unsafe(method(window:didRemoveTab:))]
            fn removed(&self, _window: &Window, tab: &Tab) {
                self.ivars().set(std::ptr::from_ref(tab));
            }
        }
    );

    #[derive(Default)]
    struct NavigationChecks {
        reports: RefCell<NavigationReports>,
        matched: Cell<bool>,
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "RiaTabsLiveNavigationObserver"]
        #[ivars = NavigationChecks]
        struct NavigationObserver;

        impl NavigationObserver {
            #[unsafe(method(tabDidChangeTitle:))]
            fn title_changed(&self, tab: &Tab) {
                self.report(tab);
            }

            #[unsafe(method(tabDidChangeLoading:))]
            fn loading_changed(&self, tab: &Tab) {
                self.report(tab);
            }
        }
    );

    impl NavigationObserver {
        fn report(&self, tab: &Tab) {
            let checks = self.ivars();
            checks.reports.borrow_mut().emit(
                tab,
                None,
                false,
                &mut false,
                SystemTime::now(),
                |event| {
                    if event["url"] == FIXTURE_URL
                        && event["title"] == FIXTURE_TITLE
                        && event["isLoading"] == false
                    {
                        checks.matched.set(true);
                    }
                },
            );
        }
    }

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

    fn fixture_window(profile: &Profile) -> Retained<Window> {
        let rect = NSRect::new(NSPoint::new(-3000.0, -3000.0), NSSize::new(420.0, 300.0));
        // SAFETY: The engine registered this class and this call is on the main thread.
        let window = unsafe {
            Window::init_with_content_rect(msg_send![class!(ArdiaCoreWindow), alloc], rect)
        };
        window.set_internal(true);
        window.prepare_profile(profile);
        window
    }

    fn open_fixture(profile: &Profile) {
        let window = fixture_window(profile);
        let tab = window.open_tab(&NSString::from_str("about:blank"), profile, false);
        let observer = NavigationObserver::alloc().set_ivars(NavigationChecks::default());
        // SAFETY: The observer implements the tab callbacks and stays alive until we clear the delegate.
        let observer: Retained<NavigationObserver> = unsafe { msg_send![super(observer), init] };
        unsafe { tab.set_delegate(Some(&observer)) };
        window.activate_tab(&tab);
        window
            .host_view()
            .addSubview(&tab.view().expect("open tab has a view"));
        let deadline = Instant::now() + Duration::from_secs(30);
        tab.load_url(&NSString::from_str(FIXTURE_URL), true);
        let profile = profile.retain();
        wait_for_event(
            deadline,
            move || observer.ivars().matched.get(),
            move |observed| {
                unsafe { tab.set_delegate(None) };
                if !observed {
                    eprintln!("live_tabs: no matching navigation event");
                    return shutdown(&window, &tab, false);
                }
                check_evaluation(window, tab, profile, deadline);
            },
        );
    }

    fn check_evaluation(
        window: Retained<Window>,
        tab: Retained<Tab>,
        profile: Retained<Profile>,
        deadline: Instant,
    ) {
        let result = Rc::new(Cell::new(None));
        page_commands::evaluate(
            Some(&tab),
            "post('result', JSON.stringify(6 * 7))",
            false,
            tab.frozen(),
            {
                let result = result.clone();
                move |reply| {
                    let number = reply
                        .as_deref()
                        .and_then(|reply| serde_json::from_str::<serde_json::Number>(reply).ok());
                    result.set(Some(
                        number.and_then(|number| number.as_f64()) == Some(42.0),
                    ));
                }
            },
        );
        wait_for_event(
            deadline,
            {
                let result = result.clone();
                move || result.get().is_some()
            },
            move |answered| {
                if !answered || result.get() != Some(true) {
                    eprintln!("live_tabs: evaluation did not return JSON number 42");
                    return shutdown(&window, &tab, false);
                }
                check_tab_services(window, tab, profile, deadline);
            },
        );
    }

    fn check_tab_services(
        window: Retained<Window>,
        tab: Retained<Tab>,
        profile: Retained<Profile>,
        deadline: Instant,
    ) {
        let host = window.host_view();
        let registry = RefCell::new(TabRegistry::default());
        let mut stashed = StashedReports::default();
        let mut attached = AttachedTab::<()>::default();
        let mut ready = false;
        let adoption = attached.attach(
            tab.clone(),
            Attach {
                host: &host,
                app_id: "tab-services".into(),
                window: Some(&window),
                placement: None,
                shown_early: true,
                created: false,
                temporary: false,
                muted: false,
                page_background: None,
                contents_background: None,
                page_color: None,
            },
            &registry,
            &mut stashed,
            |_, _| {},
            |step| ready |= matches!(step, AttachStep::Ready),
        );
        let was_registered = registry.borrow().app_tab_id(&tab) == Some("tab-services");
        let page = tab.view().expect("open tab has a view");
        page.setHidden(false);
        page.setAlphaValue(1.0);
        let records_visible = recording_shows_page(&host, Some(&tab));
        page.setHidden(true);
        let skips_hidden = !recording_shows_page(&host, Some(&tab));
        page.setHidden(false);
        attached.life.keeps_page_on_detach = true;
        let detached = attached
            .detach(&host, &registry, &mut stashed, |_, _| {}, |_| {})
            .is_some_and(|detached| std::ptr::eq(&*detached, &*tab));
        if !(adoption == Adoption::Adopted
            && ready
            && was_registered
            && records_visible
            && skips_hidden
            && detached
            && attached.tab.is_none()
            && registry.borrow().app_tab_id(&tab).is_none()
            && recording_shows_page(&host, Some(&tab)))
        {
            eprintln!("live_tabs: attach, recording, or retained-view detach check failed");
            return shutdown(&window, &tab, false);
        }
        let reply = Rc::new(Cell::new(None));
        site_data(
            Some(&tab),
            Some(&profile),
            &json!({"op": "read", "domains": [], "origins": []}),
            Box::new({
                let reply = reply.clone();
                move |value| reply.set(Some(value == Some(json!({"cookies": [], "storage": {}}))))
            }),
        );
        wait_for_event(
            deadline,
            {
                let reply = reply.clone();
                move || reply.get().is_some()
            },
            move |answered| {
                if !answered || reply.get() != Some(true) {
                    eprintln!("live_tabs: no typed site data reply from Chromium");
                    return shutdown(&window, &tab, false);
                }
                println!("live_tabs: attach, recording visibility, and site data reply: true");
                check_registry_and_parking(window, tab, profile);
            },
        );
    }

    fn check_registry_and_parking(
        window: Retained<Window>,
        tab: Retained<Tab>,
        profile: Retained<Profile>,
    ) {
        let mtm = MainThreadMarker::new().unwrap();
        let view = tab.view().expect("open tab has a view");
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
        let host = window.host_view();
        let host_window = window.window();
        pass &= view
            .window()
            .is_some_and(|attached| std::ptr::eq(&*attached, &*host_window));
        let mut parking = TabParking::new(mtm);
        registry.set_view(&tab, None);
        parking.park("fixture".into(), tab.clone());
        pass &= view.window().is_none() && registry.app_tab_id(&tab).is_none();
        pass &= parking
            .parked_tab("fixture")
            .is_some_and(|parked| std::ptr::eq(parked, &*tab));
        registry.hold_until_hosted(&tab, "test".into(), json!({"id":"held"}));
        pass &= registry.release_held_events(&tab).is_empty();
        let Some(restored) = parking.unpark("fixture") else {
            return shutdown(&window, &tab, false);
        };
        pass &= parking.parked_tab("fixture").is_none();
        let restored_view = restored.view().expect("restored tab has a view");
        pass &= std::ptr::eq(&*restored, &*tab) && std::ptr::eq(&*restored_view, &*view);
        host.addSubview(&restored_view);
        pass &= restored_view
            .window()
            .is_some_and(|attached| std::ptr::eq(&*attached, &*host_window));
        registry.set_view(&restored, Some((&restored_view, "fixture".into())));
        pass &= registry.release_held_events(&restored)
            == [("test".into(), json!({"id":"held", "tabId":"fixture"}))];
        pass &= registry.release_held_events(&restored).is_empty();

        let urls = ["about:blank#dismissed".into(), "about:blank#adopt".into()];
        parking.hold_link_tabs(&urls, "Default", Instant::now(), |url| {
            Some(window.open_tab(&NSString::from_str(url), &profile, false))
        });
        if let Some(dismissed) = parking.first_link_tab().map(Message::retain) {
            parking.dismiss_link_tab(&dismissed, |_| {});
            pass &= !parking.take_dismissed_link_mark("link:+0");
            pass &= !parking.take_dismissed_link_mark("link:00");
            pass &= parking.take_dismissed_link_mark("link:0");
            let route = parking.route_link_tabs(&urls);
            pass &= serde_json::to_value(route).unwrap()
                == json!({"profile": "Default", "dismissed": [0], "adoptIds": [null, "link:1"]});
            pass &= serde_json::to_value(parking.route_link_tabs(&urls)).unwrap() == json!({});
            pass &= parking.park_link_tab("link:+1", "alias").is_none();
            pass &= parking.park_link_tab("link:01", "alias").is_none();
            if let Some(adopted) = parking.park_link_tab("link:1", "adopted") {
                parking.discard_tab(&adopted, "adopted", |_| {});
            } else {
                pass = false;
            }
        } else {
            pass = false;
        }

        let mut parking = TabParking::new(mtm);
        parking.hold_link_tabs(&urls, "Default", Instant::now(), |url| {
            Some(window.open_tab(&NSString::from_str(url), &profile, false))
        });
        if let Some(dismissed) = parking.first_link_tab().map(Message::retain) {
            parking.dismiss_link_tab(&dismissed, |_| {});
        }
        pass &= serde_json::to_value(parking.route_link_tabs(&urls[..1])).unwrap()
            == json!({"dismissed": [0]});
        pass &= serde_json::to_value(parking.route_link_tabs(&urls[1..])).unwrap()
            == json!({"profile": "Default", "adoptIds": ["link:1"], "dismissed": []});
        if let Some(adopted) = parking.park_link_tab("link:1", "adopted") {
            parking.discard_tab(&adopted, "adopted", |_| {});
        } else {
            pass = false;
        }

        check_strip_events(window, restored, profile, registry, pass);
    }

    fn check_strip_events(
        window: Retained<Window>,
        tab: Retained<Tab>,
        profile: Retained<Profile>,
        mut registry: TabRegistry,
        pass: bool,
    ) {
        let scopes = [fixture_strip(&window, &profile)];
        let mut strips = TabStrips::default();
        let strip_id = window.chrome_window_id_for_profile(&profile);
        let strip = Number::from(strip_id);
        let command = Number(8.0);

        let StripCommandResult::Group(args) = strips.command(
            command,
            TabStripCommand::Group {
                strip,
                keys: vec!["fixture".into()],
                group: Some("new".into()),
                title: Some("Ria fixture group".into()),
                color: Some(json!("blue")),
                collapsed: Some(false),
            },
            &scopes,
            &mut registry,
        ) else {
            return shutdown(&window, &tab, false);
        };
        let queue = ActionQueue::new();
        let weak = Rc::downgrade(&queue);
        queue.set_waker(move || {
            let weak = weak.clone();
            main_queue::post(Box::new(move || _ = weak.upgrade().map(|q| q.drain())));
        });
        let call = CallEngine::new(queue);
        call.did_start();
        let emitted = Rc::new(RefCell::new(Vec::new()));
        let engine_pinned = Rc::new(Cell::new(None));
        let event_deadline = Rc::new(Cell::new(None::<Instant>));
        let tab_id = i64::from(tab.tab_id());
        let profile_path = profile.path().to_string();
        let state = Rc::new(RefCell::new((strips, registry)));
        call.on_event("tabs.strip", {
            let emitted = emitted.clone();
            let engine_pinned = engine_pinned.clone();
            let event_deadline = event_deadline.clone();
            let window = window.clone();
            let state = state.clone();
            let profile = profile.clone();
            let profile_path = profile_path.clone();
            move |event| {
                if event_deadline
                    .get()
                    .is_some_and(|deadline| Instant::now() >= deadline)
                    || event["window"].as_i64() != Some(i64::from(strip_id))
                    || event["profile"].as_str() != Some(profile_path.as_str())
                    || event["offTheRecord"].as_bool() == Some(true)
                {
                    return;
                }
                let Some(row) = event["tabs"]
                    .as_array()
                    .and_then(|rows| rows.iter().find(|row| row["tab"].as_i64() == Some(tab_id)))
                else {
                    return;
                };
                engine_pinned.set(row["pinned"].as_bool());
                let (strips, registry) = &mut *state.borrow_mut();
                let scope = fixture_strip(&window, &profile);
                if strips.observe_engine_strip(event) == Some(strip_id)
                    && let Some(tx) = strips.changed(&scope, registry, None)
                {
                    emitted.borrow_mut().push(tx);
                }
            }
        });
        call.call("ac_tabs_watch", &profile_path, false, json!({}), {
            let call = call.clone();
            let profile_path = profile_path.clone();
            move |watch| {
                if !watch.is_ok_and(|reply| reply["ok"] == true) {
                    call.shutdown();
                    return shutdown(&window, &tab, false);
                }
                call.call("ac_tabs_group", &profile_path, false, args, {
                    let call = call.clone();
                    move |reply| {
                        let group = reply
                            .ok()
                            .and_then(|reply| reply["group"].as_str().map(str::to_owned))
                            .filter(|group| !group.is_empty());
                        let Some(group) = group.filter(|_| pass) else {
                            call.shutdown();
                            return shutdown(&window, &tab, false);
                        };
                        let deadline = Instant::now() + Duration::from_secs(10);
                        event_deadline.set(Some(deadline));
                        wait_for_event(
                            deadline,
                            {
                                let group = group.clone();
                                move || {
                                    emitted
                                        .borrow()
                                        .iter()
                                        .any(|tx| matches_group(tx, None, strip, &group))
                                }
                            },
                            move |observed| {
                                let scope = fixture_strip(&window, &profile);
                                let pass = observed && {
                                    let (strips, registry) = &mut *state.borrow_mut();
                                    let tx =
                                        strips.group_finished(command, &scope, registry, false);
                                    matches_group(&tx, Some(command), strip, &group)
                                };
                                if !pass {
                                    call.shutdown();
                                    return shutdown(&window, &tab, false);
                                }
                                window.open_tab(
                                    &NSString::from_str("about:blank"),
                                    &profile,
                                    false,
                                );
                                check_engine_pins(
                                    deadline,
                                    engine_pinned,
                                    &[(9.0, 1.0, true), (10.0, 0.5, false), (11.0, 1.0, true)],
                                    {
                                        let window = window.clone();
                                        let profile = profile.clone();
                                        let state = state.clone();
                                        move |command, pinned, expected| {
                                            let (strips, registry) = &mut *state.borrow_mut();
                                            check_arrange(
                                                &fixture_strip(&window, &profile),
                                                strips,
                                                registry,
                                                command,
                                                pinned,
                                                expected,
                                            )
                                        }
                                    },
                                    move |pass| {
                                        let pass = pass && {
                                            let (strips, registry) = &mut *state.borrow_mut();
                                            check_adoption(
                                                &fixture_strip(&window, &profile),
                                                &tab,
                                                strips,
                                                registry,
                                            )
                                        };
                                        call.shutdown();
                                        shutdown(&window, &tab, pass);
                                    },
                                );
                            },
                        );
                    }
                });
            }
        });
    }

    fn fixture_strip<'a>(window: &'a Window, profile: &'a Profile) -> StripWindow<'a> {
        StripWindow {
            window,
            profile,
            profile_name: "Default",
            app_window: Some("fixture"),
            host_changes: false,
            standalone: false,
        }
    }

    fn check_engine_pins(
        deadline: Instant,
        engine_pinned: Rc<Cell<Option<bool>>>,
        steps: &'static [(f64, f64, bool)],
        arrange: impl Fn(Number, Number, bool) -> bool + 'static,
        finish: impl FnOnce(bool) + 'static,
    ) {
        let Some((&(command, pinned, expected), remaining)) = steps.split_first() else {
            return finish(true);
        };
        engine_pinned.set(None);
        if !arrange(Number(command), Number(pinned), expected) {
            return finish(false);
        }
        wait_for_event(
            deadline,
            {
                let engine_pinned = engine_pinned.clone();
                move || engine_pinned.get() == Some(expected)
            },
            move |observed| {
                if observed {
                    check_engine_pins(deadline, engine_pinned, remaining, arrange, finish);
                } else {
                    finish(false);
                }
            },
        );
    }

    fn wait_for_event(
        deadline: Instant,
        matches: impl Fn() -> bool + 'static,
        finish: impl FnOnce(bool) + 'static,
    ) {
        if matches() {
            finish(true);
        } else if Instant::now() >= deadline {
            finish(false);
        } else {
            main_queue::after(
                Duration::from_millis(20),
                Box::new(move || wait_for_event(deadline, matches, finish)),
            );
        }
    }

    fn check_arrange(
        source: &StripWindow<'_>,
        strips: &mut TabStrips,
        registry: &mut TabRegistry,
        command: Number,
        pinned: Number,
        expected: bool,
    ) -> bool {
        let strip = Number::from(source.window.chrome_window_id_for_profile(source.profile));
        let StripCommandResult::Done(arranged) = strips.command(
            command,
            TabStripCommand::Arrange {
                strip,
                keys: vec!["fixture".into()],
                pinned,
            },
            std::slice::from_ref(source),
            registry,
        ) else {
            return false;
        };
        arranged.cmd == Some(command)
            && arranged.rejected.is_none()
            && arranged.strips.iter().any(|state| {
                state.strip == strip
                    && state.tabs.iter().any(|row| {
                        row.key.as_deref() == Some("fixture")
                            && row.index == Number(0.0)
                            && row.pinned == expected
                    })
            })
    }

    fn check_adoption(
        source: &StripWindow<'_>,
        tab: &Tab,
        strips: &mut TabStrips,
        registry: &mut TabRegistry,
    ) -> bool {
        let strip_id = source.window.chrome_window_id_for_profile(source.profile);
        let destination = fixture_window(source.profile);
        strips.activated(strip_id, tab);
        let observer = RemovalObserver::alloc().set_ivars(Cell::new(std::ptr::null()));
        let observer: Retained<RemovalObserver> = unsafe { msg_send![super(observer), init] };
        unsafe { source.window.set_delegate(Some(&observer)) };
        let adopted = destination.adopt_tab(tab);
        unsafe { source.window.set_delegate(None) };
        let removed = observer.ivars().get() == std::ptr::from_ref(tab);
        if removed {
            strips.removed(strip_id, tab);
        }
        let target = StripWindow {
            window: &destination,
            profile: source.profile,
            profile_name: source.profile_name,
            app_window: Some("destination"),
            host_changes: false,
            standalone: false,
        };
        let pass = adopted == Adoption::Adopted
            && removed
            && strips.active(strip_id).is_none()
            && strips.changed(&target, registry, None).is_some_and(|tx| {
                tx.strips[0]
                    .tabs
                    .iter()
                    .any(|row| row.key.as_deref() == Some("fixture") && row.pinned)
            });
        destination.close();
        pass
    }

    fn matches_group(
        tx: &TabStripTransaction,
        command: Option<Number>,
        strip: Number,
        group: &str,
    ) -> bool {
        let [state] = tx.strips.as_slice() else {
            return false;
        };
        tx.cmd == command
            && tx.rejected.is_none()
            && state.strip == strip
            && state.app_window.as_deref() == Some("fixture")
            && state.tabs.iter().any(|row| {
                row.key.as_deref() == Some("fixture")
                    && row.group.as_deref() == Some(group)
                    && !row.pinned
            })
            && state.groups.as_deref().is_some_and(|groups| {
                groups.iter().any(|row| {
                    row.id == group
                        && row.title == "Ria fixture group"
                        && row.color == "blue"
                        && !row.collapsed
                })
            })
    }

    fn shutdown(window: &Window, tab: &Tab, pass: bool) {
        println!("live_tabs: page controls, registry, parking, and strip events: {pass}");
        RESULT.with(|result| result.set(pass));
        tab.close();
        window.close();
        if let Some(engine) = Engine::shared() {
            engine.quit();
        }
    }
}
