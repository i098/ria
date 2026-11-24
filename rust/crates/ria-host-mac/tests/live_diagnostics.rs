// Ria original, no Ardia source.

//! Run from the repository root on a Mac:
//! `ENGINE_DIR=/path/to/framework-parent cargo test --manifest-path rust/Cargo.toml -p ria-host-mac --test live_diagnostics`.
//!
//! Diagnostics Check L uses a temporary Chromium profile without activating the app.
//! The check skips on Linux.
//! It blocks the main thread for 1.5 s to make one freeze record, samples the stack of a parked thread, and
//! calls field timing, pointer trace, hover probe, paint probe, pixel grid and reveal guards with fixture
//! arguments. It does not open a web page.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("live_diagnostics: needs macOS and the prebuilt engine");
}

#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("RIA_DIAG_DATA").is_some() {
        mac::child();
    } else {
        mac::parent();
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::Cell;
    use std::ffi::CString;
    use std::os::unix::thread::JoinHandleExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSBackingStoreType, NSEvent, NSView, NSWindow, NSWindowStyleMask};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use ria_engine::call::main_queue;
    use ria_host_mac::core::bindings::{Engine, Profile, Tab, Window};
    use ria_host_mac::core::delegate::{EngineDelegate, Hooks};
    use ria_host_mac::core::host::State;
    use ria_host_mac::diagnostics::ardia_freeze_capture::{self as capture, Config};
    use ria_host_mac::diagnostics::ardia_hover_probe::{
        HoverHost, WindowRole, hover_probe_dev, hover_probe_line, watch_hover,
    };
    use ria_host_mac::diagnostics::ardia_paint_probe::PaintProbe;
    use ria_host_mac::diagnostics::ardia_pixel_grid::PixelGrid;
    use ria_host_mac::diagnostics::ardia_pointer_trace::{Entry, Trace};
    use ria_host_mac::diagnostics::ardia_reveal_guards::{self as guards, RevealHooks};
    use ria_host_mac::diagnostics::{
        ardia_field_timing as field, ardia_freeze_watch as freeze, ardia_stack_sample as stack,
    };
    use ria_host_mac::windows::ardia_window_controller::Controllers;
    use serde_json::Value;

    const FRAMEWORK: &str = "Chromium Framework.framework";

    pub fn parent() {
        let framework = PathBuf::from(
            std::env::var_os("ENGINE_DIR").expect("set ENGINE_DIR to the framework's parent"),
        )
        .join(FRAMEWORK);
        let root = std::env::temp_dir().join(format!("ria-diag-live-{}", std::process::id()));
        let contents = root.join("Live.app/Contents");
        let exe = contents.join("MacOS/live_diagnostics");
        let data = root.join("data");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::create_dir(contents.join("Frameworks")).unwrap();
        std::fs::create_dir(&data).unwrap();
        std::fs::write(
            contents.join("Info.plist"),
            br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>live_diagnostics</string>
<key>CFBundleIdentifier</key><string>sh.ria.diagnostics-live</string>
<key>CFBundleName</key><string>Ria Diagnostics Live</string>
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
        let status = Command::new(&exe).env("RIA_DIAG_DATA", data).status();
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
        let data = std::env::var("RIA_DIAG_DATA").unwrap();
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
        let hooks = Rc::new(DiagHooks::default());
        let delegate = EngineDelegate::new(
            hooks.clone(),
            State::new(),
            Some("/* diagnostics check */".into()),
        );
        // SAFETY: This is the main thread and the framework has been loaded.
        let exit = unsafe { delegate.run(&argv) };
        assert_eq!(exit, 0);
        assert!(hooks.shutdown.get());
        println!("live_diagnostics: engine shut down cleanly");
    }

    fn check_stack_sample() {
        let (park, parked) = std::sync::mpsc::channel::<()>();
        let sleeper = std::thread::spawn(move || {
            let _ = parked.recv();
        });
        std::thread::sleep(Duration::from_millis(50));
        let thread = stack::sampled_thread(sleeper.as_pthread_t()).expect("a sampled thread");
        assert!(thread.stack_low < thread.stack_high);
        let sample = stack::take(&thread).expect("a stack sample of the parked thread");
        assert!(sample.count > 0);
        assert!(!stack::debugger_attached());
        drop(park);
        sleeper.join().unwrap();
    }

    fn check_field_timing() {
        field::set_enabled(true, |_| {});
        field::mark("fixture", "start", 1000.0);
        field::mark_now("fixture", "view");
        let drained = field::drain();
        assert_eq!(drained.as_array().map(Vec::len), Some(2), "{drained}");
        assert_eq!(drained[0], serde_json::json!(["fixture", "start", 1000.0]));
        field::set_enabled(false, |_| {});
    }

    fn check_pointer_trace() {
        let trace = Trace::default();
        trace.record(Entry {
            at: 100.0,
            down: true,
            window: 12,
            hit: "NSView".into(),
            ..Entry::default()
        });
        assert!(trace.merge(100.0, true, "windowDragRegion"));
        assert_eq!(trace.history_lines(103.0).len(), 1);
        trace.begin_loop("pipDrag", 3.0, "e");
        trace.end_loop(3.0);
        assert_eq!(
            trace.loop_line(4.0),
            "press loop none, finished 1, longest pipDrag 0.0s"
        );
    }

    struct Guards;

    impl RevealHooks for Guards {
        fn refuses_view(&self, _: &NSView) -> bool {
            false
        }
        fn any_gap(&self) -> bool {
            false
        }
        fn swallows_ahead(&self, _: &NSEvent) -> bool {
            false
        }
    }

    struct Hover;

    impl HoverHost for Hover {
        fn background(&self) -> bool {
            true
        }
        fn is_hidden_page_window(&self, _: &NSWindow) -> bool {
            false
        }
        fn window_role(&self, _: &NSWindow) -> Option<WindowRole> {
            None
        }
        fn root(&self, _: &NSWindow) -> Option<Retained<NSView>> {
            None
        }
        fn note_focus_change(&self, _: &str, _: Option<&NSWindow>) {}
        fn describe_focus_window(&self, _: Option<&NSWindow>) -> String {
            "none".into()
        }
        fn consent_epoch(&self) -> String {
            String::new()
        }
        fn emit(&self, _: &str, _: Value) {}
    }

    fn check_views(mtm: MainThreadMarker) {
        guards::install(mtm, Rc::new(Guards));
        assert!(
            guards::guarded_accessibility_calls() > 0,
            "the engine's accessibility classes get guards"
        );
        watch_hover(Rc::new(Hover));
        assert!(hover_probe_line().starts_with("activation app "));
        // SAFETY: The window is never shown and is released when this function returns.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(200., 100.)),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: The window is owned by this function, not by the app's window list.
        unsafe { window.setReleasedWhenClosed(false) };
        assert_eq!(hover_probe_dev(&window, Some("cover:off")), "uncovered");
        let controllers = Controllers::new(mtm, true, false, |_| true);
        let paint = PaintProbe::new(controllers, |_| Vec::new(), |_, _| {}, String::new);
        paint.watch();
        assert!(paint.line().starts_with("paint last check not checked"));
        drop(PixelGrid::new(mtm, |_| false));
    }

    fn check_freeze(records: Arc<Mutex<Vec<Value>>>, data: PathBuf) {
        capture::configure(Config {
            data_directory: data.clone(),
            acted_out: true,
            consent_epoch: Box::new(|| "live".into()),
        });
        freeze::set_consent_epoch(1.0);
        let kept = records.clone();
        assert!(freeze::start(
            MainThreadMarker::new().unwrap(),
            move |record| {
                kept.lock().unwrap().push(record);
            }
        ));
        main_queue::after(
            Duration::from_millis(200),
            Box::new(|| std::thread::sleep(Duration::from_millis(1500))),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        main_queue::after(
            Duration::from_millis(300),
            Box::new(move || wait_for_freeze(records, data, deadline)),
        );
    }

    fn wait_for_freeze(records: Arc<Mutex<Vec<Value>>>, data: PathBuf, deadline: Instant) {
        let record = records.lock().unwrap().first().cloned();
        let Some(record) = record else {
            assert!(Instant::now() < deadline, "no freeze record came");
            main_queue::after(
                Duration::from_millis(50),
                Box::new(move || wait_for_freeze(records, data, deadline)),
            );
            return;
        };
        assert_eq!(record["thread"], "main", "{record}");
        assert!(record["durationMs"].as_f64().unwrap() >= 1400.0, "{record}");
        assert!(
            !record["samples"].as_array().unwrap().is_empty(),
            "{record}"
        );
        let slots = std::fs::read_to_string(data.join("freezes.txt")).unwrap();
        assert!(slots.starts_with("freeze 1 main"), "{slots}");
        let taken = freeze::take(1.0).expect("the tallies of the consent epoch");
        assert_eq!(
            taken["main"]["longestMs"].as_f64().map(|ms| ms >= 1400.0),
            Some(true)
        );
        println!(
            "live_diagnostics: freeze record, stack sample, field timing, pointer trace, hover, paint, pixel grid and reveal guards checked"
        );
        let _: () = Engine::shared().unwrap().quit();
    }

    #[derive(Default)]
    struct DiagHooks {
        shutdown: Cell<bool>,
    }

    impl Hooks for DiagHooks {
        fn engine_started(&self) {
            main_queue::post(Box::new(|| {
                let mtm = MainThreadMarker::new().unwrap();
                check_stack_sample();
                check_field_timing();
                check_pointer_trace();
                check_views(mtm);
                let data = PathBuf::from(std::env::var("RIA_DIAG_DATA").unwrap());
                check_freeze(Arc::default(), data);
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
