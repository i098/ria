// Ported from ardia packages/ardiacore/ios/ArdiaContentBlockerPage.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! A hidden extension page (the uBlock Origin `blank.html`, or the reader page of uBlock Origin Lite) that runs scripts
//! one at a time. The tab and its window belong to the host; it reaches them through `ExtensionScripts` and
//! `ExtensionTab`, and tells the page what the tab does through `PageDelegate`.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use serde_json::Value;

use super::content_blocker::BlockerHost;
use super::host::Profile;

/// The answer of a script: a value (`None` for `null`) or an error text.
pub type Done = Box<dyn FnOnce(Option<Value>, Option<String>)>;

/// `ArdiaCoreTab`, the part that a hidden extension page uses.
pub trait ExtensionTab {
    fn url(&self) -> String;
    fn loading(&self) -> bool;
    fn closed(&self) -> bool;
    /// `tab.delegate = page`: the tab sends its changes to `delegate`.
    fn set_delegate(&self, delegate: PageDelegate);
    /// `evaluate:completion:`: runs `script` in the page; `done` gets the text that the script posts as `result`.
    fn evaluate(&self, script: &str, done: Box<dyn FnOnce(String)>);
    fn close_now(&self);
}

/// The JavaScript of the content blocker runs in extension pages: this opens them.
pub trait ExtensionScripts {
    /// Opens `url` in the foreground of the hidden internal window of `window_key` (one 400 x 300 window for each
    /// profile, made on first use, `internal = YES`). `None` when no tab opens. The host also answers
    /// `IsHiddenPageWindow` from these windows.
    fn open_tab(
        &self,
        window_key: &str,
        profile: &Profile,
        url: &str,
    ) -> Option<Rc<dyn ExtensionTab>>;
}

/// `ArdiaCoreTabDelegate` of a page: the host calls these when the tab changes.
#[derive(Clone)]
pub struct PageDelegate(Weak<Page>);

impl PageDelegate {
    pub fn did_change_loading(&self) {
        if let Some(page) = self.0.upgrade() {
            let loading = page.tab.loading();
            page.started.set(page.started.get() || loading);
            page.loaded.set(page.started.get() && !loading);
            page.next();
        }
    }

    pub fn did_change_url(&self) {
        if let Some(page) = self.0.upgrade()
            && !page.tab.url().is_empty()
            && !page.on_own_page()
        {
            page.left_own_page();
        }
    }

    pub fn did_commit_document(&self) {
        if let Some(page) = self.0.upgrade()
            && !page.on_own_page()
        {
            page.left_own_page();
        }
    }

    pub fn will_close(&self) {
        if let Some(page) = self.0.upgrade() {
            page.fail("closed");
        }
    }

    pub fn renderer_gone(&self, status: &str) {
        if let Some(page) = self.0.upgrade() {
            page.fail(&format!("renderer {status}"));
        }
    }

    pub fn did_fail_load(&self, code: i32, description: &str) {
        if let Some(page) = self.0.upgrade()
            && code != -3
        {
            page.fail(description);
        }
    }
}

/// `ArdiaContentBlockerPage`.
pub struct Page {
    tab: Rc<dyn ExtensionTab>,
    origin: String,
    host: Rc<dyn BlockerHost>,
    dead: Cell<bool>,
    calls: RefCell<VecDeque<(String, Done)>>,
    inflight: RefCell<Option<Done>>,
    started: Cell<bool>,
    loaded: Cell<bool>,
    busy: Cell<bool>,
}

impl Page {
    pub fn new(tab: Rc<dyn ExtensionTab>, origin: &str, host: Rc<dyn BlockerHost>) -> Rc<Self> {
        let page = Rc::new(Self {
            started: Cell::new(tab.loading()),
            tab,
            origin: origin.to_owned(),
            host,
            dead: Cell::new(false),
            calls: RefCell::default(),
            inflight: RefCell::default(),
            loaded: Cell::new(false),
            busy: Cell::new(false),
        });
        page.tab.set_delegate(PageDelegate(Rc::downgrade(&page)));
        page
    }

    pub fn tab(&self) -> &Rc<dyn ExtensionTab> {
        &self.tab
    }

    pub fn dead(&self) -> bool {
        self.dead.get()
    }

    pub fn on_own_page(&self) -> bool {
        !self.dead.get() && self.tab.url().starts_with(&self.origin)
    }

    pub fn run(self: &Rc<Self>, script: String, done: Done) {
        if self.dead.get() {
            return done(None, Some("closed".into()));
        }
        self.calls.borrow_mut().push_back((script, done));
        self.next();
    }

    pub fn left_own_page(&self) {
        self.fail(&format!("not the extension page: {}", self.tab.url()));
    }

    pub fn next(self: &Rc<Self>) {
        if self.dead.get()
            || !self.loaded.get()
            || self.busy.get()
            || self.calls.borrow().is_empty()
        {
            return;
        }
        if !self.on_own_page() {
            return self.left_own_page();
        }
        let Some((script, done)) = self.calls.borrow_mut().pop_front() else {
            return;
        };
        *self.inflight.borrow_mut() = Some(done);
        self.busy.set(true);
        let page = self.clone();
        self.tab.evaluate(
            &script,
            Box::new(move |json| {
                if page.inflight.borrow().is_none() {
                    return;
                }
                if !page.on_own_page() {
                    return page.left_own_page();
                }
                let done = page.inflight.borrow_mut().take();
                page.busy.set(false);
                let (value, error) = answer_of(&json);
                if let Some(done) = done {
                    done(value, error);
                }
                page.next();
            }),
        );
    }

    pub fn fail(&self, error: &str) {
        if self.dead.replace(true) {
            return;
        }
        let calls = std::mem::take(&mut *self.calls.borrow_mut());
        let inflight = self.inflight.borrow_mut().take();
        let (tab, error) = (self.tab.clone(), error.to_owned());
        self.host.post(Box::new(move || {
            tab.close_now();
            if let Some(inflight) = inflight {
                inflight(None, Some(error.clone()));
            }
            for (_, done) in calls {
                done(None, Some(error.clone()));
            }
        }));
    }
}

/// The value and error of the text that a script posts: `{"value": ...}` or `{"error": "..."}`.
fn answer_of(json: &str) -> (Option<Value>, Option<String>) {
    let Ok(Value::Object(answer)) = serde_json::from_str::<Value>(json) else {
        return (None, Some("no answer".into()));
    };
    if let Some(Value::String(error)) = answer.get("error") {
        return (None, Some(error.clone()));
    }
    (answer.get("value").filter(|v| !v.is_null()).cloned(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::data::content_blocker::tests::{FakeHost, FakeTab};

    #[test]
    fn answers_become_values_or_errors() {
        assert_eq!(answer_of(r#"{"value":3}"#), (Some(3.into()), None));
        assert_eq!(answer_of(r#"{"value":null}"#), (None, None));
        assert_eq!(answer_of("{}"), (None, None));
        assert_eq!(answer_of(r#"{"error":"x"}"#), (None, Some("x".into())));
        assert_eq!(answer_of("[]"), (None, Some("no answer".into())));
        assert_eq!(answer_of("nope"), (None, Some("no answer".into())));
    }

    #[test]
    fn a_page_runs_after_load_and_fails_when_it_leaves_its_origin() {
        let (host, _resources) = FakeHost::new();
        let tab = FakeTab::new("chrome-extension://x/blank.html");
        let page = Page::new(tab.clone(), "chrome-extension://x/", host.clone());
        let got = Rc::new(RefCell::new(Vec::new()));
        let sink = got.clone();
        page.run(
            "1".into(),
            Box::new(move |v, e| sink.borrow_mut().push((v, e))),
        );
        assert!(tab.scripts.borrow().is_empty(), "waits for the load");
        tab.loading.set(true);
        tab.delegate().did_change_loading();
        tab.loading.set(false);
        tab.delegate().did_change_loading();
        tab.answer(r#"{"value":1}"#);
        assert_eq!(got.borrow().as_slice(), &[(Some(1.into()), None)]);

        let sink = got.clone();
        page.run(
            "2".into(),
            Box::new(move |v, e| sink.borrow_mut().push((v, e))),
        );
        *tab.url.borrow_mut() = "https://evil.example/".into();
        tab.delegate().did_change_url();
        assert!(page.dead());
        host.run_posted();
        assert!(tab.closed.get());
        assert_eq!(
            got.borrow()[1],
            (
                None,
                Some("not the extension page: https://evil.example/".into())
            )
        );
        let sink = got.clone();
        page.run(
            "3".into(),
            Box::new(move |v, e| sink.borrow_mut().push((v, e))),
        );
        assert_eq!(got.borrow()[2], (None, Some("closed".into())));
    }

    #[test]
    fn an_aborted_load_does_not_fail_the_page() {
        let (host, _resources) = FakeHost::new();
        let tab = FakeTab::new("chrome-extension://x/blank.html");
        let page = Page::new(tab.clone(), "chrome-extension://x/", host);
        tab.delegate().did_fail_load(-3, "aborted");
        assert!(!page.dead());
        tab.delegate().renderer_gone("crashed");
        assert!(page.dead());
    }
}
