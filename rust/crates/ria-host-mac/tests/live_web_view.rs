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
    println!("live_web_view: ok");
}
