// Ported from ardia packages/ardiacore/ios/ArdiaEarlyWindow.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// The tab delegate also uses ArdiaLaunchTabHolder.mm.

use std::cell::RefCell;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{
    AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, class, define_class, msg_send,
};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSColor, NSImage, NSImageView, NSScreen, NSView,
    NSWindow, NSWindowDidResizeNotification,
};
use objc2_foundation::{
    NSData, NSNotification, NSNotificationCenter, NSObjectProtocol, NSPoint, NSRect, NSSize,
    NSString,
};

use crate::core::bindings::{Profile, Tab, Window};

use super::early_window_layouts::{Layout, page_rect};
use super::host_launch::{LaunchEvent, Observer};

const RESIZE: NSAutoresizingMaskOptions =
    NSAutoresizingMaskOptions::ViewWidthSizable.union(NSAutoresizingMaskOptions::ViewHeightSizable);

pub(super) struct BrowserWindow {
    _core: Retained<Window>,
    _tab: Retained<Tab>,
    _delegate: Retained<TabDelegate>,
}

impl BrowserWindow {
    pub fn open(
        profile: &Profile,
        url: &str,
        layout: Option<Layout>,
        show: bool,
        inactive: bool,
        observer: &Observer,
    ) -> Self {
        let mtm = MainThreadMarker::new().expect("open launch window on the main thread");
        let rect = layout.as_ref().map_or_else(
            || frame([0.0, 0.0, 1360.0, 860.0]),
            |layout| frame(layout.frame),
        );
        // SAFETY: the engine loaded the class and all AppKit calls run on the main thread.
        let core = unsafe {
            Window::init_with_content_rect(msg_send![class!(ArdiaCoreWindow), alloc], rect)
        };
        let window = core.window();
        window.setTitle(&NSString::from_str("Ria"));
        window.setMinSize(NSSize::new(720.0, 460.0));
        if layout.is_none() {
            window.center();
        } else if let Some(screen) = window.screen().or_else(|| NSScreen::mainScreen(mtm)) {
            window.setFrame_display(
                window.constrainFrameRect_toScreen(rect, Some(&screen)),
                false,
            );
        }
        let host = core.host_view();
        core.prepare_profile(profile);
        core.set_active_profile(Some(profile));
        if layout.is_some() {
            profile.warm_spare_renderer();
        }
        let tab = core.open_tab(&NSString::from_str(url), profile, true);
        let delegate = TabDelegate::new(window.clone(), observer.clone());
        // SAFETY: the delegate implements the optional callbacks we use and stays owned by this window.
        unsafe { tab.set_delegate(Some(&delegate)) };
        core.activate_tab(&tab);
        let page = tab.view();
        if let Some(page) = &page {
            page.removeFromSuperview();
        }
        let bounds = host.bounds();
        let (parent, page_bounds) = layout
            .as_ref()
            .and_then(|layout| {
                let card = page_rect(bounds.size.width, bounds.size.height, layout.card)?;
                let rect = page_rect(bounds.size.width, bounds.size.height, layout.page)?;
                let cover = NSView::initWithFrame(NSView::alloc(mtm), frame(card));
                cover.setWantsLayer(true);
                if let Some(layer) = cover.layer() {
                    layer.setMasksToBounds(true);
                    // SAFETY: CALayer takes a CGFloat corner radius on macOS.
                    let _: () =
                        unsafe { msg_send![&*layer, setCornerRadius: layout.radius.min(64.0)] };
                    if let Some([r, g, b, a]) = layout.page_color {
                        let color = NSColor::colorWithSRGBRed_green_blue_alpha(
                            r.clamp(0.0, 1.0),
                            g.clamp(0.0, 1.0),
                            b.clamp(0.0, 1.0),
                            a.clamp(0.0, 1.0),
                        )
                        .CGColor();
                        // SAFETY: CALayer takes a borrowed CGColor and retains it as needed.
                        let _: () = unsafe { msg_send![&*layer, setBackgroundColor: &*color] };
                    }
                }
                cover.setAutoresizingMask(RESIZE);
                host.addSubview(&cover);
                Some((
                    cover,
                    frame([rect[0] - card[0], rect[1] - card[1], rect[2], rect[3]]),
                ))
            })
            .unwrap_or((host.clone(), bounds));
        if let Some(page) = &page {
            page.setFrame(page_bounds);
            page.setAutoresizingMask(RESIZE);
            page.setHidden(false);
            tab.note_in_shown_slot();
            parent.addSubview(page);
        }
        if tab.loading()
            && let Some(layout) = &layout
            && let Some([width, height]) = layout.picture_size
            && let Some(bytes) = &layout.picture
            && (window.frame().size.width - width).abs() < 0.5
            && (window.frame().size.height - height).abs() < 0.5
            && let Some(image) = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(bytes))
        {
            let picture = NSImageView::initWithFrame(NSImageView::alloc(mtm), bounds);
            picture.setImage(Some(&image));
            host.addSubview(&picture);
            delegate.ivars().picture.replace(Some(picture));
            let weak = Weak::new(&*delegate);
            let resize = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
                if let Some(delegate) = weak.load() {
                    delegate.drop_picture();
                }
            });
            let center = NSNotificationCenter::defaultCenter();
            let token = unsafe {
                center.addObserverForName_object_queue_usingBlock(
                    Some(NSWindowDidResizeNotification),
                    Some(&window),
                    None,
                    &resize,
                )
            };
            delegate.ivars().resize_observer.replace(Some(token));
            let release = delegate.clone();
            super::host_launch::after(std::time::Duration::from_secs(5), move || {
                release.drop_picture()
            });
        }
        if show {
            if inactive {
                window.setLevel(-1_000_000);
                core.show_inactive();
            } else {
                window.makeKeyAndOrderFront(None);
            }
        }
        observer(LaunchEvent::WindowOpened {
            windows: NSApplication::sharedApplication(mtm).windows().count(),
            tabs: core.tabs_for_profile(profile).count(),
        });
        let title = tab.title();
        if title.length() != 0 {
            delegate.report_title(&title);
        }
        Self {
            _core: core,
            _tab: tab,
            _delegate: delegate,
        }
    }
}

fn frame([x, y, width, height]: [f64; 4]) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

struct TabIvars {
    window: Retained<NSWindow>,
    observer: Observer,
    picture: RefCell<Option<Retained<NSImageView>>>,
    resize_observer: RefCell<Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "RiaLaunchTabDelegate"]
    #[ivars = TabIvars]
    struct TabDelegate;

    impl TabDelegate {
        #[unsafe(method(tabDidChangeTitle:))]
        fn title_changed(&self, tab: &Tab) {
            self.report_title(&tab.title());
        }

        #[unsafe(method(tabDidChangeLoading:))]
        fn loading_changed(&self, tab: &Tab) {
            if !tab.loading() {
                self.drop_picture();
            }
        }

        #[unsafe(method(tabWillClose:))]
        fn tab_will_close(&self, _tab: &Tab) {
            self.drop_picture();
        }
    }
);

impl TabDelegate {
    fn new(window: Retained<NSWindow>, observer: Observer) -> Retained<Self> {
        let this = Self::alloc().set_ivars(TabIvars {
            window,
            observer,
            picture: RefCell::default(),
            resize_observer: RefCell::default(),
        });
        // SAFETY: ivars are set and NSObject's initializer takes no arguments.
        unsafe { msg_send![super(this), init] }
    }

    fn drop_picture(&self) {
        if let Some(token) = self.ivars().resize_observer.take() {
            unsafe { NSNotificationCenter::defaultCenter().removeObserver((*token).as_ref()) };
        }
        if let Some(picture) = self.ivars().picture.take() {
            picture.removeFromSuperview();
        }
    }

    fn report_title(&self, title: &NSString) {
        let ivars = self.ivars();
        ivars.window.setTitle(title);
        (ivars.observer)(LaunchEvent::TabTitle(title.to_string()));
    }
}
