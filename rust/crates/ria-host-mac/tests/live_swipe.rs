// Ria original, no Ardia source.

//! Run from the repository root on a Mac:
//! `ENGINE_DIR=/path/to/framework-parent cargo test --manifest-path rust/Cargo.toml -p ria-host-mac --test live_swipe`.
//!
//! Swipe Check L uses a temporary Chromium profile without activating the app, and shows no window.
//! It drives a swipe area through `simulate_in_window` with synthetic trackpad and wheel events: a back swipe, a
//! native pager drag that settles on the next page, pager switch and step calls, and removal of the area.
//! It also exercises the colour and data-URL helpers with fixture inputs. The check skips on Linux.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("live_swipe: needs macOS and the prebuilt engine");
}

#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("RIA_SWIPE_DATA").is_some() {
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

    use objc2::rc::Retained;
    use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSBackingStoreType, NSEvent, NSImage, NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use ria_engine::call::main_queue;
    use ria_host_mac::core::bindings::{Engine, Profile, Tab, Window};
    use ria_host_mac::core::delegate::{EngineDelegate, Hooks};
    use ria_host_mac::core::host::State;
    use ria_host_mac::swipe::ardia_swipe::{
        SwipeHooks, perform_haptic, set_hooks, simulate_in_window, targets,
    };
    use ria_host_mac::swipe::nsimage_data_url::{image_from_data_url, image_png_data_url};
    use ria_host_mac::swipe::pager::Pager;
    use ria_host_mac::swipe::pager_config::PagerConfig;
    use ria_host_mac::swipe::rgba::Rgba;
    use ria_host_mac::swipe::swipe_area::SwipeArea;
    use serde_json::{Value, json};

    const FRAMEWORK: &str = "Chromium Framework.framework";
    const PAGER: &str = "live-swipe";

    pub fn parent() {
        let framework = PathBuf::from(
            std::env::var_os("ENGINE_DIR").expect("set ENGINE_DIR to the framework's parent"),
        )
        .join(FRAMEWORK);
        let root = std::env::temp_dir().join(format!("ria-swipe-live-{}", std::process::id()));
        let contents = root.join("Live.app/Contents");
        let exe = contents.join("MacOS/live_swipe");
        let data = root.join("data");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::create_dir(contents.join("Frameworks")).unwrap();
        std::fs::create_dir(&data).unwrap();
        std::fs::write(
            contents.join("Info.plist"),
            br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>live_swipe</string>
<key>CFBundleIdentifier</key><string>sh.ria.swipe-live</string>
<key>CFBundleName</key><string>Ria Swipe Live</string>
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
        let status = Command::new(&exe).env("RIA_SWIPE_DATA", data).status();
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
        let data = std::env::var("RIA_SWIPE_DATA").unwrap();
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
        let hooks = Rc::new(LiveHooks::default());
        let delegate = EngineDelegate::new(
            hooks.clone(),
            State::new(),
            Some("/* swipe check */".into()),
        );
        // SAFETY: This is the main thread and the framework has been loaded.
        let exit = unsafe { delegate.run(&argv) };
        assert_eq!(exit, 0);
        assert!(hooks.shutdown.get());
        println!("live_swipe: engine shut down cleanly");
    }

    type Seen = Rc<RefCell<Vec<(&'static str, Value)>>>;

    struct Check {
        window: Retained<NSWindow>,
        area: Retained<SwipeArea>,
        seen: Seen,
        wakes: Rc<Cell<usize>>,
    }

    #[derive(Default)]
    struct WakeCount(Rc<Cell<usize>>);

    impl SwipeHooks for WakeCount {
        fn root_view(&self, _: &NSWindow) -> Option<Retained<NSView>> {
            None
        }
        fn wake_neighbour_pages(&self, _: &NSWindow) {
            self.0.set(self.0.get() + 1);
        }
        fn wake_neighbour_pages_for(&self, _: &NSWindow, _: f64, _: bool) {
            self.0.set(self.0.get() + 1);
        }
        fn release_neighbour_pages(&self, _: &NSWindow) {}
    }

    /// Runs `then` once `ready` holds, from the main queue, so that the queued events and the spring can run.
    fn poll(
        what: &'static str,
        deadline: Instant,
        ready: Rc<dyn Fn() -> bool>,
        then: Box<dyn FnOnce()>,
    ) {
        if ready() {
            return then();
        }
        assert!(Instant::now() < deadline, "{what}");
        main_queue::after(
            Duration::from_millis(20),
            Box::new(move || poll(what, deadline, ready, then)),
        );
    }

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }

    fn phases(seen: &Seen, name: &str) -> Vec<String> {
        seen.borrow()
            .iter()
            .filter(|(event, _)| *event == name)
            .map(|(_, value)| value["phase"].as_str().unwrap_or_default().to_owned())
            .collect()
    }

    fn start(mtm: MainThreadMarker) {
        let wakes = Rc::new(Cell::new(0));
        set_hooks(Rc::new(WakeCount(wakes.clone())));
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0));
        // SAFETY: A borderless window that is never ordered in; `defer: false` gives it a window number.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: The check owns the window and closes it at the end.
        unsafe { window.setReleasedWhenClosed(false) };
        let area = SwipeArea::new(
            mtm,
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0)),
        );
        area.ivars().can_swipe_back.set(true);
        let seen: Seen = Rc::default();
        let sink = seen.clone();
        area.set_events(Some(Rc::new(move |name, value| {
            sink.borrow_mut().push((name, value))
        })));
        window.contentView().unwrap().addSubview(&area);
        assert_eq!(targets().len(), 1, "moving into a window adds the target");
        let check = Rc::new(Check {
            window,
            area,
            seen,
            wakes,
        });
        let gesture = vec![
            json!({"phase": "began", "dx": 0}),
            json!({"phase": "changed", "dx": 30}),
            json!({"phase": "changed", "dx": 30}),
            json!({"phase": "ended", "dx": 0}),
        ];
        let next = check.clone();
        simulate_in_window(
            &check.window,
            NSPoint::new(200.0, 150.0),
            gesture,
            true,
            move |result| back_swiped(next, result),
        );
    }

    fn back_swiped(check: Rc<Check>, result: Value) {
        assert_eq!(result["where"]["target"], true, "{result}");
        let events = result["events"].as_array().unwrap();
        assert!(
            events.iter().all(|event| event.get("error").is_none()),
            "{result}"
        );
        assert_eq!(events[1]["swallowed"], true, "{result}");
        let seen = check.seen.clone();
        let ready = Rc::new(move || phases(&seen, "onSwipe").len() >= 3);
        poll(
            "onSwipe did not arrive",
            soon(),
            ready,
            Box::new(move || {
                assert_eq!(
                    phases(&check.seen, "onSwipe"),
                    ["began", "changed", "ended"]
                );
                assert!(
                    check
                        .seen
                        .borrow()
                        .iter()
                        .all(|(_, value)| value["direction"] == "back")
                );
                check.seen.borrow_mut().clear();
                drag_pager(check);
            }),
        );
    }

    fn drag_pager(check: Rc<Check>) {
        check.area.ivars().is_pager.set(true);
        check.area.ivars().can_swipe_forward.set(true);
        let config = PagerConfig {
            key: PAGER.into(),
            count: 3,
            width: 200.0,
            ..PagerConfig::default()
        };
        check.area.configure_pager(Some(&config));
        assert_eq!(Pager::snapshot(PAGER).unwrap()["selected"], 0);
        let drag = vec![
            json!({"phase": "began", "dx": 0}),
            json!({"phase": "changed", "dx": -250}),
            json!({"phase": "ended", "dx": 0}),
        ];
        let next = check.clone();
        simulate_in_window(
            &check.window,
            NSPoint::new(200.0, 150.0),
            drag,
            true,
            move |result| {
                assert_eq!(result["events"][1]["swallowed"], true, "{result}");
                settled(next, 1, Box::new(switch_and_step));
            },
        );
    }

    fn settled(check: Rc<Check>, page: i64, then: Box<dyn FnOnce(Rc<Check>)>) {
        let check_seen = check.clone();
        let ready = Rc::new(move || {
            Pager::snapshot(PAGER)
                .is_some_and(|state| state["phase"] == "idle" && state["selected"] == page)
                && phases(&check_seen.seen, "onPagerState").contains(&"settled".to_owned())
        });
        poll(
            "the pager did not settle",
            soon(),
            ready,
            Box::new(move || then(check)),
        );
    }

    fn switch_and_step(check: Rc<Check>) {
        assert!(phases(&check.seen, "onPagerState").contains(&"settled".to_owned()));
        assert!(
            check.wakes.get() > 0,
            "the pager wakes the neighbour pages of its window"
        );
        let pager = Pager::existing(PAGER).unwrap();
        check.seen.borrow_mut().clear();
        assert!(pager.borrow_mut().switch_to(2, None));
        settled(
            check,
            2,
            Box::new(|check| {
                check.seen.borrow_mut().clear();
                assert!(
                    Pager::existing(PAGER)
                        .unwrap()
                        .borrow_mut()
                        .step(1, true, None)
                );
                settled(check, 0, Box::new(wheel));
            }),
        );
    }

    fn wheel(check: Rc<Check>) {
        check.seen.borrow_mut().clear();
        let next = check.clone();
        let wheel = vec![json!({"phase": "wheel", "dx": 3, "dy": 0})];
        simulate_in_window(
            &check.window,
            NSPoint::new(200.0, 150.0),
            wheel,
            true,
            move |result| {
                assert_eq!(result["events"][0]["swallowed"], true, "{result}");
                let seen = next.seen.clone();
                let ready = Rc::new(move || phases(&seen, "onSwipe") == ["wheel"]);
                poll(
                    "the wheel page did not arrive",
                    soon(),
                    ready,
                    Box::new(move || finish(next)),
                );
            },
        );
    }

    fn finish(check: Rc<Check>) {
        check.area.removeFromSuperview();
        assert!(
            targets().is_empty(),
            "leaving the window removes the target"
        );
        check.area.configure_pager(None);
        assert!(Pager::existing(PAGER).is_none() && Pager::snapshot(PAGER).is_none());
        check.window.close();
        helpers();
        println!("live_swipe: swipe, pager settle, switch, step, wheel and helpers checked");
        let _: () = Engine::shared().unwrap().quit();
    }

    fn helpers() {
        perform_haptic("alignment");
        let rgba = Rgba::from_hex("#336699CC").unwrap();
        assert!((rgba.srgb().alphaComponent() - 0.8).abs() < 1e-6);
        assert!((rgba.display_p3().alphaComponent() - 0.8).abs() < 1e-6);
        let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(4.0, 4.0));
        let url = image_png_data_url(&image, 4.0, 2.0).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
        assert!(image_from_data_url(&url).is_some());
        assert!(image_from_data_url("data:image/png;base64,***").is_none());
        assert!(image_from_data_url("image/png;base64,").is_none());
    }

    #[derive(Default)]
    struct LiveHooks {
        shutdown: Cell<bool>,
    }

    impl Hooks for LiveHooks {
        fn engine_started(&self) {
            main_queue::post(Box::new(|| start(MainThreadMarker::new().unwrap())));
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
        fn is_reserved_binding(&self, _: &str, _: &str, _: usize) -> bool {
            false
        }
        fn is_reserved_key_event(&self, _: &NSEvent) -> bool {
            false
        }
        fn window_for_new_browser(&self, _: &Profile, _: &str) -> Option<Retained<Window>> {
            None
        }
    }
}
