// Ria original, no Ardia source.

//! Run from the repository root on a Mac:
//! `cargo test --manifest-path rust/Cargo.toml -p ria-host-mac --test live_pip`.
//!
//! Picture in Picture Check L. The check drives the window controller on a real borderless AppKit window
//! of an accessory app, with fixture inputs and a recording page. It needs no engine and brings nothing to the front.
//! It covers corners, the menu, tucking, the throw, rescue, the skip button, Back to Tab and detach.
//! The check skips on Linux.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("live_pip: needs macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    mac::run();
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use std::rc::Rc;

    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly, msg_send};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSFloatingWindowLevel,
        NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSDate, NSPoint, NSRect, NSRunLoop, NSSize};
    use ria_host_mac::launch::launch_config::LaunchConfig;
    use ria_host_mac::pip::ardia_picture_in_picture_internal::{
        Edge, PipPage, Services, set_services,
    };
    use ria_host_mac::pip::ardia_pip_controller::Controller;
    use serde_json::{Value, json};

    #[derive(Default)]
    struct Page {
        emitted: RefCell<Vec<(String, Value)>>,
        exits: RefCell<u32>,
        seeks: RefCell<Vec<f64>>,
    }

    impl PipPage for Page {
        fn browser_id(&self) -> i64 {
            7
        }
        fn emit(&self, name: &str, payload: Value) {
            self.emitted.borrow_mut().push((name.to_owned(), payload));
        }
        fn exit_picture_in_picture(&self) {
            *self.exits.borrow_mut() += 1;
        }
        fn seek_picture_in_picture_by(&self, seconds: f64) {
            self.seeks.borrow_mut().push(seconds);
        }
        fn left_app_in_picture_in_picture(&self) -> bool {
            false
        }
        fn seen_by_user(&self) -> bool {
            false
        }
    }

    fn pump(seconds: f64) {
        NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(seconds));
    }

    fn pump_until(seconds: f64, done: impl Fn() -> bool) -> bool {
        let end = std::time::Instant::now() + std::time::Duration::from_secs_f64(seconds);
        while std::time::Instant::now() < end {
            if done() {
                return true;
            }
            pump(0.02);
        }
        done()
    }

    fn layer_radius(window: &NSWindow) -> f64 {
        let layer = window
            .contentView()
            .and_then(|view| view.layer())
            .expect("a layer");
        // SAFETY: `cornerRadius` is a CALayer property.
        unsafe { msg_send![&*layer, cornerRadius] }
    }

    fn install_all_wraps_once() {
        use objc2::runtime::{AnyClass, ClassBuilder, Sel};
        use objc2::{ClassType, sel};

        let class: &AnyClass = ClassBuilder::new(c"NativeWidgetMacFramelessNSWindow", NSWindow::class())
            .expect("class name is free")
            .register();
        let imps = |class: &AnyClass| -> Vec<usize> {
            [sel!(constrainFrameRect:toScreen:), sel!(setLevel:), sel!(setCollectionBehavior:)]
                .into_iter()
                .map(|selector: Sel| {
                    class.instance_method(selector).expect("inherited").implementation() as usize
                })
                .collect()
        };
        let before = imps(class);
        ria_host_mac::pip::install_all();
        let once = imps(class);
        ria_host_mac::pip::install_all();
        let twice = imps(class);
        assert!(
            before.iter().zip(&once).all(|(a, b)| a != b),
            "the first install wraps every hooked method"
        );
        assert_eq!(once, twice, "a second install wraps nothing again");
    }

    pub fn run() {
        let mtm = MainThreadMarker::new().expect("main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        install_all_wraps_once();

        let root = std::env::temp_dir().join(format!("ria-pip-live-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let data = root.join("data");
        let launch = LaunchConfig::from_environment(
            [
                ("RIA_DATA_DIR", data.to_str().unwrap()),
                ("RIA_BACKGROUND", "1"),
            ],
            &root,
            &root,
            false,
        )
        .expect("a launch config");
        set_services(Services {
            mtm,
            launch: Rc::new(launch),
            begin_stamp: Box::new(|| json!({ "stamp": 1 })),
            onto: Box::new(|stamp, mut payload| {
                payload["stamp"] = stamp["stamp"].clone();
                payload
            }),
            root_view: Box::new(|window| window.contentView()),
            reconsider_sight: Box::new(|_| {}),
            note_pointer: Box::new(|_, _| {}),
            log_activation: Box::new(|_| {}),
        });

        let screen = objc2_app_kit::NSScreen::mainScreen(mtm).expect("a screen");
        let visible = screen.visibleFrame();
        let frame = NSRect::new(
            NSPoint::new(visible.origin.x + 200.0, visible.origin.y + 200.0),
            NSSize::new(320.0, 180.0),
        );
        // SAFETY: a borderless buffered window made on the main thread.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                NSWindowStyleMask::Borderless | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: the window is retained by `window`; AppKit must not release it on close.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setContentView(Some(&NSView::initWithFrame(NSView::alloc(mtm), frame)));
        window.setAlphaValue(0.0);
        window.setIgnoresMouseEvents(true);
        window.orderFrontRegardless();

        let controller = Controller::new(&window);
        assert!(
            Controller::for_window(&window).is_some(),
            "the controller is registered"
        );

        controller.round_corners();
        assert_eq!(
            layer_radius(&window),
            controller.corner_radius(),
            "round_corners fits the corners"
        );
        assert!(
            !window.isOpaque(),
            "the window is not opaque after rounding"
        );
        assert_eq!(
            window.level(),
            NSFloatingWindowLevel,
            "keep on top is the default"
        );

        controller.toggle_keep_on_top();
        assert!(!controller.keep_on_top() && window.level() != NSFloatingWindowLevel);
        controller.toggle_keep_on_top();

        let menu = controller.menu();
        let titles: Vec<String> = menu
            .itemArray()
            .iter()
            .map(|item| item.title().to_string())
            .collect();
        assert_eq!(
            titles,
            ["Back to Tab", "", "Keep Window on Top"],
            "menu items"
        );

        // Tuck to the right edge and come back.
        controller.stash_at(Edge::Right);
        assert_eq!(controller.stashed_edge(), Edge::Right);
        let tucked = controller.frame_stashed_at(Edge::Right);
        assert!(
            pump_until(2.0, || window.frame().origin.x == tucked.origin.x
                && window.frame().origin.y == tucked.origin.y),
            "the window slides to the tuck frame"
        );
        assert!(
            !controller.overlay().handle().isHidden(),
            "the handle shows while tucked"
        );
        controller.unstash();
        assert_eq!(controller.stashed_edge(), Edge::None);
        assert!(
            pump_until(4.0, || window.frame().origin.x + 320.0
                <= visible.origin.x + visible.size.width + 0.5),
            "the window returns inside the visible frame"
        );
        pump(0.5);
        assert!(
            pump_until(2.0, || controller.overlay().handle().isHidden()),
            "the handle hides after the return"
        );

        // Throw to a corner.
        let target = NSRect::new(
            NSPoint::new(visible.origin.x + 10.0, visible.origin.y + 10.0),
            frame.size,
        );
        controller.throw_to(target, NSPoint::new(-300.0, -300.0), None);
        assert!(
            pump_until(5.0, || window.frame().origin.x == target.origin.x
                && window.frame().origin.y == target.origin.y),
            "the throw settles at the target"
        );
        let info = controller.throw_info();
        assert!(
            info["lastThrow"]["frames"]
                .as_array()
                .is_some_and(|frames| !frames.is_empty())
        );
        assert_eq!(info["lastThrow"]["corner"], "none");

        // A window dragged out of reach comes back.
        window.setFrameOrigin(NSPoint::new(
            visible.origin.x + 100.0,
            visible.origin.y + visible.size.height + 6000.0,
        ));
        assert!(
            pump_until(5.0, || {
                let f = window.frame();
                f.origin.y + f.size.height <= visible.origin.y + visible.size.height + 1.0
                    && f.origin.y >= visible.origin.y - 1.0
            }),
            "settle brings a lost window back inside the visible frame"
        );
        assert_eq!(controller.stashed_edge(), Edge::None);
        pump(1.0);

        // The skip button and Back to Tab reach the page.
        let page = Rc::new(Page::default());
        let dynamic: Rc<dyn PipPage> = page.clone();
        controller.set_view(Some(&dynamic));
        assert!(controller.skip_by(10.0, NSPoint::new(1.0, 2.0), 1));
        assert!(
            !controller.skip_by(0.0, NSPoint::ZERO, 1),
            "a zero skip does nothing"
        );
        assert_eq!(*page.seeks.borrow(), [10.0]);
        assert_eq!(controller.skip_info()["count"], 1);
        assert_eq!(page.emitted.borrow()[0].0, "pictureInPictureSkip");
        assert_eq!(page.emitted.borrow()[0].1["stamp"], 1);
        controller.back_to_tab();
        assert_eq!(*page.exits.borrow(), 1);
        assert_eq!(page.emitted.borrow()[1].0, "activateRequest");
        assert_eq!(page.emitted.borrow()[1].1["reason"], "pictureInPicture");

        controller.detach();
        assert!(
            Controller::for_window(&window).is_none(),
            "detach unregisters the controller"
        );
        window.close();
        let _: Option<Retained<NSWindow>> = None;
        std::fs::remove_dir_all(&root).ok();
        println!("live_pip: ok");
    }
}
