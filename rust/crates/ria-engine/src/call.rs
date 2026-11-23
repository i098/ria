// Ported from Ardia (https://github.com/ardia-browser/ardia), commit 551d3c13c:
// packages/ardiacore/ios/ArdiaEngineBridge.mm (`call:`, `callWithSecret:`, `callForSecretReply:`, `Dispatch`, `InstallSink`)
// and packages/sync/ios/SyncModule.swift:356 (the 30 s timeout of `ac_passwords_logins`).
// Copyright 2026 Mantra Patel, Apache-2.0. This file is a Rust port with changes.

//! The call layer (plan section 6.4): every reply and every event is a deferred entry.
//! The engine pointers are copied inside the callback, and the work goes into the [`ActionQueue`].
//! A reply and a timeout share one settled cell; the first one wins, and only the reply frees the call context.

#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ffi::{CStr, CString, c_char, c_void};
use std::rc::{Rc, Weak};
use std::time::Duration;

use ria_engine_sys::{CallFn, ReplyFn};
use serde_json::Value;
use zeroize::Zeroize;

/// Timeout of a call that Ardia gives no timeout. Ardia's bridge has none for these calls (`ArdiaEngineBridge.mm:131-148`),
/// and the plan (section 6.4) wants one for each call.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// The timeouts that Ardia sets for an engine call: only `callForSecretReply` has a value
/// (`SyncModule.swift:217` with `loginsTimeout`, `SyncModule.swift:356`).
pub const TIMEOUTS: &[(&str, Duration)] = &[("ac_passwords_logins", Duration::from_secs(30))];

pub fn timeout_for(name: &str) -> Duration {
    TIMEOUTS
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(DEFAULT_TIMEOUT, |(_, t)| *t)
}

#[derive(Debug, PartialEq)]
pub enum CallError {
    /// No `engineDidStart` yet, or after `shutdown`.
    NotStarted,
    /// The name is not an engine call, or the engine does not export it.
    UnknownCall(String),
    /// The arguments are not a JSON object, or a secret buffer is empty or has no final NUL byte.
    BadArguments,
    /// A secret call for a private profile.
    PrivateProfile,
    /// The engine answered `{"error": ...}`.
    Engine(String),
    /// The reply is null or is not JSON.
    BadReply,
    Timeout,
    /// `shutdown` ran while the call was pending.
    Cancelled,
}

pub type CallResult = Result<Value, CallError>;
type Completion = Box<dyn FnOnce(CallResult)>;
type Action = Box<dyn FnOnce()>;
type Schedule = fn(Duration, Action);

/// Items that run on the main thread, one drain at a time. A drain runs the items that are in the queue now;
/// items that they add wait for the next drain, so a drain always ends.
#[derive(Default)]
pub struct ActionQueue {
    items: RefCell<VecDeque<Action>>,
    waker: RefCell<Option<Box<dyn Fn()>>>,
}

impl ActionQueue {
    pub fn new() -> Rc<Self> {
        Rc::default()
    }

    /// The host sets a function that makes a run-loop source fire; it calls [`ActionQueue::drain`].
    pub fn set_waker(&self, waker: impl Fn() + 'static) {
        *self.waker.borrow_mut() = Some(Box::new(waker));
    }

    pub fn push(&self, action: Action) {
        let was_empty = {
            let mut items = self.items.borrow_mut();
            items.push_back(action);
            items.len() == 1
        };
        if was_empty && let Some(wake) = &*self.waker.borrow() {
            wake();
        }
    }

    /// Runs the items that are in the queue now. Returns how many ran.
    pub fn drain(&self) -> usize {
        let batch = std::mem::take(&mut *self.items.borrow_mut());
        let count = batch.len();
        for action in batch {
            action();
        }
        count
    }
}

/// The settled cell: holds the completion until the reply or the timeout takes it. The first taker wins.
struct Settle(Cell<Option<Completion>>);

impl Settle {
    fn finish(&self, queue: &ActionQueue, result: CallResult) {
        if let Some(done) = self.0.take() {
            queue.push(Box::new(move || done(result)));
        }
    }

    fn is_settled(&self) -> bool {
        let done = self.0.take();
        let settled = done.is_none();
        self.0.set(done);
        settled
    }
}

/// The context of one call in flight. Only [`replied`] frees it.
struct Ctx {
    settle: Rc<Settle>,
    queue: Rc<ActionQueue>,
}

unsafe extern "C" fn replied(context: *mut c_void, json: *const c_char) {
    // SAFETY: `context` is the `Box<Ctx>` that `Engine::start_call` made with `into_raw`; a reply runs once (`ac_engine.cc:62-70`).
    let ctx = unsafe { Box::from_raw(context.cast::<Ctx>()) };
    let result = if json.is_null() {
        Err(CallError::BadReply)
    } else {
        // SAFETY: `json` is a NUL-terminated string that is valid during this callback.
        parse_reply(unsafe { CStr::from_ptr(json) }.to_bytes())
    };
    ctx.settle.finish(&ctx.queue, result);
}

fn parse_reply(bytes: &[u8]) -> CallResult {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| CallError::BadReply)?;
    match value.get("error") {
        Some(Value::String(message)) => Err(CallError::Engine(message.clone())),
        Some(other) => Err(CallError::Engine(other.to_string())),
        None => Ok(value),
    }
}

type Handler = Rc<dyn Fn(&Value)>;

struct State {
    queue: Rc<ActionQueue>,
    lookup: fn(&str) -> Option<CallFn>,
    schedule: Schedule,
    started: Cell<bool>,
    symbols: RefCell<HashMap<String, CallFn>>,
    pending: RefCell<Vec<Weak<Settle>>>,
    handlers: RefCell<HashMap<String, Vec<Handler>>>,
    /// The `Rc<State>` raw pointer that the event sink holds, while a sink is set.
    sink_context: Cell<*const State>,
}

/// The call layer. Not `Send`: it lives on the main thread, as the engine requires.
#[derive(Clone)]
pub struct Engine(Rc<State>);

impl Engine {
    /// An engine over the loaded `Chromium Framework`. Panics when it is not called on the main thread.
    #[cfg(target_os = "macos")]
    pub fn new(queue: Rc<ActionQueue>) -> Self {
        // SAFETY: `pthread_main_np` has no preconditions.
        assert_ne!(
            unsafe { libc_main_np() },
            0,
            "the engine call layer must be made on the main thread"
        );
        Self::with(queue, ria_engine_sys::call_symbol, main_queue::after)
    }

    /// An engine with its own symbol lookup and timer (tests).
    pub fn with(
        queue: Rc<ActionQueue>,
        lookup: fn(&str) -> Option<CallFn>,
        schedule: Schedule,
    ) -> Self {
        Self(Rc::new(State {
            queue,
            lookup,
            schedule,
            started: Cell::new(false),
            symbols: RefCell::default(),
            pending: RefCell::default(),
            handlers: RefCell::default(),
            sink_context: Cell::new(std::ptr::null()),
        }))
    }

    /// Call on `engineDidStart`: opens the gate for calls and sets the event sink.
    pub fn did_start(&self) {
        self.0.started.set(true);
        self.install_sink();
    }

    /// Call on `engineWillShutDown`: clears the sink and cancels each pending call.
    /// A reply that arrives later still frees its context and has no other effect.
    pub fn shutdown(&self) {
        let state = &self.0;
        state.started.set(false);
        let context = state.sink_context.replace(std::ptr::null());
        if !context.is_null() {
            // SAFETY: clears the sink before the `Rc` that `install_sink` leaked is dropped.
            unsafe { ria_engine_sys::set_event_sink(None, std::ptr::null_mut()) };
            drop(unsafe { Rc::from_raw(context) });
        }
        for settle in state.pending.take().iter().filter_map(Weak::upgrade) {
            settle.finish(&state.queue, Err(CallError::Cancelled));
        }
    }

    /// Runs `handler` for each event of `topic`, on the drain after the event arrives.
    pub fn on_event(&self, topic: &str, handler: impl Fn(&Value) + 'static) {
        self.0
            .handlers
            .borrow_mut()
            .entry(topic.to_owned())
            .or_default()
            .push(Rc::new(handler));
    }

    /// Calls `name` for the profile in `profile_dir`. `completion` runs on a drain, exactly once.
    /// For a private profile the call gets `offTheRecord: true` (`ArdiaEngineBridge.mm:131-141`).
    pub fn call(
        &self,
        name: &str,
        profile_dir: &str,
        private: bool,
        args: Value,
        completion: impl FnOnce(CallResult) + 'static,
    ) {
        let mut args = args;
        if private {
            match args.as_object_mut() {
                Some(object) => _ = object.insert("offTheRecord".into(), Value::Bool(true)),
                None => return self.fail(completion, CallError::BadArguments),
            }
        }
        let Ok(text) = serde_json::to_string(&args) else {
            return self.fail(completion, CallError::BadArguments);
        };
        let Ok(text) = CString::new(text) else {
            return self.fail(completion, CallError::BadArguments);
        };
        self.start_call(name, profile_dir, text.as_ptr(), Box::new(completion));
    }

    /// Calls `name` with a secret argument buffer. The buffer must end in a NUL byte and is zeroed after the call,
    /// on every path. A private profile is refused before the engine call (`ArdiaEngineBridge.mm:150-171`).
    pub fn call_secret(
        &self,
        name: &str,
        profile_dir: &str,
        private: bool,
        mut args: Vec<u8>,
        completion: impl FnOnce(CallResult) + 'static,
    ) {
        if private {
            args.zeroize();
            return self.fail(completion, CallError::PrivateProfile);
        }
        if args.last() != Some(&0) {
            args.zeroize();
            return self.fail(completion, CallError::BadArguments);
        }
        self.start_call(
            name,
            profile_dir,
            args.as_ptr().cast(),
            Box::new(completion),
        );
        args.zeroize();
    }

    fn fail(&self, completion: impl FnOnce(CallResult) + 'static, error: CallError) {
        self.0.queue.push(Box::new(move || completion(Err(error))));
    }

    fn start_call(
        &self,
        name: &str,
        profile_dir: &str,
        args: *const c_char,
        completion: Completion,
    ) {
        let state = &self.0;
        let prepared = if !state.started.get() {
            Err(CallError::NotStarted)
        } else {
            self.symbol(name)
                .ok_or_else(|| CallError::UnknownCall(name.to_owned()))
                .and_then(|call| {
                    CString::new(profile_dir)
                        .map(|dir| (call, dir))
                        .map_err(|_| CallError::BadArguments)
                })
        };
        let (call, profile_dir) = match prepared {
            Ok(ready) => ready,
            Err(error) => return state.queue.push(Box::new(move || completion(Err(error)))),
        };

        let settle = Rc::new(Settle(Cell::new(Some(completion))));
        {
            let mut pending = state.pending.borrow_mut();
            pending.retain(|w| w.upgrade().is_some_and(|s| !s.is_settled()));
            pending.push(Rc::downgrade(&settle));
        }
        // The timeout only settles the cell; it never frees the context.
        let (timer_settle, timer_queue) = (settle.clone(), state.queue.clone());
        (state.schedule)(
            timeout_for(name),
            Box::new(move || timer_settle.finish(&timer_queue, Err(CallError::Timeout))),
        );

        let context = Box::into_raw(Box::new(Ctx {
            settle,
            queue: state.queue.clone(),
        }));
        // SAFETY: both strings outlive the call (engine-threads.md, section 3); the reply may run before this returns.
        unsafe {
            call(
                profile_dir.as_ptr(),
                args,
                replied as ReplyFn,
                context.cast(),
            )
        };
    }

    fn symbol(&self, name: &str) -> Option<CallFn> {
        let state = &self.0;
        if let Some(call) = state.symbols.borrow().get(name) {
            return Some(*call);
        }
        let call = (state.lookup)(name)?;
        state.symbols.borrow_mut().insert(name.to_owned(), call);
        Some(call)
    }

    fn install_sink(&self) {
        let state = &self.0;
        if !state.sink_context.get().is_null() {
            return;
        }
        let context = Rc::into_raw(state.clone());
        // SAFETY: `context` stays valid until `shutdown` clears the sink.
        if unsafe { ria_engine_sys::set_event_sink(Some(event_sink), context.cast_mut().cast()) } {
            state.sink_context.set(context);
        } else {
            drop(unsafe { Rc::from_raw(context) });
        }
    }
}

unsafe extern "C" fn event_sink(context: *mut c_void, topic: *const c_char, json: *const c_char) {
    if context.is_null() || topic.is_null() {
        return;
    }
    // SAFETY: `context` is the `Rc<State>` pointer of `install_sink`, alive until `shutdown`; the sink runs on the main thread.
    let state = unsafe { &*context.cast::<State>() };
    // The sink can run inside an engine call, so it only copies and queues.
    let topic = unsafe { CStr::from_ptr(topic) }
        .to_string_lossy()
        .into_owned();
    let payload = if json.is_null() {
        Value::Null
    } else {
        serde_json::from_slice(unsafe { CStr::from_ptr(json) }.to_bytes()).unwrap_or(Value::Null)
    };
    let handlers: Vec<Handler> = state
        .handlers
        .borrow()
        .get(&topic)
        .cloned()
        .unwrap_or_default();
    if handlers.is_empty() {
        return;
    }
    state
        .queue
        .push(Box::new(move || handlers.iter().for_each(|h| h(&payload))));
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    #[link_name = "pthread_main_np"]
    fn libc_main_np() -> std::ffi::c_int;
}

/// Main-queue helpers of libdispatch.
#[cfg(target_os = "macos")]
pub mod main_queue {
    use std::ffi::c_void;
    use std::time::Duration;

    unsafe extern "C" {
        static _dispatch_main_q: c_void;
        fn dispatch_time(when: u64, delta: i64) -> u64;
        fn dispatch_after_f(
            when: u64,
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
        fn dispatch_async_f(
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
    }

    extern "C" fn run(context: *mut c_void) {
        // SAFETY: `context` is the `Box<Box<dyn FnOnce()>>` made in `after` or `post`; libdispatch runs the work once.
        let work = unsafe { Box::from_raw(context.cast::<Box<dyn FnOnce()>>()) };
        work();
    }

    /// Runs `work` on the main queue after `delay`.
    pub fn after(delay: Duration, work: Box<dyn FnOnce()>) {
        let context = Box::into_raw(Box::new(work)).cast();
        let nanos = i64::try_from(delay.as_nanos()).unwrap_or(i64::MAX);
        // SAFETY: `DISPATCH_TIME_NOW` is 0; `_dispatch_main_q` is the main queue (`dispatch_get_main_queue()`).
        unsafe {
            dispatch_after_f(
                dispatch_time(0, nanos),
                &raw const _dispatch_main_q,
                context,
                run,
            )
        };
    }

    /// Runs `work` on the main queue as soon as it can. This is the one way for another thread to reach the main thread.
    pub fn post(work: Box<dyn FnOnce()>) {
        let context = Box::into_raw(Box::new(work)).cast();
        // SAFETY: as in `after`.
        unsafe { dispatch_async_f(&raw const _dispatch_main_q, context, run) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_char;

    thread_local! {
        static TIMERS: RefCell<Vec<(Duration, Action)>> = const { RefCell::new(Vec::new()) };
        /// The reply and context that `later` keeps, to answer after the call returned.
        static HELD: Cell<Option<(ReplyFn, usize)>> = const { Cell::new(None) };
    }

    fn schedule(delay: Duration, work: Action) {
        TIMERS.with(|t| t.borrow_mut().push((delay, work)));
    }

    fn fire_timers() {
        for (_, work) in TIMERS.with(|t| std::mem::take(&mut *t.borrow_mut())) {
            work();
        }
    }

    unsafe extern "C" fn sync_call(
        _: *const c_char,
        _: *const c_char,
        reply: ReplyFn,
        context: *mut c_void,
    ) {
        unsafe { reply(context, c"{\"n\":1}".as_ptr()) };
    }

    unsafe extern "C" fn later_call(
        _: *const c_char,
        _: *const c_char,
        reply: ReplyFn,
        context: *mut c_void,
    ) {
        HELD.set(Some((reply, context as usize)));
    }

    /// Answers with its own arguments, so a test sees what the engine would get.
    unsafe extern "C" fn echo_call(
        _: *const c_char,
        args: *const c_char,
        reply: ReplyFn,
        context: *mut c_void,
    ) {
        unsafe { reply(context, args) };
    }

    unsafe extern "C" fn error_call(
        _: *const c_char,
        _: *const c_char,
        reply: ReplyFn,
        context: *mut c_void,
    ) {
        unsafe { reply(context, c"{\"error\":\"no bookmarks\"}".as_ptr()) };
    }

    fn lookup(name: &str) -> Option<CallFn> {
        match name {
            "sync" => Some(sync_call),
            "later" => Some(later_call),
            "echo" => Some(echo_call),
            "error" => Some(error_call),
            _ => None,
        }
    }

    fn engine() -> (Engine, Rc<ActionQueue>) {
        TIMERS.with(|t| t.borrow_mut().clear());
        let queue = ActionQueue::new();
        let engine = Engine::with(queue.clone(), lookup, schedule);
        engine.did_start();
        (engine, queue)
    }

    type Log = Rc<RefCell<Vec<CallResult>>>;

    fn call(engine: &Engine, name: &str, private: bool, args: Value) -> Log {
        let log = Log::default();
        let sink = log.clone();
        engine.call(name, "/profile", private, args, move |r| {
            sink.borrow_mut().push(r)
        });
        log
    }

    #[test]
    fn reply_before_the_call_returns_settles_once() {
        let (engine, queue) = engine();
        let log = call(&engine, "sync", false, serde_json::json!({}));
        // The reply already ran, but the completion waits for the drain.
        assert!(log.borrow().is_empty());
        assert_eq!(queue.drain(), 1);
        assert_eq!(*log.borrow(), vec![Ok(serde_json::json!({"n": 1}))]);
        // The timer that fires now finds the cell settled.
        fire_timers();
        assert_eq!(queue.drain(), 0);
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn reply_after_the_timeout_only_frees_the_context() {
        let (engine, queue) = engine();
        let log = call(&engine, "later", false, serde_json::json!({}));
        fire_timers();
        queue.drain();
        assert_eq!(*log.borrow(), vec![Err(CallError::Timeout)]);

        let (reply, context) = HELD.take().expect("the engine kept the reply");
        unsafe { reply(context as *mut c_void, c"{\"late\":true}".as_ptr()) };
        assert_eq!(queue.drain(), 0);
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn shutdown_cancels_pending_calls_and_refuses_new_ones() {
        let (engine, queue) = engine();
        let pending = call(&engine, "later", false, serde_json::json!({}));
        engine.shutdown();
        queue.drain();
        assert_eq!(*pending.borrow(), vec![Err(CallError::Cancelled)]);
        let after = call(&engine, "sync", false, serde_json::json!({}));
        queue.drain();
        assert_eq!(*after.borrow(), vec![Err(CallError::NotStarted)]);
        // The late reply still frees the context.
        let (reply, context) = HELD.take().expect("held");
        unsafe { reply(context as *mut c_void, c"{}".as_ptr()) };
        assert_eq!(queue.drain(), 0);
    }

    #[test]
    fn private_profile_adds_off_the_record_and_engine_errors_come_back() {
        let (engine, queue) = engine();
        let plain = call(&engine, "echo", false, serde_json::json!({"a": 1}));
        let private = call(&engine, "echo", true, serde_json::json!({"a": 1}));
        let failed = call(&engine, "error", false, serde_json::json!({}));
        let unknown = call(&engine, "nope", false, serde_json::json!({}));
        queue.drain();
        assert_eq!(*plain.borrow(), vec![Ok(serde_json::json!({"a": 1}))]);
        assert_eq!(
            *private.borrow(),
            vec![Ok(serde_json::json!({"a": 1, "offTheRecord": true}))]
        );
        assert_eq!(
            *failed.borrow(),
            vec![Err(CallError::Engine("no bookmarks".into()))]
        );
        assert_eq!(
            *unknown.borrow(),
            vec![Err(CallError::UnknownCall("nope".into()))]
        );
    }

    #[test]
    fn secret_calls_refuse_private_profiles_and_missing_nul() {
        let (engine, queue) = engine();
        let log = Log::default();
        for (private, bytes) in [
            (true, b"{}\0".to_vec()),
            (false, b"{}".to_vec()),
            (false, b"{\"p\":1}\0".to_vec()),
        ] {
            let sink = log.clone();
            engine.call_secret("echo", "/profile", private, bytes, move |r| {
                sink.borrow_mut().push(r)
            });
        }
        queue.drain();
        assert_eq!(
            *log.borrow(),
            vec![
                Err(CallError::PrivateProfile),
                Err(CallError::BadArguments),
                Ok(serde_json::json!({"p": 1}))
            ]
        );
    }

    #[test]
    fn the_logins_call_has_the_ardia_timeout() {
        assert_eq!(timeout_for("ac_passwords_logins"), Duration::from_secs(30));
        assert_eq!(timeout_for("ac_bookmarks_tree"), DEFAULT_TIMEOUT);
    }
}
