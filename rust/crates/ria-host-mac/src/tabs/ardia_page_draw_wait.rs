// Ported from ardia packages/ardiacore/ios/ArdiaPageDrawWait.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use objc2::rc::Weak;
use objc2_foundation::{NSSize, NSString};
use ria_engine::call::main_queue;

use crate::core::bindings::Tab;

const SCRIPT: &str = "requestAnimationFrame(() => requestAnimationFrame(() => post('result', JSON.stringify([innerWidth * devicePixelRatio, innerHeight * devicePixelRatio, !!document.fullscreenElement]))))";

struct Wait<T, D> {
    tab: Weak<Tab>,
    test: T,
    done: Cell<Option<D>>,
}

pub fn after_page_draw(
    tab: &Tab,
    test: impl Fn(bool, bool) -> bool + 'static,
    done: impl FnOnce() + 'static,
) {
    if super::tab_view(tab).is_none_or(|page| {
        page.window().is_none() || page.isHiddenOrHasHiddenAncestor()
    }) {
        done();
        return;
    }
    let wait = Rc::new(Wait {
        tab: Weak::new(tab),
        test,
        done: Cell::new(Some(done)),
    });
    let backstop = wait.clone();
    main_queue::after(
        Duration::from_millis(400),
        Box::new(move || backstop.finish()),
    );
    wait.ask();
}

impl<T: Fn(bool, bool) -> bool + 'static, D: FnOnce() + 'static> Wait<T, D> {
    fn finish(&self) {
        if let Some(done) = self.done.take() {
            done();
        }
    }

    fn ask(self: &Rc<Self>) {
        let Some(tab) = self.tab.load() else {
            return self.finish();
        };
        let Some(page) = super::tab_view(&tab) else {
            return self.finish();
        };
        if page.window().is_none() {
            return self.finish();
        }
        let asked = page.convertRect_toView(page.bounds(), None).size;
        let wait = Rc::downgrade(self);
        tab.evaluate(&NSString::from_str(SCRIPT), move |reply| {
            let wait = wait.clone();
            main_queue::post(Box::new(move || {
                if let Some(wait) = wait.upgrade() {
                    wait.answered(reply, asked);
                }
            }));
        });
    }

    fn answered(self: &Rc<Self>, reply: Option<String>, asked: NSSize) {
        let Some(done) = self.done.take() else { return };
        self.done.set(Some(done));
        let Some(tab) = self.tab.load() else {
            return self.finish();
        };
        let Some(page) = super::tab_view(&tab) else {
            return self.finish();
        };
        let Some(content) = page.window().and_then(|window| window.contentView()) else {
            return self.finish();
        };
        let Some((width, height, fullscreen)) = reply
            .as_deref()
            .and_then(|reply| serde_json::from_str::<(f64, f64, bool)>(reply).ok())
        else {
            return self.finish();
        };
        let now = page.convertRect_toView(page.bounds(), None).size;
        let pixels = page.convertSizeToBacking(page.bounds().size);
        let content = content.bounds().size;
        let fills =
            (now.width - content.width).abs() < 1.0 && (now.height - content.height).abs() < 1.0;
        let sized = asked == now
            && (width - pixels.width).abs() <= 2.0
            && (height - pixels.height).abs() <= 2.0;
        if sized && (self.test)(fullscreen, fills) {
            self.finish();
        } else {
            self.ask();
        }
    }
}
