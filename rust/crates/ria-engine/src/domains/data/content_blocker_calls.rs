// Ported from ardia packages/ardiacore/ios/ArdiaContentBlockerCalls.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The calls into uBlock Origin: one queue of operations for each profile, each operation a script that runs in the
//! hidden `blank.html` page of the extension, with retries while uBlock Origin restarts and one deadline.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use serde_json::Value;

use super::content_blocker::{ContentBlocker, EXTENSION_ID, OFF_DIRECTIVE};
use super::content_blocker_page::{Done, Page};
use super::host::Profile;

/// `ScriptWith`: `format` with each `%@` replaced by the JSON of the next argument (`null` when there are fewer).
pub fn script_with(format: &str, args: &[Value]) -> String {
    let mut out = String::with_capacity(format.len());
    for (i, part) in format.split("%@").enumerate() {
        if i > 0
            && let Some(arg) = args.get(i - 1)
        {
            out.push_str(&serde_json::to_string(arg).unwrap_or_else(|_| "null".into()));
        }
        out.push_str(part);
    }
    out
}

/// `format` with each `%@` replaced by the next raw text (`stringWithFormat:`).
fn fill(format: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(format.len());
    for (i, part) in format.split("%@").enumerate() {
        if i > 0 {
            out.push_str(args.get(i - 1).copied().unwrap_or(""));
        }
        out.push_str(part);
    }
    out
}

/// One operation of the queue of a profile (`ArdiaContentBlockerOp`).
pub(super) struct Op {
    profile: String,
    expression: String,
    completion: RefCell<Option<Done>>,
    attempts_left: Cell<i32>,
    fail_next: Cell<bool>,
    deadline: Duration,
    attempt: Cell<u64>,
    loads_profile: bool,
}

const ATTEMPTS: i32 = 40;
const RETRY: Duration = Duration::from_millis(250);
const OP_TIME: Duration = Duration::from_secs(20);
const RELOAD_WAIT_MS: u64 = 4000;
const PAGE_IDLE: Duration = Duration::from_secs(30);

const PRELUDE: &str = concat!(
    "let failing = %@;",
    "const offDirective = %@;",
    "const extensionOrigin = %@;",
    "const deadline = Date.now() + %@;",
    "const left = () => Math.max(0, deadline - Date.now());",
    "let expired = false;",
    "const withDeadline = (promise, what) => new Promise((resolve, reject) => {",
    "  const timer = setTimeout(() => { expired = true; reject(new Error(`${what}: timed out`)); }, left());",
    "  promise.then((v) => { clearTimeout(timer); resolve(v); }, (e) => { clearTimeout(timer); reject(e); });",
    "});",
    "const ready = () => new Promise((resolve, reject) => {",
    "  const look = () => {",
    "    const ublock = chrome.extension.getBackgroundPage()?.µBlock;",
    "    if (ublock?.isReadyPromise) return ublock.isReadyPromise.then(resolve, reject);",
    "    if (left() === 0) return reject(new Error('uBlock Origin never started'));",
    "    setTimeout(look, 10);",
    "  };",
    "  look();",
    "});",
    "const loadScript = (src) => new Promise((resolve, reject) => {",
    "  const element = document.createElement('script');",
    "  element.src = src;",
    "  element.onload = resolve;",
    "  element.onerror = () => reject(new Error(`${src} didn't load`));",
    "  document.head.append(element);",
    "});",
    "const client = () => self.vAPI?.messaging ? Promise.resolve()",
    "  : ['js/vapi.js', 'js/vapi-common.js', 'js/vapi-client.js'].reduce((p, src) => p.then(() => loadScript(src)), Promise.resolve());",
    "const send = (message) => {",
    "  if (expired) return Promise.reject(new Error(`${message.what}: timed out`));",
    "  if (failing) { failing = false; return Promise.reject(new Error(`${message.what}: failed (dev)`)); }",
    "  return client().then(() => {",
    "    if (expired) throw new Error(`${message.what}: timed out`);",
    "    return vAPI.messaging.send('dashboard', message);",
    "  }).then((reply) => {",
    "    if (reply === undefined) throw new Error(`${message.what}: no reply`);",
    "    return reply;",
    "  });",
    "};",
    "const trusted = () => send({what: 'getWhitelist'}).then((t) => {",
    "  if (!t || !Array.isArray(t.whitelist)) throw new Error('getWhitelist: no answer');",
    "  return t;",
    "});",
    "const lists = () => send({what: 'getLists'}).then((l) => {",
    "  if (!l || !l.available || typeof l.available !== 'object') throw new Error('getLists: no answer');",
    "  return l;",
    "});",
    "const selected = (l) => Object.keys(l.available).filter((k) => l.available[k] && l.available[k].off !== true);",
    "const imported = (l) => Object.keys(l.available).filter((k) => l.available[k] && l.available[k].group === 'custom').sort();",
    "const covers = (directive, host) => !directive.includes('/') && (host === directive || host.endsWith('.' + directive));",
    "const sameSet = (a, b) => [...a].sort().join('\\n') === [...b].sort().join('\\n');",
    "const storageCall = (name, arg) => expired ? Promise.reject(new Error(`storage ${name}: timed out`))",
    "  : new Promise((resolve, reject) => chrome.storage.local[name](arg, (v) =>",
    "    chrome.runtime.lastError ? reject(new Error(chrome.runtime.lastError.message)) : resolve(v || {})));",
    "const local = {get: (k) => storageCall('get', k), set: (v) => storageCall('set', v), remove: (k) => storageCall('remove', k)};",
    "const pendingReload = () => local.get('ardiaReloadPending').then((b) => b.ardiaReloadPending);",
    "const reloadLists = () => pendingReload().then((asked) => send({what: 'reloadAllFilters'})",
    "  .then(pendingReload)",
    "  .then((now) => now === asked ? local.remove('ardiaReloadPending') : reloadLists()));",
    "const reloadWait = (ms = %@) => new Promise((resolve) => setTimeout(resolve, Math.min(ms, Math.max(0, left() - 1000))));",
    "const askReload = () => local.set({ardiaReloadPending: `${Date.now()}-${Math.random()}`});",
    "const applySelection = (details, waitMs) => askReload()",
    "  .then(() => send({what: 'applyFilterListSelection', ...details}))",
    "  .then(askReload)",
    "  .then(() => Promise.race([reloadLists(), reloadWait(waitMs)]));",
    "const applyLists = (toSelect) => applySelection({toSelect});",
    "const resumePendingReload = () => { pendingReload().then((asked) => asked ? reloadLists() : null).catch(() => {}); };",
    "Promise.resolve().then(() => { if (location.origin !== extensionOrigin) throw new Error(`not the extension page: ${location.href}`); })",
    ".then(() => withDeadline(ready(), 'uBlock Origin'))",
    ".then(() => { resumePendingReload(); return withDeadline(Promise.resolve().then(() => (%@)), 'the call'); })",
    ".then((v) => post('result', JSON.stringify({value: v === undefined ? null : v})),",
    "(e) => post('result', JSON.stringify({error: String(e && e.message || e)})))",
);

/// The script of one attempt: the prelude with the dev failure flag, the time left and the expression.
pub(super) fn attempt_script(fail_next: bool, left_ms: u64, expression: &str) -> String {
    fill(
        PRELUDE,
        &[
            if fail_next { "true" } else { "false" },
            &script_with("%@", &[OFF_DIRECTIVE.into()]),
            &script_with("%@", &[format!("chrome-extension://{EXTENSION_ID}").into()]),
            &left_ms.to_string(),
            &RELOAD_WAIT_MS.to_string(),
            expression,
        ],
    )
}

fn origin_of(extension_id: &str) -> String {
    format!("chrome-extension://{extension_id}/")
}

/// `Retriable`: errors that a restart of uBlock Origin or of its page explains.
pub(super) fn retriable(error: &str, on_own_page: bool) -> bool {
    !on_own_page
        || [
            "no reply",
            "no answer",
            "context invalidated",
            "not the extension page",
            "renderer ",
            "closed",
        ]
        .iter()
        .any(|sign| error.contains(sign))
}

impl ContentBlocker {
    /// `HiddenPage`: the live page `key`, or a new one of `file` of the extension in the hidden window of `profile`.
    pub(super) fn hidden_page(
        &self,
        key: &str,
        profile: &str,
        p: &Profile,
        extension_id: &str,
        file: &str,
    ) -> Option<Rc<Page>> {
        let page = self.0.state.borrow().pages.get(key).cloned();
        if let Some(page) = &page {
            if page.tab().closed() {
                page.fail("closed");
            }
            if !page.dead() {
                return Some(page.clone());
            }
        }
        let origin = origin_of(extension_id);
        let tab = self
            .0
            .scripts
            .open_tab(profile, p, &format!("{origin}{file}"));
        let page = tab.map(|tab| Page::new(tab, &origin, self.0.host.clone()));
        let mut state = self.0.state.borrow_mut();
        match &page {
            Some(page) => state.pages.insert(key.to_owned(), page.clone()),
            None => state.pages.remove(key),
        };
        page
    }

    fn close_when_idle(&self, profile: &str) {
        let use_ = {
            let mut state = self.0.state.borrow_mut();
            let uses = state.uses.entry(profile.to_owned()).or_default();
            *uses += 1;
            *uses
        };
        let (blocker, profile) = (self.clone(), profile.to_owned());
        self.0.host.after(
            PAGE_IDLE,
            Box::new(move || {
                let page = {
                    let mut state = blocker.0.state.borrow_mut();
                    if state.uses.get(&profile) != Some(&use_)
                        || state.busy.contains(&profile)
                        || state.queues.get(&profile).is_some_and(|q| !q.is_empty())
                    {
                        return;
                    }
                    state.pages.remove(&profile)
                };
                if let Some(page) = page {
                    page.fail("idle");
                }
            }),
        );
    }

    fn finish(&self, op: &Rc<Op>, value: Option<Value>, error: Option<String>) {
        {
            let mut state = self.0.state.borrow_mut();
            if let Some(queue) = state.queues.get_mut(&op.profile)
                && queue.front().is_some_and(|first| Rc::ptr_eq(first, op))
            {
                queue.pop_front();
            }
            state.busy.remove(&op.profile);
        }
        if let Some(error) = &error {
            eprintln!("[blocker] profile \"{}\": {error}", op.profile);
        }
        let completion = op.completion.borrow_mut().take();
        if let Some(completion) = completion {
            completion(value, error);
        }
        self.pump(&op.profile);
        let idle = self
            .0
            .state
            .borrow()
            .queues
            .get(&op.profile)
            .is_none_or(|q| q.is_empty());
        if idle {
            self.close_when_idle(&op.profile);
        }
    }

    fn attempt(&self, op: Rc<Op>) {
        let host = self.0.host.clone();
        let profile = op.profile.clone();
        if !host.is_started() {
            return self.finish(&op, None, Some("unavailable".into()));
        }
        self.forget_if_gone(&profile);
        if !self.is_loaded(&profile) {
            if !op.loads_profile {
                return self.finish(&op, None, Some("profile gone".into()));
            }
            let blocker = self.clone();
            return self.load_into_profile(
                &profile,
                Box::new(move |error| match error {
                    Some(error) => blocker.finish(&op, None, Some(error)),
                    None => blocker.attempt(op),
                }),
            );
        }
        let page = host
            .loaded_profile(&profile)
            .and_then(|p| self.hidden_page(&profile, &profile, &p, EXTENSION_ID, "blank.html"));
        let Some(page) = page else {
            return self.finish(&op, None, Some("not loaded".into()));
        };
        let left = op.deadline.saturating_sub(host.now());
        let left_ms = u64::try_from(left.as_millis()).unwrap_or(u64::MAX);
        let script = attempt_script(op.fail_next.get(), left_ms, &op.expression);
        let attempt = op.attempt.get() + 1;
        op.attempt.set(attempt);
        let (blocker, timed_op, timed_page) = (self.clone(), op.clone(), page.clone());
        host.after(
            left + Duration::from_secs(2),
            Box::new(move || {
                let first = blocker
                    .0
                    .state
                    .borrow()
                    .queues
                    .get(&timed_op.profile)
                    .and_then(|q| q.front().cloned());
                if timed_op.attempt.get() == attempt
                    && first.is_some_and(|f| Rc::ptr_eq(&f, &timed_op))
                {
                    timed_page.fail("the call timed out");
                }
            }),
        );
        let (blocker, run_page) = (self.clone(), page.clone());
        page.run(
            script,
            Box::new(move |value, error| {
                if op.attempt.get() != attempt {
                    return;
                }
                if let Some(error) = &error {
                    if error.ends_with("failed (dev)") {
                        op.fail_next.set(false);
                    }
                    if error.contains("timed out") {
                        run_page.fail(error);
                    }
                }
                let in_time = blocker.0.host.now() < op.deadline;
                if let Some(error) = &error
                    && op.attempts_left.get() > 0
                    && in_time
                    && retriable(error, run_page.on_own_page())
                    && blocker.is_loaded(&op.profile)
                    && !blocker.0.host.is_deleted_profile(&op.profile)
                {
                    op.attempts_left.set(op.attempts_left.get() - 1);
                    run_page.fail("uBlock Origin restarted");
                    let retry = blocker.clone();
                    blocker
                        .0
                        .host
                        .after(RETRY, Box::new(move || retry.attempt(op)));
                    return;
                }
                blocker.finish(&op, value, error);
            }),
        );
    }

    fn pump(&self, profile: &str) {
        let op = {
            let mut state = self.0.state.borrow_mut();
            if state.busy.contains(profile) {
                return;
            }
            let Some(op) = state.queues.get(profile).and_then(|q| q.front().cloned()) else {
                return;
            };
            state.busy.insert(profile.to_owned());
            op
        };
        self.attempt(op);
    }

    /// `Call`: queues `expression` for the uBlock Origin page of `profile`. `loads_profile` loads the extension into
    /// the profile first when it is not there.
    pub(super) fn call(
        &self,
        profile: &str,
        expression: String,
        completion: Done,
        loads_profile: bool,
    ) {
        let fail_next = {
            let mut state = self.0.state.borrow_mut();
            expression != "null" && std::mem::take(&mut state.fail_next_message)
        };
        let op = Rc::new(Op {
            profile: profile.to_owned(),
            expression,
            completion: RefCell::new(Some(completion)),
            attempts_left: Cell::new(ATTEMPTS),
            fail_next: Cell::new(fail_next),
            deadline: self.0.host.now() + OP_TIME,
            attempt: Cell::new(0),
            loads_profile,
        });
        self.0
            .state
            .borrow_mut()
            .queues
            .entry(profile.to_owned())
            .or_default()
            .push_back(op);
        self.pump(profile);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domains::data::content_blocker::tests::{Answers, setup};

    #[test]
    fn scripts_take_json_arguments() {
        assert_eq!(
            script_with("[%@, %@]", &[json!("a\"b"), json!(true)]),
            r#"["a\"b", true]"#
        );
        assert_eq!(script_with("%@-%@", &[json!(1)]), "1-");
        assert_eq!(script_with("none", &[json!(1)]), "none");
    }

    #[test]
    fn retries_cover_restarts_only() {
        assert!(retriable("getLists: no reply", true));
        assert!(retriable("renderer crashed", true));
        assert!(retriable("anything", false));
        assert!(!retriable("no list x", true));
    }

    #[test]
    fn calls_run_in_order_and_retry_a_restart() {
        let (blocker, host, scripts) = setup();
        let got = Answers::default();
        blocker.call("", "1".into(), got.done(), true);
        blocker.call("", "2".into(), got.done(), true);
        let tab = scripts.tab(0);
        tab.load();
        assert_eq!(tab.scripts.borrow().len(), 1, "one call at a time");
        tab.answer(r#"{"error":"getLists: no reply"}"#);
        host.run_posted();
        host.fire(RETRY);
        let tab = scripts.tab(1);
        tab.load();
        tab.answer(r#"{"value":"one"}"#);
        tab.answer(r#"{"value":"two"}"#);
        assert_eq!(
            got.take(),
            vec![(Some(json!("one")), None), (Some(json!("two")), None)]
        );
    }

    #[test]
    fn errors_after_the_deadline_are_final() {
        let (blocker, host, scripts) = setup();
        let got = Answers::default();
        blocker.call("", "1".into(), got.done(), true);
        let tab = scripts.tab(0);
        tab.load();
        host.advance(OP_TIME);
        tab.answer(r#"{"error":"getLists: no reply"}"#);
        assert_eq!(got.take(), vec![(None, Some("getLists: no reply".into()))]);
    }

    #[test]
    fn a_stuck_call_times_out_and_an_idle_page_closes() {
        let (blocker, host, scripts) = setup();
        let got = Answers::default();
        blocker.call("", "1".into(), got.done(), true);
        let tab = scripts.tab(0);
        tab.load();
        host.advance(OP_TIME);
        host.fire(Duration::from_secs(2));
        host.run_posted();
        assert_eq!(got.take(), vec![(None, Some("the call timed out".into()))]);
        assert!(tab.closed.get());

        blocker.call("", "2".into(), got.done(), true);
        let tab = scripts.tab(1);
        tab.load();
        tab.answer(r#"{"value":2}"#);
        host.fire(PAGE_IDLE);
        host.run_posted();
        assert!(tab.closed.get());
    }

    #[test]
    fn a_call_without_a_started_host_is_unavailable() {
        let (blocker, host, _) = setup();
        host.started.set(false);
        let got = Answers::default();
        blocker.call("", "1".into(), got.done(), true);
        assert_eq!(got.take(), vec![(None, Some("unavailable".into()))]);
    }
}
