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
    use std::time::Duration;

    use block2::RcBlock;
    use objc2::{MainThreadMarker, rc::Retained};
    use objc2_app_kit::{
        NSApplication, NSEvent, NSEventModifierFlags, NSMenu, NSMenuItem,
        NSUserInterfaceItemIdentification, NSWindowButton, NSWindowDidEnterFullScreenNotification,
    };
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSPoint, ns_string};
    use ria_engine::call::main_queue;
    use ria_host_mac::core::bindings::{Engine, Profile, Tab, Window};
    use ria_host_mac::core::delegate::{EngineDelegate, Hooks};
    use ria_host_mac::core::host::State;
    use ria_host_mac::windows::ardia_reserved_keys::is_reserved_binding;
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

    fn check_windows(profile: &Profile) {
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApplication::sharedApplication(mtm);
        let menu = NSMenu::new(mtm);
        let item = NSMenuItem::new(mtm);
        item.setIdentifier(Some(ns_string!("ardia.command.closeTab")));
        item.setKeyEquivalent(ns_string!("w"));
        item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        menu.addItem(&item);
        let previous_menu = app.mainMenu();
        app.setMainMenu(Some(&menu));
        assert!(is_reserved_binding(
            &app,
            ns_string!("W"),
            ns_string!(""),
            NSEventModifierFlags::Command
        ));
        assert!(!is_reserved_binding(
            &app,
            ns_string!("w"),
            ns_string!(""),
            NSEventModifierFlags::Shift
        ));
        item.setHidden(true);
        assert!(!is_reserved_binding(
            &app,
            ns_string!("w"),
            ns_string!(""),
            NSEventModifierFlags::Command
        ));
        app.setMainMenu(previous_menu.as_deref());

        let controllers = Controllers::new(mtm, true, false, |_| false, |_, _| false);
        let normal = controllers.make_normal();
        let chrome = controllers.make_chrome(profile);
        let token = chrome.chrome_token().unwrap();
        assert_eq!(chrome.kind(), WindowKind::AwaitingApp);
        assert!(Rc::ptr_eq(
            &controllers.claim_chrome(token).unwrap(),
            &chrome
        ));
        assert_eq!(chrome.kind(), WindowKind::Normal);
        assert!(controllers.claim_chrome(token).is_none());
        let stray = controllers.make_stray(profile);
        assert_eq!(stray.kind(), WindowKind::Stray);
        assert!(controllers.user_sees(&stray.core.window()));
        let standalone = controllers.standalone(profile);
        assert_eq!(standalone.kind(), WindowKind::Standalone);
        assert!(standalone.core.internal());
        assert!(Rc::ptr_eq(&standalone, &controllers.standalone(profile)));
        assert_eq!(controllers.all().len(), 4);

        let window = normal.core.window();
        let lights = normal.traffic_lights.as_ref().unwrap();
        let before = window.frame();
        assert!(window.titlebarAppearsTransparent());
        let placement = Placement {
            center: Some(NSPoint::new(50., 27.)),
            trailing: false,
        };
        lights.set_placement(placement);
        let close = window
            .standardWindowButton(NSWindowButton::CloseButton)
            .unwrap();
        let frame = ria_host_mac::windows::ardia_traffic_lights::frame_view(&window).unwrap();
        let rect = close.convertRect_toView(close.bounds(), Some(&frame));
        assert!((rect.origin.x + rect.size.width / 2. - 50.).abs() < 1.);
        assert!(controllers.user_sees(&window));
        let enter_events = Rc::new(Cell::new(0));
        let enter = {
            let events = enter_events.clone();
            RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| events.set(events.get() + 1))
        };
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: The block reads no notification data and filters for this live window.
        let observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWindowDidEnterFullScreenNotification),
                Some(&window),
                None,
                &enter,
            )
        };
        let observer: Retained<objc2::runtime::AnyObject> = observer.into();
        normal
            .fullscreen
            .act_window_full_screen(true, Duration::ZERO, None);
        // SAFETY: This token was registered with this notification center above.
        unsafe { center.removeObserver(&observer) };
        assert_eq!(enter_events.get(), 1);
        assert!(normal.core.acted_full_screen() && normal.fullscreen.is_full_screen());
        assert!(!normal.fullscreen.transitioning());
        normal
            .fullscreen
            .act_window_full_screen(false, Duration::ZERO, None);
        assert!(!normal.core.acted_full_screen() && !normal.fullscreen.is_full_screen());
        assert_eq!(window.frame(), before);
        assert!(!normal.fullscreen.transitioning());
        for controller in [&normal, &chrome, &stray, &standalone] {
            controller.core.close();
        }
        println!(
            "live_windows: reserved bindings, controller claims, native layout, and fullscreen transition checked"
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
                    check_windows(&profile.expect("Default profile"));
                    main_queue::post(Box::new(|| {
                        let _: () = Engine::shared().unwrap().quit();
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
