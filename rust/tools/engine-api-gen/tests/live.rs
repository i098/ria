// Ria original, no Ardia source.

//! Check L of slice 3.2: the typed API of `ria-engine` against the prebuilt engine. It runs as `blaze test --test live`
//! inside `rust/tools/engine-api-gen/` (a package of its own, so the row of slice 3.2 holds all its files).
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
    use std::cell::RefCell;
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
    use ria_engine::api::{self, ApiError, Call, call_symbol, events, subscribe};
    use ria_engine::call::{ActionQueue, Engine, main_queue};
    use ria_engine::types::{Number, Profile};
    use serde_json::{Value, json};

    const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>live</string>
<key>CFBundleIdentifier</key><string>sh.ria.engine-live</string>
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
        let root = std::env::temp_dir().join(format!("ria-engine-live-{}", std::process::id()));
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
        events: RefCell<Vec<Value>>,
        finished: RefCell<bool>,
    }

    thread_local! {
        static LIVE: RefCell<Option<Rc<Live>>> = const { RefCell::new(None) };
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "RiaEngineLiveDelegate"]
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
        println!(
            "{}",
            json!({"live": "result", "exit_code": code, "checks": checks.len(), "failed": failed, "pass": code == 0 && failed == 0 && !checks.is_empty()})
        );
        std::process::exit(if code == 0 && failed == 0 && !checks.is_empty() {
            0
        } else {
            1
        });
    }

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
        // The drain runs from the main queue, so it also runs inside the engine's own run loop.
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
            events: RefCell::default(),
            finished: RefCell::default(),
        });
        LIVE.with(|l| *l.borrow_mut() = Some(live.clone()));

        let profile = || Profile {
            profile: dir.clone(),
        };

        // Each call of the slice that goes straight to an engine symbol, with fixture arguments.
        // The other calls of `api.ts` go through the Swift modules of Ardia; slices 3.3 and 3.4 map them.
        expect::<api::bookmarks::Tree>(
            &live,
            "bookmarks.tree replies bar and other (typed)",
            &dir,
            "ac_bookmarks_tree",
            profile(),
            |r| r.bar.k == ria_engine::types::EngineBookmarkK::F,
        );
        expect::<api::downloads::Settings>(
            &live,
            "downloads.settings replies a folder (typed)",
            &dir,
            "ac_downloads_settings",
            profile(),
            |r| !r.folder.is_empty(),
        );
        expect::<api::history::Query>(
            &live,
            "history.query replies entries (typed)",
            &dir,
            "ac_history_query",
            api::history::QueryArgs {
                profile: dir.clone(),
                max_urls: Number(10.0),
                max_visits: Number(10.0),
            },
            |_| true,
        );
        expect::<api::tabs::LastTabPolicy>(
            &live,
            "tabs.lastTabPolicy replies (typed)",
            &dir,
            "ac_tabs_last_tab_policy",
            api::NoArgs {},
            |_| true,
        );
        expect::<api::engine::CliControlStatus>(
            &live,
            "engine.cliControlStatus replies a state (typed)",
            &dir,
            "ac_cli_control",
            api::NoArgs {},
            |_| true,
        );
        expect::<api::sites::RegistrableDomains>(
            &live,
            "sites.registrableDomains replies one domain (typed)",
            &dir,
            "ac_registrable_domains",
            api::sites::RegistrableDomainsArgs {
                profile: dir.clone(),
                hosts: vec!["www.example.com".into()],
            },
            |r| r.domains == ["example.com"],
        );
        expect::<api::history::Watch>(
            &live,
            "history.watch replies (typed)",
            &dir,
            "ac_history_watch",
            profile(),
            |_| true,
        );
        let seen = live.clone();
        subscribe::<events::HistoryChanged>(&engine, "history.changed", move |data| {
            seen.events.borrow_mut().push(match data {
                Ok(batch) => json!({"profile": batch.profile, "changes": batch.changes.len()}),
                Err(error) => json!({"error": format!("{error:?}")}),
            })
        });
        expect::<api::history::Add>(
            &live,
            "history.add adds one page (typed)",
            &dir,
            "ac_history_add",
            api::history::AddArgs {
                profile: dir.clone(),
                pages: vec![api::history::AddArgsPagesItem {
                    u: "https://example.com/".into(),
                    t: "Example".into(),
                    v: vec![Number(1_700_000_000_000.0)],
                }],
                tolerance_ms: Number(0.0),
            },
            |r| r.added == Number(1.0),
        );
        expect::<api::bookmarks::Tree>(
            &live,
            "an unknown profile gives an engine error",
            "/nonexistent",
            "ac_bookmarks_tree",
            Profile {
                profile: "/nonexistent".into(),
            },
            |_| false,
        );
        let missing = ria_engine_sys::missing_calls();
        record(
            &live,
            "all 106 symbols resolve",
            missing.is_empty(),
            format!("missing: {missing:?}"),
        );

        // The zoom event arrives on a drain after its reply; wait for it, then quit.
        let watch = live.clone();
        let engine_object = engine_object.clone();
        poll(
            Rc::new(move |waited: Duration| {
                let done = *watch.outstanding.borrow() == 0;
                let seen = !watch.events.borrow().is_empty();
                if (done && seen) || waited >= Duration::from_secs(60) {
                    if !*watch.finished.borrow() {
                        *watch.finished.borrow_mut() = true;
                        record(
                            &watch,
                            "a history.changed event arrives and parses (typed)",
                            seen && watch.events.borrow()[0]["profile"].is_string(),
                            format!("{:?}", watch.events.borrow()),
                        );
                        for (name, ok, detail) in watch.checks.borrow().iter() {
                            println!(
                                "{}",
                                json!({"live": "check", "name": name, "pass": ok, "detail": detail})
                            );
                        }
                        let _: () = unsafe { msg_send![&*engine_object, quit] };
                    }
                    return true;
                }
                false
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

    fn expect<C: Call + 'static>(
        live: &Rc<Live>,
        name: &'static str,
        dir: &str,
        symbol: &str,
        args: C::Args,
        pass: fn(&C::Reply) -> bool,
    ) where
        C::Reply: std::fmt::Debug,
    {
        *live.outstanding.borrow_mut() += 1;
        let live = live.clone();
        let engine = live.engine.clone();
        call_symbol::<C>(&engine, symbol, dir, false, &args, move |result| {
            // The unknown-profile check expects an engine error: `pass` is `|_| false` and the error is the pass.
            let ok = match &result {
                Ok(reply) => pass(reply),
                Err(ApiError::Call(ria_engine::call::CallError::Engine(_))) => {
                    name.contains("error")
                }
                Err(_) => false,
            };
            record(&live, name, ok, format!("{result:?}"));
            *live.outstanding.borrow_mut() -= 1;
        });
    }
}
