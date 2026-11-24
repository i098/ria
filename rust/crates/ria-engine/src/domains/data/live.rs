// Ria original, no Ardia source.

//! Check L of slice 3.3 exercises selected `ria-engine` data calls against prebuilt Chromium.
//! From `rust/` on macOS, set `ENGINE_DIR` to the directory that contains `Chromium Framework.framework`.
//! Run `blaze test -p ria-engine --test live`.
//! The check has its own `main` because the engine owns the main thread.
//! It copies the framework into a temporary app bundle so the engine can start its helper processes.
//! The check uses a temporary profile to verify selected replies and events.
//! The harness supplies the bookmark export path instead of opening a native save panel.
//! Changes from Ardia: Ardia has no test of `bookmarks.changed` delivery. This check installs an unpacked extension
//! whose service worker calls `chrome.bookmarks.create`, a change outside `bookmarks.apply` (`ac_bookmarks.cc:130`,
//! `:152`), and requires the event. The captain ordered every open issue resolved (issue 128).
//! It does not exercise WebKit, private profile lifecycle, or the content blocker.

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
    use std::ffi::{CString, OsStr, c_char, c_int};
    use std::io::Write;
    use std::os::unix::ffi::OsStrExt;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;
    use std::time::Duration;

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject};
    use objc2::{ClassType, class, define_class, msg_send};
    use objc2_foundation::NSString;
    use ria_engine::api::{self, Call, decode_reply};
    use ria_engine::call::{ActionQueue, Engine, main_queue};
    use ria_engine::domains::data::call_table::{CallReply, Settled};
    use ria_engine::domains::data::ctx::DataCtx;
    use ria_engine::domains::data::favicons::Favicons;
    use ria_engine::domains::data::host::{
        CoreEngine, CoreProfile, EventSink, PageEngine, Profile, ServiceDone, Services,
    };
    use ria_engine::domains::data::page_engines::clear_chromium_browsing_data;
    use ria_engine::domains::data::profiles::ProfileRegistry;
    use ria_engine::types::HistoryWireChange;
    use serde_json::{Value, json};

    const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>live</string>
<key>CFBundleIdentifier</key><string>sh.ria.data-live</string>
<key>CFBundleName</key><string>Ria Engine Live</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>0</string>
<key>NSPrincipalClass</key><string>BrowserCrApplication</string>
</dict></plist>"#;

    struct TestRoot<'a>(&'a Path);

    impl Drop for TestRoot<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }

    /// The process that cargo starts: builds the bundle, runs the child in it, forwards the exit code.
    pub fn parent() {
        let engine_dir = PathBuf::from(
            std::env::var_os("ENGINE_DIR")
                .expect("set ENGINE_DIR to the folder that holds Chromium Framework.framework"),
        );
        let template = std::env::temp_dir().join("ria-data-live-XXXXXX");
        let mut template = CString::new(template.as_os_str().as_bytes())
            .unwrap()
            .into_bytes_with_nul();
        let created = unsafe { libc::mkdtemp(template.as_mut_ptr().cast()) };
        assert!(
            !created.is_null(),
            "cannot create the live test root: {}",
            std::io::Error::last_os_error()
        );
        let root = Path::new(OsStr::from_bytes(&template[..template.len() - 1]));
        let cleanup = TestRoot(root);
        let macos = root.join("Live.app/Contents/MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::create_dir_all(root.join("Live.app/Contents/Frameworks")).unwrap();
        std::fs::write(root.join("Live.app/Contents/Info.plist"), INFO_PLIST).unwrap();
        let exe = macos.join("live");
        std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        run(Command::new("cp")
            .arg("-cR")
            .arg(engine_dir.join("Chromium Framework.framework"))
            .arg(root.join("Live.app/Contents/Frameworks/")));
        run(Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .arg(&exe));
        let data = root.join("data");
        std::fs::create_dir_all(&data).unwrap();
        let status = Command::new(&exe)
            .env("RIA_LIVE_CHILD", "1")
            .env("RIA_LIVE_DATA", &data)
            .status()
            .expect("start the child");
        drop(cleanup);
        std::process::exit(status.code().unwrap_or(1));
    }

    fn run(command: &mut Command) {
        let status = command.status().expect("run a setup command");
        assert!(status.success(), "setup command failed: {command:?}");
    }

    struct Live {
        engine: Engine,
        checks: RefCell<Vec<(String, bool, String)>>,
        outstanding: RefCell<usize>,
        finished: RefCell<bool>,
    }

    thread_local! {
        static LIVE: RefCell<Option<Rc<Live>>> = const { RefCell::new(None) };
    }

    fn callback<T>(name: &str, work: impl FnOnce() -> T) -> T {
        match catch_unwind(AssertUnwindSafe(work)) {
            Ok(value) => value,
            Err(_) => {
                let _ = writeln!(std::io::stderr(), "Ria data live: panic in {name}");
                std::process::abort();
            }
        }
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "RiaDataLiveDelegate"]
        struct Delegate;

        impl Delegate {
            #[unsafe(method(engineDidStart))]
            fn engine_did_start(&self) {
                callback("engineDidStart", started);
            }

            #[unsafe(method(engineWillShutDown))]
            fn engine_will_shut_down(&self) {
                callback("engineWillShutDown", || {
                    if let Some(live) = LIVE.with(|l| l.borrow().clone()) {
                        live.engine.shutdown();
                    }
                });
            }

            #[unsafe(method(engineQuitCancelled))]
            fn engine_quit_cancelled(&self) {
                callback("engineQuitCancelled", || ());
            }

            #[unsafe(method_id(engineWindowForNewBrowserOfProfile:type:))]
            fn window_for_new_browser(&self, _profile: &AnyObject, _kind: &NSString) -> Option<Retained<AnyObject>> {
                callback("engineWindowForNewBrowserOfProfile:type:", || None)
            }
        }
    );

    pub fn child() {
        let exe = std::env::current_exe().unwrap();
        let contents = exe
            .parent()
            .and_then(Path::parent)
            .expect("inside a bundle");
        let framework = CString::new(
            contents
                .join("Frameworks/Chromium Framework.framework/Versions/Current/Chromium Framework")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        // SAFETY: loads the engine once, before any class of it is used.
        assert!(
            !unsafe { libc::dlopen(framework.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) }
                .is_null(),
            "cannot load the engine framework"
        );

        let data = std::env::var("RIA_LIVE_DATA").unwrap();
        let argv = [
            std::env::args().next().unwrap(),
            format!("--user-data-dir={data}/Chromium"),
            "--use-mock-keychain".into(),
            "--profile-directory=Default".into(),
            "--no-first-run".into(),
            "--disable-stack-profiler".into(),
            // The engine runs BEST_EFFORT tasks (the bookmark HTML writer posts one) only after startup completes. With no
            // visible tab loading, this feature ends that wait after 5 s. Same value as `ArdiaHost+Launch.mm` and
            // `ria-host-mac` `launch/startup.rs`.
            "--enable-features=ImprovedStartupBestEffortDelay:StartupDelayStopOnLoadingTimedOut/true".into(),
        ];
        let c_args: Vec<CString> = argv
            .iter()
            .map(|a| CString::new(a.as_str()).unwrap())
            .collect();
        let mut ptrs: Vec<*const c_char> = c_args.iter().map(|a| a.as_ptr()).collect();
        ptrs.push(std::ptr::null());

        let delegate: Retained<AnyObject> = unsafe { msg_send![Delegate::class(), new] };
        let class = objc2::runtime::AnyClass::get(c"ArdiaCoreEngine")
            .expect("ArdiaCoreEngine is in the framework");
        let code: c_int = unsafe {
            msg_send![class, runWithArgc: (ptrs.len() - 1) as c_int, argv: ptrs.as_ptr(), delegate: &*delegate]
        };
        let Some(live) = LIVE.with(|l| l.borrow().clone()) else {
            println!(
                "{}",
                json!({
                    "live": "result",
                    "exit_code": code,
                    "checks": 0,
                    "failed": 0,
                    "pass": false,
                    "detail": "the engine returned before the profile was ready"
                })
            );
            std::process::exit(if code == 0 { 1 } else { code });
        };
        let checks = live.checks.borrow();
        let failed = checks.iter().filter(|c| !c.1).count();
        let pass = code == 0 && failed == 0 && *live.finished.borrow();
        println!(
            "{}",
            json!({"live": "result", "exit_code": code, "checks": checks.len(), "failed": failed, "pass": pass})
        );
        std::process::exit(if pass { 0 } else { 1 });
    }

    /// The one profile that the engine loaded: the registry gets it from `LiveCore`.
    struct LiveProfile(String);

    impl CoreProfile for LiveProfile {
        fn path(&self) -> String {
            self.0.clone()
        }
        fn directory_name(&self) -> String {
            "Default".into()
        }
        fn off_the_record(&self) -> bool {
            false
        }
        fn destroyed(&self) -> bool {
            false
        }
        fn fetch_icon(&self, _: &str, _: usize, done: Box<dyn FnOnce(Option<Vec<u8>>)>) {
            done(None)
        }
        fn load_component_extension(&self, _: &str) -> String {
            String::new()
        }
        fn hold_navigations_until_rulesets_of(&self, _: &str) {}
        fn release_navigation_hold_ready(&self) {}
        fn release_navigation_hold(&self, _: &str) {}
    }

    struct LiveCore(Profile);

    impl CoreEngine for LiveCore {
        fn default_profile(&self) -> Option<Profile> {
            Some(self.0.clone())
        }
        fn load_profile(&self, _: &str, done: Box<dyn FnOnce(Option<Profile>)>) {
            done(None)
        }
        fn off_the_record_profile_for(
            &self,
            _: &Profile,
            done: Box<dyn FnOnce(Option<Profile>, Option<String>)>,
        ) {
            done(None, Some("not in the live test".into()))
        }
        fn release_profile(&self, _: &Profile) {}
    }

    struct NoEvents;

    impl EventSink for NoEvents {
        fn emit(&self, _: &str, _: Value) {}
    }

    /// The native services that the data calls use are slice 3.5's: only the save panel answers here.
    struct LiveServices(String);

    impl Services for LiveServices {
        fn run(&self, op: &str, _: &str, _: Value, done: ServiceDone) {
            done(match op {
                "savePanel" => Ok(Value::String(self.0.clone())),
                _ => Err(format!("{op} is not in the live test")),
            })
        }
    }

    struct LivePages(Option<DataCtx>);

    impl PageEngine for LivePages {
        fn owns(&self, _: &str) -> bool {
            false
        }
        fn resolve_permission(&self, _: &str, _: &str, _: bool) {}
        fn resolve_external_app(&self, _: &str, _: bool, _: bool) {}
        fn download_command(&self, _: &str, _: &str) {}
        fn external_app_allowances(&self, _: &str, done: Box<dyn FnOnce(Vec<Value>)>) {
            done(vec![])
        }
        fn remove_external_app_allowance(&self, _: &str, _: &str, _: &str) {}
        fn site_settings(&self, _: &str, _: &str, done: ServiceDone) {
            done(Ok(Value::Null))
        }
        fn set_site_setting(&self, _: &str, _: &str, _: &str, _: &str) {}
        fn clear_site_data(&self, _: &str, _: &str, done: ServiceDone) {
            done(Ok(Value::Null))
        }
        fn clear_browsing_data(
            &self,
            profile: &str,
            types: &[String],
            since: f64,
            done: Box<dyn FnOnce(Option<String>)>,
        ) {
            match &self.0 {
                Some(ctx) => clear_chromium_browsing_data(ctx, profile, types, since, done),
                None => done(None),
            }
        }
        fn delete_profile_data(&self, _: &str, done: Box<dyn FnOnce(Vec<String>)>) {
            done(vec![])
        }
    }

    /// A 1 x 1 PNG.
    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    fn started() {
        let engine: Retained<AnyObject> =
            unsafe { msg_send![class!(ArdiaCoreEngine), sharedEngine] };
        let loaded: Option<Retained<AnyObject>> = unsafe { msg_send![&*engine, defaultProfile] };
        if let Some(profile) = loaded {
            return go(&engine, &profile);
        }
        let again = engine.clone();
        let block = RcBlock::new(move |profile: *mut AnyObject| {
            callback("loadProfile completion", || {
                match unsafe { Retained::retain(profile) } {
                    Some(profile) => go(&again, &profile),
                    None => {
                        println!(
                            "{}",
                            json!({"live": "error", "detail": "loadProfile returned nil"})
                        );
                        std::process::exit(2);
                    }
                }
            });
        });
        let name = NSString::from_str("Default");
        let _: () = unsafe { msg_send![&*engine, loadProfile: &*name, completion: &*block] };
    }

    fn go(engine_object: &Retained<AnyObject>, profile: &Retained<AnyObject>) {
        let path: Retained<NSString> = unsafe { msg_send![&**profile, path] };
        let dir = path.to_string();
        let queue = ActionQueue::new();
        let weak = Rc::downgrade(&queue);
        queue.set_waker(move || {
            let weak = weak.clone();
            main_queue::post(Box::new(move || _ = weak.upgrade().map(|q| q.drain())));
        });
        let engine = Engine::new(queue.clone());
        engine.did_start();
        let live = Rc::new(Live {
            engine: engine.clone(),
            checks: RefCell::default(),
            outstanding: RefCell::default(),
            finished: RefCell::default(),
        });
        LIVE.with(|l| *l.borrow_mut() = Some(live.clone()));

        let data = PathBuf::from(std::env::var_os("RIA_LIVE_DATA").unwrap());
        let exported = data.join("bookmarks.html");
        let profile: Profile = Rc::new(LiveProfile(dir.clone()));
        let registry = ProfileRegistry::new(Rc::new(LiveCore(profile)), Rc::new(NoEvents));
        let ctx = DataCtx {
            engine: engine.clone(),
            favicons: Rc::new(Favicons::new(registry.clone(), data)),
            registry,
            services: Rc::new(LiveServices(exported.to_string_lossy().into_owned())),
            chromium: Rc::new(LivePages(None)),
            webkit: Rc::new(LivePages(None)),
            after: main_queue::after,
        };
        let ctx = DataCtx {
            chromium: Rc::new(LivePages(Some(ctx.clone()))),
            ..ctx
        };

        // Each call of the slice that reaches the engine, with fixture arguments. `C` is the typed call of slice 3.2:
        // the reply must parse into `C::Reply`.
        let site = "https://example.com";
        let url = "https://example.com/";
        let bookmark_seen = Rc::new(Cell::new(false));
        let quiet_window = Rc::new(Cell::new(None));
        let extension_seen = Rc::new(Cell::new(false));
        check_bookmarks(
            &live,
            &ctx,
            exported,
            url,
            dir,
            bookmark_seen,
            quiet_window.clone(),
            extension_seen.clone(),
        );
        check_download_settings(&live, &ctx);

        let icon = "https://example.com/icon.png";
        expect_then::<api::favicons::Set>(
            &live,
            &ctx,
            "favicons",
            "set",
            json!({"page": url, "icon": icon, "png": PNG}),
            move |live, ctx, r| {
                expect::<api::favicons::Get>(
                    live,
                    ctx,
                    "favicons",
                    "get",
                    json!({"pages": [url], "icons": [icon], "size": 16}),
                    move |r| {
                        r.pages
                            .get(url)
                            .is_some_and(|entry| entry.icon == icon && !entry.png.is_empty())
                            && r.icons.get(icon).is_some_and(|entry| !entry.png.is_empty())
                    },
                );
                r.stored
            },
        );
        expect::<api::favicons::Fetch>(
            &live,
            &ctx,
            "favicons",
            "fetch",
            json!({"url": format!("data:image/png;base64,{PNG}")}),
            |r| {
                r.as_ref().is_some_and(|r| {
                    r.uri.starts_with("data:image/png;base64,") && r.width.0 == 1.0
                })
            },
        );
        let history_seen = Rc::new(Cell::new(false));
        let seen = history_seen.clone();
        api::subscribe::<api::events::HistoryChanged>(&engine, "history.changed", move |batch| {
            if batch.is_ok_and(|batch| {
                batch.changes.iter().any(|change| match change {
                    HistoryWireChange::Visit { u, .. } => u == url,
                    HistoryWireChange::Modified { rows } => rows.iter().any(|row| row.u == url),
                    HistoryWireChange::Deleted { .. } => false,
                })
            }) {
                seen.set(true);
            }
        });
        expect_then::<api::history::Watch>(
            &live,
            &ctx,
            "history",
            "watch",
            json!({}),
            move |live, ctx, _| {
                check_history(live, ctx, url);
                true
            },
        );
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let suggest_url = format!("http://{}/suggest", listener.local_addr().unwrap());
        expect::<api::search::StopSuggest>(
            &live,
            &ctx,
            "search",
            "suggest",
            json!({"url": suggest_url, "id": 1}),
            move |r| {
                drop(listener);
                r.as_ref().and_then(|m| m.get("stopped")) == Some(&Value::Bool(true))
            },
        );
        expect::<api::search::StopSuggest>(
            &live,
            &ctx,
            "search",
            "stopSuggest",
            json!({"id": 1}),
            |_| true,
        );
        check_storage_clear(&live, &ctx, site);
        expect::<api::sites::RegistrableDomains>(
            &live,
            &ctx,
            "sites",
            "registrableDomains",
            json!({"hosts": ["www.example.com"]}),
            |r| r.domains == ["example.com"],
        );
        let watch = live.clone();
        let engine_object = engine_object.clone();
        poll(
            Rc::new(move |waited: Duration| {
                let outstanding = *watch.outstanding.borrow();
                let seen = history_seen.get();
                let quiet = quiet_window.get();
                if (outstanding != 0 || !seen || quiet.is_none() || !extension_seen.get())
                    && waited < Duration::from_secs(150)
                {
                    return false;
                }
                record(
                    &watch,
                    "history.changed includes the inserted URL",
                    seen,
                    url.into(),
                );
                // Ardia's engine mutes the event while `bookmarks.apply` edits: `ac_bookmarks.cc:152` returns when
                // `applying_` is set, and `:438` and `:456` set and clear it around the edits. The event must not
                // arrive in the 500 ms after the reply either.
                record(
                    &watch,
                    "bookmarks.apply raises no bookmarks.changed within 500 ms (Ardia ac_bookmarks.cc:152)",
                    quiet == Some(true),
                    String::new(),
                );
                // No engine call edits bookmarks outside `bookmarks.apply`. An installed extension does: its
                // `chrome.bookmarks.create` reaches `BookmarkNodeAdded` (`ac_bookmarks.cc:130`) with `applying_` clear.
                record(
                    &watch,
                    "bookmarks.changed arrives for a bookmark that an extension creates (Ardia ac_bookmarks.cc:130)",
                    extension_seen.get(),
                    String::new(),
                );
                if outstanding != 0 {
                    record(
                        &watch,
                        "all calls settled",
                        false,
                        format!("{outstanding} calls did not settle before shutdown"),
                    );
                }
                *watch.finished.borrow_mut() = true;
                for (name, ok, detail) in watch.checks.borrow().iter() {
                    println!(
                        "{}",
                        json!({"live": "check", "name": name, "pass": ok, "detail": detail})
                    );
                }
                let _: () = unsafe { msg_send![&*engine_object, quit] };
                true
            }),
            Duration::ZERO,
        );
    }

    fn check_storage_clear(live: &Rc<Live>, ctx: &DataCtx, origin: &'static str) {
        expect_then::<api::sites::WriteStorage>(
            live,
            ctx,
            "sites",
            "writeStorage",
            json!({"origin": origin, "set": [["k", "v"]], "remove": []}),
            move |live, ctx, written| {
                if written.written.0 != 1.0 {
                    return false;
                }
                expect_then::<api::sites::ReadStorage>(
                    live,
                    ctx,
                    "sites",
                    "readStorage",
                    json!({"origin": origin}),
                    move |live, ctx, seeded| {
                        if !seeded.items.as_ref().is_some_and(|items| {
                            items.iter().any(|(key, value)| key == "k" && value == "v")
                        }) {
                            return false;
                        }
                        expect_then::<api::profiles::ClearBrowsingData>(
                            live,
                            ctx,
                            "profiles",
                            "clearBrowsingData",
                            json!({"types": ["siteData"], "since": 0}),
                            move |live, ctx, ()| {
                                expect::<api::sites::ReadStorage>(
                                    live,
                                    ctx,
                                    "sites",
                                    "readStorage",
                                    json!({"origin": origin}),
                                    |cleared| cleared.items.as_ref().is_none_or(Vec::is_empty),
                                );
                                true
                            },
                        );
                        true
                    },
                );
                true
            },
        );
    }

    /// Installs an unpacked extension whose service worker calls `chrome.bookmarks.create`. That is a bookmark change
    /// outside `bookmarks.apply`, so Ardia's watcher is not muted (`ac_bookmarks.cc:130`, `:152`).
    fn install_bookmark_extension(live: &Rc<Live>, ctx: &DataCtx, dir: &Path) {
        let written = std::fs::create_dir_all(dir)
            .and_then(|()| {
                std::fs::write(
                    dir.join("manifest.json"),
                    r#"{"manifest_version":3,"name":"Ria live bookmarks","version":"1","permissions":["bookmarks"],"background":{"service_worker":"sw.js"}}"#,
                )
            })
            .and_then(|()| {
                std::fs::write(
                    dir.join("sw.js"),
                    "chrome.bookmarks.create({title: 'Ria live extension bookmark', url: 'https://example.net/extension'});",
                )
            });
        if let Err(error) = written {
            return record(
                live,
                "the bookmark extension is written",
                false,
                error.to_string(),
            );
        }
        *live.outstanding.borrow_mut() += 1;
        let live = live.clone();
        ctx.engine_call(
            "ac_extensions_install",
            "",
            json!({"path": dir.to_string_lossy()}),
            move |reply| {
                let ok = reply.get("id").is_some_and(Value::is_string);
                record(
                    &live,
                    "ac_extensions_install loads the bookmark extension",
                    ok,
                    reply.to_string(),
                );
                *live.outstanding.borrow_mut() -= 1;
            },
        );
    }

    fn holds_extension_node(node: &ria_engine::types::EngineBookmark) -> bool {
        node.u.as_deref() == Some("https://example.net/extension")
            || node.c.iter().flatten().any(holds_extension_node)
    }

    #[allow(clippy::too_many_arguments)]
    fn check_bookmarks(
        live: &Rc<Live>,
        ctx: &DataCtx,
        exported: PathBuf,
        url: &'static str,
        profile_path: String,
        seen: Rc<Cell<bool>>,
        window: Rc<Cell<Option<bool>>>,
        delivered: Rc<Cell<bool>>,
    ) {
        let seen_after = seen.clone();
        let armed = Rc::new(Cell::new(false));
        let watch_armed = armed.clone();
        let window_state = window.clone();
        let checking = Rc::new(Cell::new(false));
        let (event_live, event_ctx) = (live.clone(), ctx.clone());
        api::subscribe::<api::events::BookmarksChanged>(
            &ctx.engine,
            "bookmarks.changed",
            move |event| {
                if watch_armed.get() && event.is_ok_and(|event| event.profile == profile_path) {
                    // Before the quiet window ends the event belongs to `bookmarks.apply`; after it, to the extension.
                    if window_state.get().is_none() {
                        seen.set(true);
                    } else if !checking.replace(true) {
                        let (delivered, checking) = (delivered.clone(), checking.clone());
                        expect_then::<api::bookmarks::Tree>(
                            &event_live,
                            &event_ctx,
                            "bookmarks",
                            "tree",
                            json!({}),
                            move |_, _, tree| {
                                let found = holds_extension_node(&tree.bar)
                                    || holds_extension_node(&tree.other);
                                if found {
                                    delivered.set(true);
                                } else {
                                    checking.set(false);
                                }
                                true
                            },
                        );
                    }
                }
            },
        );
        expect_then::<api::bookmarks::Watch>(
            live,
            ctx,
            "bookmarks",
            "watch",
            json!({}),
            move |live, ctx, _| {
                expect_then::<api::bookmarks::Tree>(
                    live,
                    ctx,
                    "bookmarks",
                    "tree",
                    json!({}),
                    move |live, ctx, tree| {
                        armed.set(true);
                        expect_then::<api::bookmarks::Apply>(
                            live,
                            ctx,
                            "bookmarks",
                            "apply",
                            json!({"ops": [{"op": "add", "id": "3df75ccb-f536-4d07-a8fc-90b729ff3299",
                                "k": "u", "t": "Ria live bookmark", "u": url, "a": 1_700_000_000_000_i64,
                                "parent": tree.bar.id}]}),
                            move |live, ctx, r| {
                                let (seen, window) = (seen_after, window);
                                let extension_dir = exported.with_file_name("bookmark-extension");
                                let (after_live, after_ctx) = (live.clone(), ctx.clone());
                                main_queue::after(
                                    Duration::from_millis(500),
                                    Box::new(move || {
                                        window.set(Some(!seen.get()));
                                        // Only after the quiet window: from here every event counts as delivered.
                                        install_bookmark_extension(
                                            &after_live,
                                            &after_ctx,
                                            &extension_dir,
                                        );
                                    }),
                                );
                                // Ardia's caller waits `EXPORT_ANSWER_MS` = 120 s, then throws ERR_ENGINE.
                                // Only a settled call counts: the file with the bookmark, or an ERR_ENGINE rejection.
                                *live.outstanding.borrow_mut() += 1;
                                let done = live.clone();
                                let settle = CallReply::new(move |settled: Settled| {
                                    let (ok, name, detail) = match settled {
                                        Ok(text) => {
                                            let reply = serde_json::from_str::<Value>(&text)
                                                .map_err(|error| error.to_string())
                                                .and_then(|value| {
                                                    decode_reply::<api::bookmarks::ExportHtml>(Ok(
                                                        value,
                                                    ))
                                                    .map_err(|error| format!("{error:?}"))
                                                });
                                            let proven = reply.as_ref().is_ok_and(|path| {
                                                path.as_deref().map(Path::new)
                                                    == Some(exported.as_path())
                                            }) && std::fs::read_to_string(&exported)
                                                .is_ok_and(|html| {
                                                    html.contains(url)
                                                        && html.contains("Ria live bookmark")
                                                });
                                            (
                                                proven,
                                                "bookmarks.exportHtml settled ok with the file on disk".to_owned(),
                                                text,
                                            )
                                        }
                                        Err(rejection) => (
                                            false,
                                            "bookmarks.exportHtml settled ok with the file on disk"
                                                .to_owned(),
                                            format!("{}: {}", rejection.code, rejection.message),
                                        ),
                                    };
                                    record(&done, &name, ok, detail);
                                    *done.outstanding.borrow_mut() -= 1;
                                });
                                ria_engine::domains::data::tables::run_data_call(
                                    ctx,
                                    "bookmarks",
                                    "exportHtml",
                                    "",
                                    "{}",
                                    settle,
                                );
                                r.applied.0 == 1.0 && r.invalid.is_empty()
                            },
                        );
                        tree.bar.k == ria_engine::types::EngineBookmarkK::F
                    },
                );
                true
            },
        );
    }

    fn check_download_settings(live: &Rc<Live>, ctx: &DataCtx) {
        expect_then::<api::downloads::Settings>(
            live,
            ctx,
            "downloads",
            "settings",
            json!({}),
            |live, ctx, before| {
                let ask = !before.ask_where_to_save;
                expect_then::<api::downloads::SetSettings>(
                    live,
                    ctx,
                    "downloads",
                    "setSettings",
                    json!({"askWhereToSave": ask}),
                    move |live, ctx, r| {
                        expect::<api::downloads::Settings>(
                            live,
                            ctx,
                            "downloads",
                            "settings",
                            json!({}),
                            move |after| after.ask_where_to_save == ask,
                        );
                        r.ask_where_to_save == ask
                    },
                );
                !before.folder.is_empty()
            },
        );
    }

    fn check_history(live: &Rc<Live>, ctx: &DataCtx, added: &'static str) {
        const IMPORTED: &str = "https://example.org/";
        expect_then::<api::history::Add>(
            live,
            ctx,
            "history",
            "add",
            json!({"pages": [{"u": added, "t": "Example", "v": [1_700_000_000_000_i64]}], "toleranceMs": 0}),
            move |live, ctx, r| {
                expect_then::<api::history::Import>(
                    live,
                    ctx,
                    "history",
                    "import",
                    json!({"rows": [{"u": IMPORTED, "t": "Org", "n": 1, "l": 1_700_000_000_000_i64}]}),
                    move |live, ctx, _| {
                        expect_then::<api::history::Query>(
                            live,
                            ctx,
                            "history",
                            "query",
                            json!({"maxUrls": 10, "maxVisits": 10}),
                            move |live, ctx, before| {
                                expect_then::<api::history::DeleteUrls>(
                                    live,
                                    ctx,
                                    "history",
                                    "deleteUrls",
                                    json!({"urls": [IMPORTED]}),
                                    move |live, ctx, _| {
                                        expect::<api::history::Query>(
                                            live,
                                            ctx,
                                            "history",
                                            "query",
                                            json!({"maxUrls": 10, "maxVisits": 10}),
                                            move |after| {
                                                after.entries.iter().any(|entry| entry.u == added)
                                                    && after
                                                        .entries
                                                        .iter()
                                                        .all(|entry| entry.u != IMPORTED)
                                            },
                                        );
                                        true
                                    },
                                );
                                before.entries.iter().any(|entry| entry.u == added)
                                    && before.entries.iter().any(|entry| entry.u == IMPORTED)
                            },
                        );
                        true
                    },
                );
                r.added.0 == 1.0
            },
        );
    }

    fn expect_then<C: Call + 'static>(
        live: &Rc<Live>,
        ctx: &DataCtx,
        domain: &'static str,
        name: &'static str,
        args: Value,
        pass: impl FnOnce(&Rc<Live>, &DataCtx, &C::Reply) -> bool + 'static,
    ) where
        C::Reply: std::fmt::Debug,
    {
        let next_live = live.clone();
        let next_ctx = ctx.clone();
        expect::<C>(live, ctx, domain, name, args, move |reply| {
            pass(&next_live, &next_ctx, reply)
        });
    }

    fn poll(step: Rc<dyn Fn(Duration) -> bool>, waited: Duration) {
        if step(waited) {
            return;
        }
        let next = Duration::from_millis(100);
        main_queue::after(next, Box::new(move || poll(step, waited + next)));
    }

    fn record(live: &Live, name: &str, pass: bool, detail: String) {
        live.checks
            .borrow_mut()
            .push((name.to_owned(), pass, detail));
    }

    /// Makes call `domain.name` through the call table, then checks that the reply parses into the typed reply of `C`.
    fn expect<C: Call + 'static>(
        live: &Rc<Live>,
        ctx: &DataCtx,
        domain: &'static str,
        name: &'static str,
        args: Value,
        pass: impl FnOnce(&C::Reply) -> bool + 'static,
    ) where
        C::Reply: std::fmt::Debug,
    {
        *live.outstanding.borrow_mut() += 1;
        let live = live.clone();
        let settle = CallReply::new(move |settled: Settled| {
            let label = format!("{domain}.{name} replies and parses (typed)");
            let (ok, detail) = match settled {
                Ok(text) => match serde_json::from_str::<Value>(&text) {
                    Ok(value) => match decode_reply::<C>(Ok(value)) {
                        Ok(reply) => (pass(&reply), format!("{reply:?}")),
                        Err(error) => (false, format!("{error:?}")),
                    },
                    Err(error) => (false, error.to_string()),
                },
                Err(rejection) => (false, format!("{rejection:?}")),
            };
            record(&live, &label, ok, detail);
            *live.outstanding.borrow_mut() -= 1;
        });
        // The blocker is not in the live test: its calls need an extension page, which the host of slice 3.5 owns.
        ria_engine::domains::data::tables::run_data_call(
            ctx,
            domain,
            name,
            "",
            &args.to_string(),
            settle,
        );
    }
}
