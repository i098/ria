// Ported from ardia packages/ardiacore/ios/ArdiaPageCover.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send};
use objc2_app_kit::{NSAnimationContext, NSAutoresizingMaskOptions, NSView};
use objc2_foundation::{NSPoint, NSSize};
use ria_engine::call::main_queue;

use super::ardia_page_draw_wait::after_page_draw;
use crate::core::bindings::{CGImageRef, Tab};

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRetain(value: *const c_void) -> *const c_void;
    fn CFRelease(value: *const c_void);
}

struct Picture(*const c_void);

impl Picture {
    fn retain(image: CGImageRef) -> Option<Self> {
        if image.0.is_null() {
            return None;
        }
        // SAFETY: The engine supplies a live CGImage for this callback.
        Some(Self(unsafe { CFRetain(image.0) }))
    }
}

impl Drop for Picture {
    fn drop(&mut self) {
        // SAFETY: This reference came from CFRetain and has one owner.
        unsafe { CFRelease(self.0) };
    }
}

pub struct Ivars {
    tab: Weak<Tab>,
    pictured: Cell<NSSize>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaPageCover"]
    #[ivars = Ivars]
    pub struct PageCover;

    impl PageCover {
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> Option<Retained<NSView>> { None }

        #[unsafe(method(isAccessibilityElement))]
        fn is_accessibility_element(&self) -> bool { false }
    }
);

impl PageCover {
    pub fn cover(tab: &Tab, disabled: bool, done: impl FnOnce(Option<Retained<Self>>) + 'static) {
        let page = tab.view();
        let Some(window) = page
            .window()
            .filter(|_| !disabled && !page.isHiddenOrHasHiddenAncestor())
        else {
            done(None);
            return;
        };
        let weak_tab = Weak::new(tab);
        let weak_window = Weak::new(&*window);
        let host = super::superview(&page).as_deref().map(Weak::new);
        let pictured = page.bounds().size;
        let done = Cell::new(Some(done));
        tab.copy_picture(
            0.25,
            &RcBlock::new(move |image| {
                let Some(done) = done.take() else { return };
                let image = Picture::retain(image);
                let tab = weak_tab.clone();
                let window = weak_window.clone();
                let host = host.clone();
                main_queue::post(Box::new(move || {
                    let cover = (|| {
                        let (picture, tab, window) = (image?, tab.load()?, window.load()?);
                        let page = tab.view();
                        if !page
                            .window()
                            .is_some_and(|current| std::ptr::eq(&*current, &*window))
                        {
                            return None;
                        }
                        let former_host = match &host {
                            Some(host) => Some(host.load()?),
                            None => None,
                        };
                        match (super::superview(&page), former_host) {
                            (Some(a), Some(b)) if std::ptr::eq(&*a, &*b) => {}
                            (None, None) => {}
                            _ => return None,
                        }
                        let mtm = MainThreadMarker::new()?;
                        let this = <Self as MainThreadOnly>::alloc(mtm).set_ivars(Ivars {
                            tab: Weak::new(&*tab),
                            pictured: Cell::new(pictured),
                        });
                        // SAFETY: NSView initializes the allocated subclass for this rectangle.
                        let cover: Retained<Self> =
                            unsafe { msg_send![super(this), initWithFrame: page.bounds()] };
                        cover.setWantsLayer(true);
                        cover.setAutoresizingMask(
                            NSAutoresizingMaskOptions::ViewWidthSizable
                                | NSAutoresizingMaskOptions::ViewHeightSizable,
                        );
                        page.addSubview(&cover);
                        cover.displayIfNeeded();
                        cover.set_picture(&picture);
                        let weak = Weak::new(&*cover);
                        main_queue::after(
                            Duration::from_secs(4),
                            Box::new(move || {
                                if let Some(cover) = weak.load() {
                                    cover.uncover(false);
                                }
                            }),
                        );
                        Some(cover)
                    })();
                    done(cover);
                }));
            }),
        );
    }

    pub fn cover_tabs(
        tabs: &[Retained<Tab>],
        disabled: bool,
        done: impl FnOnce(Vec<Retained<Self>>) + 'static,
    ) {
        if tabs.is_empty() {
            done(Vec::new());
            return;
        }
        let shared = Rc::new((
            Cell::new(tabs.len()),
            RefCell::new(Vec::new()),
            Cell::new(Some(done)),
        ));
        for tab in tabs {
            let shared = shared.clone();
            Self::cover(tab, disabled, move |cover| {
                if let Some(cover) = cover {
                    shared.1.borrow_mut().push(cover);
                }
                shared.0.set(shared.0.get() - 1);
                if shared.0.get() == 0
                    && let Some(done) = shared.2.take()
                {
                    let covers = std::mem::take(&mut *shared.1.borrow_mut());
                    done(covers);
                }
            });
        }
    }

    fn set_picture(&self, picture: &Picture) {
        // SAFETY: A layer-backed NSView returns a CALayer; `contents` accepts a CGImage.
        unsafe {
            let layer: Option<Retained<AnyObject>> = msg_send![self, layer];
            if let Some(layer) = layer {
                let image: *const AnyObject = picture.0.cast();
                let _: () = msg_send![&*layer, setContents: image];
            }
        }
    }

    pub fn repicture(&self) {
        let Some(tab) = self
            .ivars()
            .tab
            .load()
            .filter(|_| super::superview(self).is_some())
        else {
            return;
        };
        let page = tab.view();
        let weak = Weak::new(self);
        let before = self.ivars().pictured.get();
        after_page_draw(
            &tab,
            move |_, _| page.bounds().size != before,
            move || {
                let Some(cover) = weak
                    .load()
                    .filter(|cover| super::superview(cover).is_some())
                else {
                    return;
                };
                let Some(tab) = cover.ivars().tab.load() else {
                    return;
                };
                let now = tab.view().bounds().size;
                if now == before {
                    return;
                }
                tab.copy_picture(
                    0.25,
                    &RcBlock::new(move |image| {
                        let Some(image) = Picture::retain(image) else {
                            return;
                        };
                        let weak = weak.clone();
                        main_queue::post(Box::new(move || {
                            if let Some(cover) = weak
                                .load()
                                .filter(|cover| super::superview(cover).is_some())
                            {
                                cover.set_picture(&image);
                                cover.ivars().pictured.set(now);
                            }
                        }));
                    }),
                );
            },
        );
    }

    pub fn uncover(&self, fade: bool) {
        if super::superview(self).is_none() {
            return;
        }
        if !fade {
            self.removeFromSuperview();
            return;
        }
        let cover = self.retain();
        NSAnimationContext::runAnimationGroup_completionHandler(
            &RcBlock::new(|context: std::ptr::NonNull<NSAnimationContext>| {
                // SAFETY: AppKit lends the animation context for this block.
                unsafe {
                    context.as_ref().setDuration(0.1);
                    let animator: Retained<AnyObject> = msg_send![self, animator];
                    let _: () = msg_send![&*animator, setAlphaValue: 0.0_f64];
                }
            }),
            Some(&RcBlock::new(move || cover.removeFromSuperview())),
        );
    }
}
