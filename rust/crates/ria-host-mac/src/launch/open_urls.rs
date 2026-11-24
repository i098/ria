// Ported from ardia packages/shell/ios/OpenURLInbox.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// The event entries also use apps/browser/macos/Ardia-macOS/AppDelegate.swift and
// packages/shell/ios/ShellApp.swift.

//! URLs that other macOS apps send, and the Dock reopen event.
//!
//! Changes from Ardia:
//! - Ardia gets both events from its Swift `AppDelegate`, which `ArdiaHost.run` installs (`AppDelegate.swift:36-38`, `54-56`).
//!   The Ria engine takes no application delegate and the application has none, so AppKit calls neither delegate
//!   method. [`install`] registers Apple Event handlers for `GURL` and `rapp` instead (checked on the Mac: the events
//!   reach them and not the delegate methods).
//! - Ardia keeps the sender and the pointer position of each batch (`ArdiaHost.senderOfOpenedURL`, `returnTo`).
//!   Ria has no handoff feature, so the inbox keeps only the URL strings, in arrival order.
//! - Ardia sends the reopen event to the JS window manager when no browser window is shown. Ria opens a new
//!   browser window, because it has no JS window manager.

use std::cell::RefCell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager};

/// `OpenURLInbox.pending`: URLs that arrived before they could be delivered.
#[derive(Default)]
pub(super) struct Inbox(RefCell<Vec<String>>);

impl Inbox {
    /// `OpenURLInbox.receive`. Older URLs go first, so a new batch never passes a queued one.
    pub fn receive(&self, urls: Vec<String>, deliver: impl Fn(&[String]) -> bool) {
        if urls.is_empty() {
            return;
        }
        let mut pending = self.0.borrow_mut();
        pending.extend(urls);
        if deliver(&pending) {
            pending.clear();
        }
    }

    /// `OpenURLInbox.flush`.
    pub fn flush(&self, deliver: impl Fn(&[String]) -> bool) {
        let mut pending = self.0.borrow_mut();
        if !pending.is_empty() && deliver(&pending) {
            pending.clear();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
}

/// What the delegate methods call. Both run on the main thread.
pub(super) struct Entry {
    pub urls: Box<dyn Fn(Vec<String>)>,
    pub reopen: Box<dyn Fn(bool)>,
}

thread_local! {
    static ENTRY: RefCell<Option<Rc<Entry>>> = const { RefCell::new(None) };
}

pub(super) fn set_entry(entry: Option<Rc<Entry>>) {
    ENTRY.with(|slot| *slot.borrow_mut() = entry);
}

fn entry() -> Option<Rc<Entry>> {
    ENTRY.with(|slot| slot.borrow().clone())
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaOpenEvents"]
    struct OpenEvents;

    impl OpenEvents {
        /// `kAEGetURL`.
        #[unsafe(method(handleURL:withReplyEvent:))]
        fn handle_url(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            let url: Option<Retained<NSAppleEventDescriptor>> =
                unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT] };
            if let Some(url) = url.and_then(|url| url.stringValue())
                && let Some(entry) = entry()
            {
                let url = url.to_string();
                if is_web_url(&url) {
                    (entry.urls)(vec![url]);
                } else {
                    eprintln!("[ria] dropped URL with scheme {:?}", scheme(&url));
                }
            }
        }

        /// `kAEReopenApplication`: the Dock icon was clicked.
        #[unsafe(method(handleReopen:withReplyEvent:))]
        fn handle_reopen(&self, _event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            if let Some(entry) = entry() {
                (entry.reopen)(MainThreadMarker::new().is_some_and(has_visible_windows));
            }
        }
    }
);

fn scheme(url: &str) -> &str {
    url.split_once(':').map_or("", |(scheme, _)| scheme)
}

fn is_web_url(url: &str) -> bool {
    let scheme = scheme(url);
    scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
}

const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

fn has_visible_windows(mtm: MainThreadMarker) -> bool {
    NSApplication::sharedApplication(mtm)
        .windows()
        .iter()
        .any(|window| window.isVisible())
}

thread_local! {
    static HANDLER: RefCell<Option<Retained<OpenEvents>>> = const { RefCell::new(None) };
}

/// Register the Apple Event handlers for URLs and Dock reopen. `NSApplication` does not call
/// `application:openURLs:` or `applicationShouldHandleReopen:hasVisibleWindows:` here, because the engine has no
/// application delegate; the events arrive in these handlers only. AppKit sets its own handlers while it finishes
/// launching, so call this again after that.
pub(super) fn install(mtm: MainThreadMarker) {
    let handler = HANDLER.with(|slot| {
        slot.borrow_mut()
            .get_or_insert_with(|| unsafe { msg_send![OpenEvents::alloc(mtm), init] })
            .clone()
    });
    let manager = NSAppleEventManager::sharedAppleEventManager();
    for (class, id, selector) in [
        (*b"GURL", *b"GURL", sel!(handleURL:withReplyEvent:)),
        (*b"aevt", *b"rapp", sel!(handleReopen:withReplyEvent:)),
    ] {
        let (class, id) = (u32::from_be_bytes(class), u32::from_be_bytes(id));
        // SAFETY: the selector takes the event and the reply, and the manager keeps no strong reference.
        let _: () = unsafe {
            msg_send![&*manager, setEventHandler: &*handler, andSelector: selector, forEventClass: class, andEventID: id]
        };
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use objc2::ClassType;
    use objc2_foundation::NSString;

    use super::*;

    #[test]
    fn urls_wait_until_delivery_works_and_keep_order() {
        let inbox = Inbox::default();
        let sent = RefCell::new(Vec::new());
        let ready = Cell::new(false);
        let deliver = |urls: &[String]| {
            ready.get() && {
                sent.borrow_mut().push(urls.to_vec());
                true
            }
        };
        inbox.receive(vec!["https://a.test/".into()], deliver);
        inbox.receive(vec!["https://b.test/".into()], deliver);
        assert!(sent.borrow().is_empty());
        ready.set(true);
        inbox.receive(vec!["https://c.test/".into()], deliver);
        assert_eq!(
            *sent.borrow(),
            [["https://a.test/", "https://b.test/", "https://c.test/"]]
        );
        assert!(inbox.is_empty());
        inbox.flush(deliver);
        assert_eq!(sent.borrow().len(), 1);
    }

    #[test]
    fn apple_events_reach_the_entry() {
        // SAFETY: the test thread is not the main thread, so the handlers see no windows.
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let urls = Rc::new(RefCell::new(Vec::new()));
        let reopens = Rc::new(RefCell::new(Vec::new()));
        set_entry(Some(Rc::new(Entry {
            urls: Box::new({
                let urls = urls.clone();
                move |batch| urls.borrow_mut().extend(batch)
            }),
            reopen: Box::new({
                let reopens = reopens.clone();
                move |visible| reopens.borrow_mut().push(visible)
            }),
        })));
        let handler: Retained<OpenEvents> = unsafe { msg_send![OpenEvents::alloc(mtm), init] };
        let event = |class: &[u8; 4], id: &[u8; 4]| -> Retained<NSAppleEventDescriptor> {
            let (class, id) = (u32::from_be_bytes(*class), u32::from_be_bytes(*id));
            let target = NSAppleEventDescriptor::nullDescriptor();
            unsafe {
                msg_send![
                    NSAppleEventDescriptor::class(),
                    appleEventWithEventClass: class,
                    eventID: id,
                    targetDescriptor: &*target,
                    returnID: -1i16,
                    transactionID: 0i32
                ]
            }
        };
        let send_url = |url: &str| {
            let get_url = event(b"GURL", b"GURL");
            let text = NSAppleEventDescriptor::descriptorWithString(&NSString::from_str(url));
            let _: () = unsafe {
                msg_send![&*get_url, setParamDescriptor: &*text, forKeyword: DIRECT_OBJECT]
            };
            let reply = NSAppleEventDescriptor::nullDescriptor();
            let _: () =
                unsafe { msg_send![&*handler, handleURL: &*get_url, withReplyEvent: &*reply] };
        };
        send_url("file:///etc/passwd");
        send_url("javascript:alert(1)");
        send_url("HTTPS://example.test/a");
        let reply = NSAppleEventDescriptor::nullDescriptor();
        let rapp = event(b"aevt", b"rapp");
        let _: () = unsafe { msg_send![&*handler, handleReopen: &*rapp, withReplyEvent: &*reply] };
        set_entry(None);
        assert_eq!(*urls.borrow(), ["HTTPS://example.test/a"]);
        assert_eq!(*reopens.borrow(), [false]);
    }
}
