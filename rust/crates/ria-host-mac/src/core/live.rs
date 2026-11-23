// Ria original, no Ardia source.

//! Check L of slice 3.5 (it runs as `cargo test -p ria-host-mac --test live`; the target has its path here, because
//! this folder is the only one that the slice owns for tests).
//!
//! Needs a Mac and `ENGINE_DIR=<dir that holds "Chromium Framework.framework">`. The engine owns the main thread,
//! so this test has its own `main`. It wraps itself in an app bundle beside a copy of the framework, starts the engine
//! with a temporary profile and the delegate of this crate, and then:
//! - reads each class of `bindings.rs` that works without a window (engine, profile, window, tab);
//! - makes each call of `services.rs` that has no side effect outside the temporary profile;
//! - sends each callback of the delegate by Objective-C dispatch and checks the JSON that reaches the hooks.
//! Each reply must parse and each event that a call causes must arrive.

#![allow(unsafe_code)]

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("live: skipped, the prebuilt engine runs on macOS only");
}

#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("RIA_LIVE_CHILD").is_some() {
        mac::child()
    } else {
        mac::parent()
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::{Cell, RefCell};
    use std::ffi::CString;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;
    use std::time::Duration;

        use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_app_kit::NSEvent;
    use objc2_foundation::{NSRect, NSString};
    use ria_engine::call::{ActionQueue, Engine as CallEngine, main_queue};
    use ria_host_mac::core::bindings::{Engine, Profile, Tab, Window};
    use ria_host_mac::core::convert::from_json;
    use ria_host_mac::core::delegate::{EngineDelegate, Hooks};
    use ria_host_mac::core::host::{self, State};
    use ria_host_mac::core::services::{Reply, ServiceHost, Services};
    use serde_json::{Value, json};

    const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>live</string>
<key>CFBundleIdentifier</key><string>sh.ria.host-live</string>
<key>CFBundleName</key><string>Ria Host Live</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>0</string>
<key>NSPrincipalClass</key><string>BrowserCrApplication</string>
</dict></plist>"#;

    const PAGE_SCRIPT: &str = "/* live test page script */";

    /// The process that cargo starts: builds the bundle, runs the child in it, forwards the exit code.
    pub fn parent() {
        let engine_dir = PathBuf::from(
            std::env::var_os("ENGINE_DIR").expect("set ENGINE_DIR to the folder that holds Chromium Framework.framework"),
        );
        let root = std::env::temp_dir().join(format!("ria-host-live-{}", std::process::id()));
        let macos = root.join("Live.app/Contents/MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::create_dir_all(root.join("Live.app/Contents/Frameworks")).unwrap();
        std::fs::write(root.join("Live.app/Contents/Info.plist"), INFO_PLIST).unwrap();
        let exe = macos.join("live");
        std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        run(Command::new("cp").arg("-cR").arg(engine_dir.join("Chromium Framework.framework")).arg(root.join("Live.app/Contents/Frameworks/")));
        run(Command::new("codesign").args(["--force", "--sign", "-"]).arg(&exe));
        let data = root.join("data");
        std::fs::create_dir_all(&data).unwrap();
        let status = Command::new(&exe)
            .env("RIA_LIVE_CHILD", "1")
            .env("RIA_LIVE_DATA", &data)
            .status()
            .expect("start the child");
        let _ = std::fs::remove_dir_all(&root);
        std::process::exit(status.code().unwrap_or(1));
    }

    fn run(command: &mut Command) {
        let status = command.status().expect("run a setup command");
        assert!(status.success(), "setup command failed: {command:?}");
    }

    /// Hooks that record what the delegate sends.
    #[derive(Default)]
    struct TestHooks {
        emitted: RefCell<Vec<(String, Value)>>,
        listening: RefCell<Vec<String>>,
        started: Cell<bool>,
        shut_down: Cell<bool>,
        reserved: RefCell<Vec<(String, String, usize)>>,
    }

    thread_local! {
        static HOOKS: Rc<TestHooks> = Rc::default();
        static LIVE: RefCell<Option<Rc<Live>>> = const { RefCell::new(None) };
    }

    impl Hooks for TestHooks {
        fn engine_started(&self) {
            self.started.set(true);
            go();
        }
        fn engine_shut_down(&self) {
            self.shut_down.set(true);
        }
        fn emit(&self, topic: &str, data: Value) {
            self.emitted.borrow_mut().push((topic.to_owned(), data));
        }
        fn listens(&self, topic: &str) -> bool {
            self.listening.borrow().iter().any(|t| t == topic)
        }
        fn app_tab_id_value(&self, _tab: Option<&Tab>) -> Value {
            Value::Null
        }
        fn profile_name(&self, _profile: &Profile) -> String {
            "Default".into()
        }
        fn names_of_loaded_profile(&self, _profile: &Profile) -> Vec<String> {
            vec!["Private 1".into(), "Private 2".into()]
        }
        fn ask_permission_in_popup_window(&self, _tab: &Tab, _request: &Value) -> bool {
            false
        }
        fn hold_until_hosted(&self, _tab: &Tab, _topic: &str, _payload: Value) {}
        fn drop_popup_window_permission(&self, _request_id: &str) {}
        fn drop_held_event(&self, _topic: &str, _request_id: &str) {}
        fn is_reserved_binding(&self, key: &str, unshifted_key: &str, modifiers: usize) -> bool {
            self.reserved.borrow_mut().push((key.into(), unshifted_key.into(), modifiers));
            key == "q"
        }
        fn is_reserved_key_event(&self, _event: &NSEvent) -> bool {
            false
        }
        fn window_for_new_browser(&self, _profile: &Profile, _kind: &str) -> Option<Retained<Window>> {
            None
        }
    }

    /// Resolves each profile name to the one profile of this test.
    struct TestHost {
        profile: Retained<Profile>,
        hooks: Rc<TestHooks>,
    }

    impl ServiceHost for TestHost {
        fn is_private(&self, _profile: &str) -> bool {
            false
        }
        fn original_name(&self, profile: &str) -> String {
            profile.to_owned()
        }
        fn with_profile(&self, _name: &str, done: Box<dyn FnOnce(Option<Retained<Profile>>)>) {
            done(Some(self.profile.clone()))
        }
        fn emit(&self, topic: &str, data: Value) {
            self.hooks.emit(topic, data)
        }
        fn manifest_for(&self, _profile: &str, _info: &Value) -> Option<Value> {
            None
        }
        fn app_tab_ids(&self, _chrome_tab_id: i64) -> Vec<Value> {
            vec![]
        }
        fn forget_early_window_pictures(&self) {}
    }

    struct Live {
        checks: RefCell<Vec<(String, bool, String)>>,
        outstanding: Cell<usize>,
        finished: Cell<bool>,
    }

    pub fn child() {
        let exe = std::env::current_exe().unwrap();
        let contents = exe.parent().and_then(Path::parent).expect("inside a bundle");
        let framework = CString::new(
            contents.join("Frameworks/Chromium Framework.framework/Versions/Current/Chromium Framework").to_str().unwrap(),
        )
        .unwrap();
        // SAFETY: loads the engine once, before any class of it is used.
        assert!(
            !unsafe { libc::dlopen(framework.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) }.is_null(),
            "cannot load the engine framework"
        );
        let data = std::env::var("RIA_LIVE_DATA").unwrap();
        let argv: Vec<CString> = [
            std::env::args().next().unwrap(),
            format!("--user-data-dir={data}/Chromium"),
            "--use-mock-keychain".into(),
            "--profile-directory=Default".into(),
            "--no-first-run".into(),
            "--disable-stack-profiler".into(),
        ]
        .iter()
        .map(|a| CString::new(a.as_str()).unwrap())
        .collect();

        let hooks = HOOKS.with(Rc::clone);
        let state = State::new();
        let delegate = EngineDelegate::new(hooks.clone(), state.clone(), Some(PAGE_SCRIPT.into()));
        // SAFETY: the framework is loaded and this is the main thread.
        let code = unsafe { delegate.run(&argv) };

        let live = LIVE.with(|l| l.borrow().clone()).expect("the engine started");
        record(&live, "engineWillShutDown arrived after quit", hooks.shut_down.get(), String::new());
        record(&live, "state is stopped after quit", !state.is_started(), String::new());
        let checks = live.checks.borrow();
        for (name, ok, detail) in checks.iter() {
            println!("{}", json!({"live": "check", "name": name, "pass": ok, "detail": detail}));
        }
        let failed = checks.iter().filter(|c| !c.1).count();
        let pass = code == 0 && failed == 0 && !checks.is_empty();
        println!("{}", json!({"live": "result", "exit_code": code, "checks": checks.len(), "failed": failed, "pass": pass}));
        std::process::exit(if pass { 0 } else { 1 });
    }

    fn record(live: &Live, name: &str, pass: bool, detail: String) {
        live.checks.borrow_mut().push((name.to_owned(), pass, detail));
    }

    /// Runs after `engineDidStart`.
    fn go() {
        let engine = Engine::shared().expect("the engine exists at engineDidStart");
        match Some(engine.default_profile()).filter(|p| !p.path().to_string().is_empty()) {
            Some(profile) => ready(engine, profile),
            None => {
                let again = engine.clone();
                engine.load_profile(&NSString::from_str("Default"), move |profile| match profile {
                    Some(p) => ready(again.clone(), p),
                    None => {
                        println!("{}", json!({"live": "error", "detail": "loadProfile returned nil"}));
                        std::process::exit(2);
                    }
                });
            }
        }
    }

    fn ready(engine: Retained<Engine>, profile: Retained<Profile>) {
        let queue = ActionQueue::new();
        let weak = Rc::downgrade(&queue);
        queue.set_waker(move || {
            let weak = weak.clone();
            main_queue::post(Box::new(move || _ = weak.upgrade().map(|q| q.drain())));
        });
        let call = CallEngine::new(queue);
        call.did_start();
        let live = Rc::new(Live { checks: RefCell::default(), outstanding: Cell::new(0), finished: Cell::new(false) });
        LIVE.with(|l| *l.borrow_mut() = Some(live.clone()));
        let hooks = HOOKS.with(Rc::clone);

        bindings(&live, &engine, &profile, &hooks);
        delegate_callbacks(&live, &engine, &hooks);

        let services = Services::new(call.clone(), Rc::new(TestHost { profile: profile.clone(), hooks: hooks.clone() }));
        services.watch_download_events();
        services_calls(&live, &services);
        window_and_tab(&live, &profile);

        let watch = live.clone();
        poll(
            Rc::new(move |waited: Duration| {
                let done = watch.outstanding.get() == 0;
                let seen = hooks.emitted.borrow().iter().any(|(t, _)| t == "downloads.changed")
                    || waited >= Duration::from_secs(20);
                if (done && seen) || waited >= Duration::from_secs(60) {
                    if !watch.finished.replace(true) {
                        record(&watch, "every reply arrived", done, format!("outstanding {}", watch.outstanding.get()));
                        let _: () = engine.quit();
                    }
                    return true;
                }
                false
            }),
            Duration::ZERO,
        );
        std::mem::forget(call);
    }

    fn poll(step: Rc<dyn Fn(Duration) -> bool>, waited: Duration) {
        if step(waited) {
            return;
        }
        let next = Duration::from_millis(100);
        main_queue::after(next, Box::new(move || poll(step, waited + next)));
    }

    /// Reads the classes that need no window.
    fn bindings(live: &Live, engine: &Engine, profile: &Profile, hooks: &TestHooks) {
        let version = engine.chromium_version().to_string();
        record(live, "Engine.chromiumVersion is a version", version.split('.').count() >= 3, version);
        record(live, "engineDidStart reached the hooks", hooks.started.get(), String::new());
        record(
            live,
            "the page script reached the engine",
            engine.page_script().map(|s| s.to_string()).as_deref() == Some(PAGE_SCRIPT),
            String::new(),
        );
        record(live, "Profile has a name and a path", !profile.name().to_string().is_empty() && Path::new(&profile.path().to_string()).is_dir(), profile.path().to_string());
        record(live, "Profile is not off the record or destroyed", !profile.off_the_record() && !profile.destroyed(), String::new());
        profile.set_passwords_offered(false);
        record(live, "Profile.passwordsOffered stores a value", !profile.passwords_offered(), String::new());
        record(live, "Engine.isTracing is false", !host::is_tracing(), String::new());
        record(live, "Engine.testExternalLaunches is a list", host::test_external_launches().is_array(), String::new());
        record(live, "Engine.testChooserEvents is a list", host::test_chooser_events().is_array(), String::new());
        record(live, "Engine.videoUpscalingState is a dictionary", Engine::video_upscaling_state().count() > 0, String::new());
        record(live, "Tab.originForURL gives an origin", Tab::origin_for_url(&NSString::from_str("https://a.example/x?y")).map(|o| o.to_string()).as_deref() == Some("https://a.example"), String::new());
        record(live, "Window.contentViewClass is a class", !Window::content_view_class().name().to_bytes().is_empty(), String::new());
    }

    /// Sends each callback of the delegate by Objective-C dispatch, as the engine does.
    fn delegate_callbacks(live: &Live, engine: &Engine, hooks: &Rc<TestHooks>) {
        let delegate = ria_delegate();
        let dict = |v: Value| from_json(&v);
        let none: Option<&AnyObject> = None;
        let emitted = |topic: &str| hooks.emitted.borrow().iter().rev().find(|(t, _)| t == topic).map(|(_, v)| v.clone());

        let request = dict(json!({"id": "r1", "origin": "https://a.example"}));
        let _: () = unsafe { msg_send![&*delegate, engine: engine, permissionRequest: &*request, tab: none] };
        record(live, "permissionRequest emits permissions.requested with a null tabId", emitted("permissions.requested") == Some(json!({"id": "r1", "origin": "https://a.example", "tabId": null})), format!("{:?}", emitted("permissions.requested")));

        let dismissed = NSString::from_str("r1");
        let _: () = unsafe { msg_send![&*delegate, engine: engine, permissionRequestDismissed: &*dismissed] };
        record(live, "permissionRequestDismissed emits permissions.dismissed", emitted("permissions.dismissed") == Some(json!({"id": "r1"})), String::new());

        // Nobody listens to `media`: a closed chooser emits nothing, and an open one is cancelled.
        let chooser = dict(json!({"id": 7, "open": true}));
        let before = hooks.emitted.borrow().len();
        let _: () = unsafe { msg_send![&*delegate, engine: engine, deviceChooser: &*chooser, tab: none] };
        record(live, "deviceChooser without a media listener emits nothing", hooks.emitted.borrow().len() == before, String::new());
        hooks.listening.borrow_mut().push("media".into());
        let _: () = unsafe { msg_send![&*delegate, engine: engine, deviceChooser: &*chooser, tab: none] };
        record(live, "deviceChooser with a media listener emits media.deviceChooser", emitted("media.deviceChooser").is_some_and(|v| v["id"] == 7), String::new());
        let dialog = dict(json!({"id": 3, "open": false}));
        let _: () = unsafe { msg_send![&*delegate, engine: engine, castDialog: &*dialog, tab: none] };
        record(live, "castDialog emits media.castDialog", emitted("media.castDialog").is_some_and(|v| v["id"] == 3), String::new());

        let routes = dict(json!([{"id": "a", "name": "b"}]));
        let profile: Retained<Profile> = engine.default_profile();
        let _: () = unsafe { msg_send![&*delegate, engine: engine, castRoutes: &*routes, profile: &*profile] };
        record(live, "castRoutes emits for the profile name", emitted("media.castRoutes") == Some(json!({"profile": "Default", "routes": [{"id": "a", "name": "b"}]})), format!("{:?}", emitted("media.castRoutes")));

        let panel = dict(json!({"extensionId": "e"}));
        let _: () = unsafe { msg_send![&*delegate, engine: engine, extensionSidePanel: &*panel, tab: none] };
        record(live, "extensionSidePanel emits extensions.sidePanel", emitted("extensions.sidePanel").is_some_and(|v| v["extensionId"] == "e"), String::new());

        let popup = dict(json!({"extensionId": "e"}));
        let handled: bool = unsafe { msg_send![&*delegate, engine: engine, extensionActionPopup: &*popup, tab: none] };
        record(live, "extensionActionPopup without a tab answers NO", !handled, String::new());
        let shortcut: bool = unsafe { msg_send![&*delegate, engine: engine, extensionActionShortcut: &*popup, tab: none] };
        record(live, "extensionActionShortcut without a tab answers NO", !shortcut, String::new());

        let prompt = dict(json!({"requestId": "p1"}));
        let _: () = unsafe { msg_send![&*delegate, engine: engine, extensionInstallPrompt: &*prompt, profile: &*profile, tab: none] };
        record(live, "extensionInstallPrompt adds the profile and the tabId", emitted("extensions.installPrompt") == Some(json!({"requestId": "p1", "tabId": null, "profile": "Default"})), String::new());
        let message = NSString::from_str("bad");
        let _: () = unsafe { msg_send![&*delegate, engine: engine, extensionInstallFailed: &*message, tab: none] };
        record(live, "extensionInstallFailed emits the message", emitted("extensions.installFailed") == Some(json!({"message": "bad", "tabId": null})), String::new());

        let (key, unshifted) = (NSString::from_str("q"), NSString::from_str("q"));
        let reserved: bool = unsafe { msg_send![&*delegate, engineIsReservedKey: &*key, unshiftedKey: &*unshifted, modifiers: 1usize << 20] };
        record(live, "engineIsReservedKey reaches the hook and returns its answer", reserved && hooks.reserved.borrow().len() == 1, String::new());

        let window: Option<Retained<Window>> = unsafe { msg_send![&*delegate, engineWindowForNewBrowserOfProfile: &*profile, type: &*NSString::from_str("popup")] };
        record(live, "engineWindowForNewBrowserOfProfile returns the hook's nil", window.is_none(), String::new());
    }

    /// The delegate that the engine uses is the one of `child`; a second one with the same hooks is as good for a
    /// direct call, because each callback reads only its ivars.
    fn ria_delegate() -> Retained<EngineDelegate> {
        EngineDelegate::new(HOOKS.with(Rc::clone), State::new(), None)
    }

    /// Each call of `services.rs` that is safe on a new profile.
    fn services_calls(live: &Rc<Live>, services: &Services) {
        let want = |name: &'static str, check: fn(&Result<Value, String>) -> bool| -> Reply {
            live.outstanding.set(live.outstanding.get() + 1);
            let live = live.clone();
            Box::new(move |r| {
                record(&live, name, check(&r), format!("{r:?}"));
                live.outstanding.set(live.outstanding.get() - 1);
            })
        };
        services.list_passwords("Default", want("list_passwords answers or the engine refuses with a text", |r| match r {
            Ok(v) => v["passwords"].is_array() || v.is_object(),
            Err(e) => !e.is_empty(),
        }));
        services.zoom_levels("Default", want("zoom_levels answers an object", |r| matches!(r, Ok(v) if v.is_object())));
        services.set_zoom(1.44, "Default", "Example.COM");
        let again = services.clone();
        // The engine answers calls in order, so this list comes after the set.
        again.zoom_levels("Default", want("zoom_levels shows the factor that set_zoom stored", |r| {
            matches!(r, Ok(v) if v["example.com"] == json!(1.44))
        }));
        services.addresses("Default", want("addresses answers a dated list", |r| matches!(r, Ok(v) if v["addresses"].is_array())));
        services.cards("Default", want("cards answers a dated list", |r| matches!(r, Ok(v) if v["cards"].is_array())));
        services.search_engine_list("Default", want("search_engine_list answers engines", |r| matches!(r, Ok(v) if v["list"]["engines"].is_array())));
        services.site_settings_origins("Default", want("site_settings_origins answers a list", |r| matches!(r, Ok(v) if v.is_array())));
        services.external_app_allowances("Default", want("external_app_allowances answers a list", |r| matches!(r, Ok(v) if v.is_array())));
        services.components(want("components answers a list", |r| matches!(r, Ok(v) if v.is_array())));
        services.list_extensions("Default", want("list_extensions answers a list", |r| matches!(r, Ok(v) if v["extensions"].is_array())));
        services.profile_toggles("Default", Some(json!({"passwords": false, "cards": true}).as_object().unwrap().clone()), want("profile_toggles stores and reads back", |r| {
            matches!(r, Ok(v) if v["passwords"] == false && v["cards"] == true)
        }));
        services.preload_pages("Default", want("preload_pages answers a bool", |r| matches!(r, Ok(v) if v.is_boolean())));
        services.export_passwords("Default", None, want("export_passwords with no path is cancelled", |r| matches!(r, Ok(v) if v["status"] == "cancelled")));
        services.watch_downloads("Default");
        services.clear_browsing_data("Default", &["cache"], 0.0, {
            live.outstanding.set(live.outstanding.get() + 1);
            let live = live.clone();
            move |error| {
                record(&live, "clear_browsing_data completes", error.is_none(), format!("{error:?}"));
                live.outstanding.set(live.outstanding.get() - 1);
            }
        });
    }

    /// Opens a window that is never shown, loads `about:blank` in a tab, and reads the tab.
    fn window_and_tab(live: &Rc<Live>, profile: &Profile) {
        let rect = NSRect::new(objc2_foundation::NSPoint::new(-3000.0, -3000.0), objc2_foundation::NSSize::new(400.0, 300.0));
        let window: Retained<Window> = unsafe { Window::init_with_content_rect(msg_send![class!(ArdiaCoreWindow), alloc], rect) };
        window.set_internal(true);
        window.prepare_profile(profile);
        let tab = window.open_tab(&NSString::from_str("about:blank"), profile, false);
        record(live, "Window.openTab makes a tab with an id", tab.tab_id() > 0, tab.tab_id().to_string());
        record(live, "the tab is in tabsForProfile", window.tabs_for_profile(profile).count() == 1, String::new());
        record(live, "Tab.profile is the profile", tab.profile().is_some_and(|p| &*p == profile), String::new());
        live.outstanding.set(live.outstanding.get() + 1);
        let live = live.clone();
        poll(
            Rc::new(move |waited| {
                let loaded = !tab.loading() && tab.url().to_string() == "about:blank";
                if loaded || waited >= Duration::from_secs(30) {
                    record(&live, "Tab loads about:blank and reports its url", loaded, format!("{} loading={}", tab.url(), tab.loading()));
                    tab.close();
                    window.close();
                    live.outstanding.set(live.outstanding.get() - 1);
                    return true;
                }
                false
            }),
            Duration::ZERO,
        );
    }
}
