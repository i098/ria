// Ria original, no Ardia source.

//! Run on a Mac: `cargo test --manifest-path rust/Cargo.toml -p ria-host-mac --test live_web_view`.
//!
//! It composes the state of an `ArdiaWebView`, registers it in `LiveViews`, records one `NoteEvent` and calls
//! `ChromiumTabView::make_page`. It needs no engine, shows no window and does not activate the app.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("live_web_view: needs macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::MainThreadMarker;
    use ria_host_mac::tabs::ardia_web_view::{LiveViews, note_event, recent_events};
    use ria_host_mac::tabs::chromium_tab_view::ChromiumTabView;
    use serde_json::{Map, json};

    let mtm = MainThreadMarker::new().expect("main thread");
    let live = LiveViews::default();
    assert!(live.all().is_empty());

    let page = objc2::rc::autoreleasepool(|_| ChromiumTabView::make_page(mtm, &live));
    assert_eq!(live.all().len(), 1, "make_page registers the view");
    assert!(
        page.state().borrow().visible,
        "state composes with Ardia defaults"
    );
    assert_eq!(page.state().borrow().tab_index, -1);

    let mut payload = Map::new();
    payload.insert(
        "url".into(),
        json!(format!("otpauth://totp/{}", "x".repeat(300))),
    );
    payload.insert("long".into(), json!("é".repeat(300)));
    note_event(mtm, 7, "navigation", payload);
    let events = recent_events(mtm, 7);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["name"], "navigation");
    assert_eq!(
        events[0]["payload"]["url"], "otpauth:",
        "otpauth values are cut at the scheme"
    );
    assert_eq!(
        events[0]["payload"]["long"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        200
    );
    assert!(recent_events(mtm, 8).is_empty());

    for n in 0..100 {
        note_event(mtm, 7, &format!("e{n}"), Map::new());
    }
    let events = recent_events(mtm, 7);
    assert_eq!(events.len(), 80, "the ring keeps 80 entries");
    assert_eq!(events[0]["name"], "e20");

    objc2::rc::autoreleasepool(|_| drop(page));
    assert!(live.all().is_empty(), "the registry holds views weakly");
    println!("live_web_view: ok");
}
