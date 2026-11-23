// Ria original, no Ardia source.

//! Check L of slice 3.3: the data domains of `ria-engine` against the prebuilt engine. It runs as
//! `blaze test -p ria-engine --test live` (the `[[test]]` entry of `ria-engine/Cargo.toml` points here, so the row of
//! slice 3.3 holds the file).
//!
//! Needs a Mac and `ENGINE_DIR=<dir that holds "Chromium Framework.framework">`. The engine owns the main thread,
//! so this test has its own `main`. It wraps itself in an app bundle beside a copy of the framework (the engine
//! starts its helper processes from the bundle), starts the engine there with a temporary profile, and makes
//! calls with fixture arguments. Each reply must parse and each event that a call causes must arrive.

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
    use std::ffi::{CString, c_char, c_int};
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

    /// The process that cargo starts: builds the bundle, runs the child in it, forwards the exit code.
    pub fn parent() {
        let engine_dir = PathBuf::from(
            std::env::var_os("ENGINE_DIR")
                .expect("set ENGINE_DIR to the folder that holds Chromium Framework.framework"),
        );
        let root = std::env::temp_dir().join(format!("ria-data-live-{}", std::process::id()));
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
        let _ = std::fs::remove_dir_all(&root);
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

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "RiaDataLiveDelegate"]
        struct Delegate;

        impl Delegate {
            #[unsafe(method(engineDidStart))]
            fn engine_did_start(&self) {
                started();
            }

            #[unsafe(method(engineWillShutDown))]
            fn engine_will_shut_down(&self) {
                if let Some(live) = LIVE.with(|l| l.borrow().clone()) {
                    live.engine.shutdown();
                }
            }

            #[unsafe(method(engineQuitCancelled))]
            fn engine_quit_cancelled(&self) {}

            #[unsafe(method_id(engineWindowForNewBrowserOfProfile:type:))]
            fn window_for_new_browser(&self, _profile: &AnyObject, _kind: &NSString) -> Option<Retained<AnyObject>> {
                None
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
        let live = LIVE
            .with(|l| l.borrow().clone())
            .expect("the engine started");
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
        fn passwords_offered(&self) -> bool {
            true
        }
        fn set_passwords_offered(&self, _: bool) {}
        fn addresses_autofilled(&self) -> bool {
            true
        }
        fn set_addresses_autofilled(&self, _: bool) {}
        fn cards_autofilled(&self) -> bool {
            true
        }
        fn set_cards_autofilled(&self, _: bool) {}
        fn preload_pages(&self) -> bool {
            true
        }
        fn set_preload_pages(&self, _: bool) {}
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

    /// A page engine that has no page: nothing to resolve, nothing to clear.
    struct NoPages;

    impl PageEngine for NoPages {
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
            _: &str,
            _: &[String],
            _: f64,
            done: Box<dyn FnOnce(Option<String>)>,
        ) {
            done(None)
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

        let exported = std::env::temp_dir().join(format!("ria-live-{}.html", std::process::id()));
        let profile: Profile = Rc::new(LiveProfile(dir.clone()));
        let registry = ProfileRegistry::new(Rc::new(LiveCore(profile)), Rc::new(NoEvents));
        let ctx = DataCtx {
            engine: engine.clone(),
            favicons: Rc::new(Favicons::new(registry.clone(), std::env::temp_dir())),
            registry,
            services: Rc::new(LiveServices(exported.to_string_lossy().into_owned())),
            chromium: Rc::new(NoPages),
            webkit: Rc::new(NoPages),
            after: main_queue::after,
        };

        // Each call of the slice that reaches the engine, with fixture arguments. `C` is the typed call of slice 3.2:
        // the reply must parse into `C::Reply`.
        let site = "https://example.com";
        let url = "https://example.com/";
        expect::<api::bookmarks::Tree>(&live, &ctx, "bookmarks", "tree", json!({}), |r| {
            r.bar.k == ria_engine::types::EngineBookmarkK::F
        });
        expect::<api::bookmarks::Apply>(
            &live,
            &ctx,
            "bookmarks",
            "apply",
            json!({"ops": []}),
            |r| r.applied.0 == 0.0,
        );
        expect::<api::bookmarks::Watch>(&live, &ctx, "bookmarks", "watch", json!({}), |_| true);
        expect::<api::bookmarks::ExportHtml>(&live, &ctx, "bookmarks", "exportHtml", json!({}), {
            |r| r.as_deref().is_some_and(|p| p.ends_with(".html"))
        });
        expect::<api::downloads::Settings>(&live, &ctx, "downloads", "settings", json!({}), |r| {
            !r.folder.is_empty()
        });
        expect::<api::downloads::SetSettings>(
            &live,
            &ctx,
            "downloads",
            "setSettings",
            json!({"askWhereToSave": false}),
            |_| true,
        );
        expect::<api::favicons::Set>(
            &live,
            &ctx,
            "favicons",
            "set",
            json!({"page": url, "icon": "https://example.com/icon.png", "png": PNG}),
            |r| r.stored,
        );
        expect::<api::favicons::Get>(
            &live,
            &ctx,
            "favicons",
            "get",
            json!({"pages": [url], "icons": [], "size": 16}),
            |_| true,
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
        let history_live = live.clone();
        let history_ctx = ctx.clone();
        expect::<api::history::Watch>(&live, &ctx, "history", "watch", json!({}), move |_| {
            expect::<api::history::Add>(
                &history_live,
                &history_ctx,
                "history",
                "add",
                json!({"pages": [{"u": url, "t": "Example", "v": [1_700_000_000_000_i64]}], "toleranceMs": 0}),
                |r| r.added.0 == 1.0,
            );
            expect::<api::history::Import>(
                &history_live,
                &history_ctx,
                "history",
                "import",
                json!({"rows": [{"u": "https://example.org/", "t": "Org", "n": 1, "l": 1_700_000_000_000_i64}]}),
                |_| true,
            );
            expect::<api::history::Query>(
                &history_live,
                &history_ctx,
                "history",
                "query",
                json!({"maxUrls": 10, "maxVisits": 10}),
                |_| true,
            );
            expect::<api::history::DeleteUrls>(
                &history_live,
                &history_ctx,
                "history",
                "deleteUrls",
                json!({"urls": ["https://example.org/"]}),
                |_| true,
            );
            true
        });
        // A refused or stopped request gives `{netError}` or `{stopped}` without `status`, which `api::search::SuggestReply` (slice 3.2) requires; so the reply is read as a plain map.
        expect::<api::search::StopSuggest>(
            &live,
            &ctx,
            "search",
            "suggest",
            json!({"url": "http://127.0.0.1:1/suggest", "id": 1}),
            |r| {
                r.as_ref()
                    .is_some_and(|m| m.contains_key("netError") || m.contains_key("stopped"))
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
        expect::<api::sites::WriteStorage>(
            &live,
            &ctx,
            "sites",
            "writeStorage",
            json!({"origin": site, "set": [["k", "v"]], "remove": []}),
            |_| true,
        );
        expect::<api::sites::ReadStorage>(
            &live,
            &ctx,
            "sites",
            "readStorage",
            json!({"origin": site}),
            |_| true,
        );
        expect::<api::sites::RegistrableDomains>(
            &live,
            &ctx,
            "sites",
            "registrableDomains",
            json!({"hosts": ["www.example.com"]}),
            |r| r.domains == ["example.com"],
        );
        // `profiles.clearBrowsingData` goes to both page engines; Chromium clears through the engine.
        *live.outstanding.borrow_mut() += 1;
        let cleared = live.clone();
        clear_chromium_browsing_data(
            &ctx,
            "",
            &["cache".to_owned()],
            0.0,
            Box::new(move |error| {
                record(
                    &cleared,
                    "browsing data clears the cache",
                    error.is_none(),
                    format!("{error:?}"),
                );
                *cleared.outstanding.borrow_mut() -= 1;
            }),
        );
        // An unknown profile gives the Ardia error text.
        *live.outstanding.borrow_mut() += 1;
        let unknown = live.clone();
        ctx.engine_call(
            "ac_bookmarks_tree",
            "never-loaded-profile",
            json!({}),
            move |value| {
                record(
                    &unknown,
                    "a profile that does not load is an error",
                    value.get("error").is_some(),
                    value.to_string(),
                );
                *unknown.outstanding.borrow_mut() -= 1;
            },
        );
        let missing = ria_engine_sys::missing_calls();
        record(
            &live,
            "all 106 symbols resolve",
            missing.is_empty(),
            format!("missing: {missing:?}"),
        );

        let watch = live.clone();
        let engine_object = engine_object.clone();
        poll(
            Rc::new(move |waited: Duration| {
                let outstanding = *watch.outstanding.borrow();
                let seen = history_seen.get();
                if (outstanding != 0 || !seen) && waited < Duration::from_secs(60) {
                    return false;
                }
                record(
                    &watch,
                    "history.changed includes the inserted URL",
                    seen,
                    url.into(),
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
