// Ria original, no Ardia source.

//! Run on a Mac: `cargo test --manifest-path rust/Cargo.toml -p ria-host-mac --test live_web_view`.
//!
//! It checks that `ChromiumTabView::make_page` creates and registers an `ArdiaWebView`.
//! It needs no engine, shows no window and does not activate the app.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("live_web_view: needs macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::MainThreadMarker;
    use ria_host_mac::tabs::ardia_web_view::LiveViews;
    use ria_host_mac::tabs::chromium_tab_view::ChromiumTabView;

    let mtm = MainThreadMarker::new().expect("main thread");
    let live = LiveViews::default();
    assert!(live.all().is_empty());

    let page = objc2::rc::autoreleasepool(|_| ChromiumTabView::make_page(mtm, &live));
    assert_eq!(live.all().len(), 1, "make_page registers the view");

    objc2::rc::autoreleasepool(|_| drop(page));
    assert!(live.all().is_empty(), "the registry holds views weakly");
    neighbour_wake_repaints_warm_views(mtm);
    println!("live_web_view: ok");
}

/// `PageHooks` wakes and releases the warm neighbour views of a window (`ArdiaWebView+Painting.mm:208–235`).
/// The window is never shown.
#[cfg(target_os = "macos")]
fn neighbour_wake_repaints_warm_views(mtm: objc2::MainThreadMarker) {
    use std::rc::Rc;

    use objc2::MainThreadOnly;
    use objc2::rc::Retained;
    use objc2_app_kit::{NSBackingStoreType, NSView, NSWindow, NSWindowStyleMask};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use ria_host_mac::swipe::ardia_swipe::SwipeHooks;
    use ria_host_mac::tabs::ardia_page_hooks::{PageHooks, in_tab_page};
    use ria_host_mac::tabs::ardia_web_view::LiveViews;
    use ria_host_mac::tabs::chromium_tab_view::ChromiumTabView;

    let live = Rc::new(LiveViews::default());
    let hooks = PageHooks::new(live.clone(), |_| None, |_| {});
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, 100.0));
    // SAFETY: The window is made and used on the main thread; it is never ordered front.
    let window: Retained<NSWindow> = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Titled,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    let page = ChromiumTabView::make_page(mtm, &live);
    let child = NSView::initWithFrame(NSView::alloc(mtm), frame);
    page.addSubview(&child);
    window
        .contentView()
        .expect("content view")
        .addSubview(&page);
    page.set_visible(false);
    page.set_warm(true);
    page.set_neighbour(true);

    page.repaint(true);
    assert!(child.isHidden(), "an asleep warm neighbour does not paint");
    hooks.wake_neighbour_pages_for(&window, 1.0, false);
    assert!(!child.isHidden(), "the wake repaints the warm neighbour");
    hooks.release_neighbour_pages(&window);
    assert!(
        child.isHidden(),
        "the release of a speculative wake puts it back to sleep"
    );

    assert!(in_tab_page(&child));
    page.set_standalone(true);
    assert!(
        !in_tab_page(&child),
        "a standalone web view is not a tab page"
    );
    assert!(
        page.paint_facts().page_detached,
        "a view with no tab has no page"
    );
}
