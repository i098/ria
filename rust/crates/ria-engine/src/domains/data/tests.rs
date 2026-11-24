// Ria original, no Ardia source.

//! Tests of the data domains with a fake engine, fake profiles, fake services and fake page engines. They port the 2
//! cases of `clear-browsing-data.test.mjs` and test the call tables and the profile data steps.

#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::rc::Rc;
use std::time::Duration;

use ria_engine_sys::{CallFn, ReplyFn};
use serde_json::{Value, json};

use crate::call::{ActionQueue, Engine};

use super::call_table::{CallReply, Settled};
use super::ctx::DataCtx;
use super::favicons::Favicons;
use super::host::{CoreEngine, CoreProfile, EventSink, PageEngine, Profile, ServiceDone, Services};
use super::page_engines::clear_chromium_browsing_data;
use super::profile_data::delete_default;
use super::profiles::ProfileRegistry;
use super::tables::tables;

thread_local! {
    static ANSWERS: RefCell<HashMap<String, Value>> = RefCell::default();
    static CALLS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    static TIMERS: RefCell<Vec<Box<dyn FnOnce()>>> = const { RefCell::new(Vec::new()) };
}

unsafe extern "C" fn answer(
    path: *const c_char,
    args: *const c_char,
    reply: ReplyFn,
    context: *mut c_void,
) {
    let args = unsafe { CStr::from_ptr(args) }
        .to_string_lossy()
        .into_owned();
    let path = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();
    CALLS.with(|c| c.borrow_mut().push((path, args.clone())));
    // The fake engine answers with the text that a test queued for the symbol, or with the arguments.
    let text = ANSWERS
        .with(|a| a.borrow().get("*").cloned())
        .map_or(args, |v| v.to_string());
    let text = std::ffi::CString::new(text).unwrap();
    unsafe { reply(context, text.as_ptr()) };
}

fn lookup(name: &str) -> Option<CallFn> {
    name.starts_with("ac_").then_some(answer as CallFn)
}

fn schedule(_: Duration, work: Box<dyn FnOnce()>) {
    TIMERS.with(|t| t.borrow_mut().push(work));
}

fn fire_timers() {
    loop {
        let next = TIMERS.with(|t| {
            let mut timers = t.borrow_mut();
            (!timers.is_empty()).then(|| timers.remove(0))
        });
        match next {
            Some(work) => work(),
            None => break,
        }
    }
}

struct FakeProfile {
    directory: String,
    private: bool,
}

impl CoreProfile for FakeProfile {
    fn path(&self) -> String {
        format!("/data/{}", self.directory)
    }
    fn directory_name(&self) -> String {
        self.directory.clone()
    }
    fn off_the_record(&self) -> bool {
        self.private
    }
    fn destroyed(&self) -> bool {
        false
    }
    fn fetch_icon(&self, _: &str, _: usize, done: Box<dyn FnOnce(Option<Vec<u8>>)>) {
        done(None)
    }
    fn load_component_extension(&self, _: &str) -> String {
        String::new()
    }
    fn hold_navigations_until_rulesets_of(&self, _: &str) {}
    fn release_navigation_hold_ready(&self) {}
    fn release_navigation_hold(&self, _: &str) {}
}

type BaseLoad = Box<dyn FnOnce(Option<Profile>)>;
type PrivateLoad = Box<dyn FnOnce(Option<Profile>, Option<String>)>;

#[derive(Default)]
struct FakeCore {
    default: Option<Profile>,
    defer_base_load: bool,
    pending_base: RefCell<Option<BaseLoad>>,
    pending_private: RefCell<Option<PrivateLoad>>,
    released: RefCell<Vec<Profile>>,
}

impl CoreEngine for FakeCore {
    fn default_profile(&self) -> Option<Profile> {
        self.default.clone()
    }
    fn load_profile(&self, directory: &str, done: BaseLoad) {
        if self.defer_base_load {
            assert!(self.pending_base.replace(Some(done)).is_none());
        } else {
            done(Some(Rc::new(FakeProfile {
                directory: directory.into(),
                private: false,
            })));
        }
    }
    fn off_the_record_profile_for(&self, _: &Profile, done: PrivateLoad) {
        assert!(self.pending_private.replace(Some(done)).is_none());
    }
    fn release_profile(&self, profile: &Profile) {
        self.released.borrow_mut().push(profile.clone());
    }
}

struct NoEvents;

impl EventSink for NoEvents {
    fn emit(&self, _: &str, _: Value) {}
}

#[derive(Default)]
struct FakeServices {
    log: RefCell<Vec<(String, String, Value)>>,
    answers: RefCell<HashMap<String, Result<Value, String>>>,
}

impl Services for FakeServices {
    fn run(&self, op: &str, profile: &str, args: Value, done: ServiceDone) {
        self.log
            .borrow_mut()
            .push((op.into(), profile.into(), args));
        done(
            self.answers
                .borrow()
                .get(op)
                .cloned()
                .unwrap_or(Ok(Value::Null)),
        );
    }
}

type ClearDone = Box<dyn FnOnce(Option<String>)>;

/// A page engine that keeps the completions of `clear_browsing_data` for the test to answer.
#[derive(Default)]
struct FakeEngine {
    prefix: &'static str,
    cleared: RefCell<Vec<(String, Vec<String>, f64)>>,
    pending: RefCell<Option<ClearDone>>,
    commands: RefCell<Vec<String>>,
}

impl PageEngine for FakeEngine {
    fn owns(&self, id: &str) -> bool {
        id.starts_with(self.prefix)
    }
    fn resolve_permission(&self, id: &str, result: &str, remember: bool) {
        self.commands
            .borrow_mut()
            .push(format!("permission {id} {result} {remember}"));
    }
    fn resolve_external_app(&self, _: &str, _: bool, _: bool) {}
    fn download_command(&self, action: &str, id: &str) {
        self.commands.borrow_mut().push(format!("{action} {id}"));
    }
    fn external_app_allowances(&self, _: &str, done: Box<dyn FnOnce(Vec<Value>)>) {
        done(vec![json!({ "origin": "a", "scheme": "x" })])
    }
    fn remove_external_app_allowance(&self, _: &str, _: &str, _: &str) {}
    fn site_settings(&self, _: &str, _: &str, done: ServiceDone) {
        done(Ok(Value::Null))
    }
    fn set_site_setting(&self, _: &str, _: &str, _: &str, _: &str) {}
    fn clear_site_data(&self, _: &str, _: &str, done: ServiceDone) {
        done(Ok(Value::Null))
    }
    fn clear_browsing_data(
        &self,
        profile: &str,
        types: &[String],
        since: f64,
        done: Box<dyn FnOnce(Option<String>)>,
    ) {
        self.cleared
            .borrow_mut()
            .push((profile.into(), types.to_vec(), since));
        *self.pending.borrow_mut() = Some(done);
    }
    fn delete_profile_data(&self, _: &str, done: Box<dyn FnOnce(Vec<String>)>) {
        done(vec![])
    }
}

struct Rig {
    ctx: DataCtx,
    queue: Rc<ActionQueue>,
    services: Rc<FakeServices>,
    chromium: Rc<FakeEngine>,
    webkit: Rc<FakeEngine>,
}

fn rig() -> Rig {
    rig_with_core(Rc::new(FakeCore::default()))
}

fn rig_with_core(core: Rc<FakeCore>) -> Rig {
    ANSWERS.with(|a| a.borrow_mut().clear());
    CALLS.with(|c| c.borrow_mut().clear());
    TIMERS.with(|t| t.borrow_mut().clear());
    let queue = ActionQueue::new();
    let engine = Engine::with(queue.clone(), lookup, schedule);
    engine.did_start();
    let services = Rc::new(FakeServices::default());
    let chromium = Rc::new(FakeEngine::default());
    let webkit = Rc::new(FakeEngine {
        prefix: "wk-",
        ..FakeEngine::default()
    });
    let registry = ProfileRegistry::new(core, Rc::new(NoEvents));
    let ctx = DataCtx {
        engine,
        favicons: Rc::new(Favicons::new(registry.clone(), std::env::temp_dir())),
        registry,
        services: services.clone(),
        chromium: chromium.clone(),
        webkit: webkit.clone(),
        after: |delay, work| {
            // The mover hop and the zoom poll run when a test says so.
            let _ = delay;
            schedule(delay, work)
        },
    };
    Rig {
        ctx,
        queue,
        services,
        chromium,
        webkit,
    }
}

fn run_table(
    rig: &Rig,
    domain: &str,
    name: &str,
    profile: &str,
    args: Value,
) -> Rc<RefCell<Option<Settled>>> {
    let out = Rc::new(RefCell::new(None));
    let sink = out.clone();
    let all = tables();
    let table = all.iter().find(|t| t.domain == domain).expect("a table");
    table.run(
        &rig.ctx,
        name,
        profile,
        &args.to_string(),
        CallReply::new(move |s| *sink.borrow_mut() = Some(s)),
    );
    rig.queue.drain();
    out
}

#[test]
fn bookmark_export_suggests_a_ria_html_filename() {
    let rig = rig();
    let reply = run_table(&rig, "bookmarks", "exportHtml", "work", json!({}));
    assert_eq!(
        rig.services.log.borrow().as_slice(),
        &[(
            "savePanel".into(),
            "work".into(),
            json!({"name": "Ria Bookmarks.html", "type": "html"}),
        )]
    );
    assert_eq!(reply.take(), Some(Ok("null".into())));
}

#[test]
fn the_service_accepts_only_boolean_true_and_keeps_engine_errors() {
    let answers = [
        (json!({"ok": true}), None),
        (
            json!({"ok": false}),
            Some("Some browsing data could not be cleared."),
        ),
        (json!({}), Some("Some browsing data could not be cleared.")),
        (
            json!({"ok": 1}),
            Some("Some browsing data could not be cleared."),
        ),
        (
            json!({"ok": "true"}),
            Some("Some browsing data could not be cleared."),
        ),
        (
            json!({"ok": null}),
            Some("Some browsing data could not be cleared."),
        ),
        (
            json!({"error": "engine refused", "ok": true}),
            Some("engine refused"),
        ),
    ];
    let rig = rig();
    for (answer, expected) in &answers {
        ANSWERS.with(|a| a.borrow_mut().insert("*".into(), answer.clone()));
        let replies = Rc::new(RefCell::new(Vec::new()));
        let sink = replies.clone();
        let types = ["history".to_owned(), "cache".to_owned()];
        clear_chromium_browsing_data(
            &rig.ctx,
            "unloaded-profile",
            &types,
            1_234_567.0,
            Box::new(move |e| sink.borrow_mut().push(e)),
        );
        rig.queue.drain();
        assert_eq!(*replies.borrow(), vec![expected.map(str::to_owned)]);
    }
    let sent: Value = serde_json::from_str(&CALLS.with(|c| c.borrow()[0].1.clone())).unwrap();
    assert_eq!(
        sent,
        json!({"types": ["history", "cache"], "since": 1_234_567.0})
    );
    let forgot = rig
        .services
        .log
        .borrow()
        .iter()
        .filter(|(op, ..)| op == "forgetPictures")
        .count();
    assert_eq!(forgot, answers.len());
}

#[test]
fn the_aggregate_waits_for_both_engines_and_rejects_the_module_call_on_failure() {
    for webkit_first in [false, true] {
        for (first_error, second_error) in [
            (None, None),
            (Some("first engine failed"), None),
            (None, Some("second engine failed")),
            (Some("first engine failed"), Some("second engine failed")),
        ] {
            let rig = rig();
            let out = run_table(
                &rig,
                "profiles",
                "clearBrowsingData",
                "requested-profile",
                json!({"types": ["history", "siteData"], "since": 1_234_567}),
            );
            for engine in [&rig.webkit, &rig.chromium] {
                assert_eq!(
                    *engine.cleared.borrow(),
                    vec![(
                        "requested-profile".into(),
                        vec!["history".to_owned(), "siteData".to_owned()],
                        1_234_567.0
                    )]
                );
            }
            let (first, second) = if webkit_first {
                (&rig.webkit, &rig.chromium)
            } else {
                (&rig.chromium, &rig.webkit)
            };
            first.pending.take().unwrap()(first_error.map(str::to_owned));
            fire_timers();
            assert!(
                out.borrow().is_none(),
                "no reply before both engines answered"
            );
            second.pending.take().unwrap()(second_error.map(str::to_owned));
            fire_timers();
            let expected = match first_error.or(second_error) {
                None => Ok("null".to_owned()),
                Some(e) => Err(super::call_table::Rejection::new("ERR_ENGINE", e)),
            };
            assert_eq!(out.borrow_mut().take(), Some(expected));
        }
    }
}

#[test]
fn engine_passing_calls_add_off_the_record_and_use_the_original_profile() {
    let rig = rig();
    ANSWERS.with(|a| {
        a.borrow_mut()
            .insert("*".into(), json!({"bar": {}, "other": {}}))
    });
    let out = run_table(
        &rig,
        "bookmarks",
        "tree",
        "incognito1@work",
        json!({"q": 1}),
    );
    assert_eq!(
        out.borrow_mut().take(),
        Some(Ok(r#"{"bar":{},"other":{}}"#.to_owned()))
    );
    let (path, args) = CALLS.with(|c| c.borrow()[0].clone());
    assert_eq!(path, "/data/Profile work");
    assert_eq!(
        serde_json::from_str::<Value>(&args).unwrap(),
        json!({"q": 1, "offTheRecord": true})
    );
    ANSWERS.with(|a| {
        a.borrow_mut()
            .insert("*".into(), json!({"error": "no bookmarks"}))
    });
    let out = run_table(&rig, "bookmarks", "tree", "", json!({}));
    assert_eq!(
        out.borrow_mut().take(),
        Some(Err(super::call_table::Rejection::new(
            "ERR_ENGINE",
            "no bookmarks"
        )))
    );
}

#[test]
fn bookmark_dispatch_rejects_an_unloaded_profile_without_calling_the_engine() {
    let core = Rc::new(FakeCore {
        defer_base_load: true,
        ..FakeCore::default()
    });
    let rig = rig_with_core(core.clone());
    let out = run_table(&rig, "bookmarks", "tree", "missing", json!({}));
    core.pending_base.take().expect("requested profile load")(None);
    rig.queue.drain();
    assert_eq!(
        out.take(),
        Some(Err(super::call_table::Rejection::new(
            "ERR_ENGINE",
            "profile not loaded",
        )))
    );
    CALLS.with(|calls| assert!(calls.borrow().is_empty()));
}

#[test]
fn regular_profile_registration_notifies_hooks_once_before_lookup_returns() {
    use super::host::ProfileLoadHooks;

    #[derive(Default)]
    struct Hooks(RefCell<Vec<String>>);
    impl ProfileLoadHooks for Hooks {
        fn profile_loaded(&self, name: &str) {
            self.0.borrow_mut().push(name.into());
        }
    }

    let core = Rc::new(FakeCore {
        default: Some(Rc::new(FakeProfile {
            directory: "Default".into(),
            private: false,
        })),
        ..FakeCore::default()
    });
    let registry = ProfileRegistry::new(core, Rc::new(NoEvents));
    let hooks = Rc::new(Hooks::default());
    registry.set_load_hooks(hooks.clone());
    for (name, expected) in [("", &[""][..]), ("work", &["", "work"][..])] {
        for _ in 0..2 {
            let hooks = hooks.clone();
            let completed = Rc::new(Cell::new(false));
            let callback_completed = completed.clone();
            registry.with_profile(name, move |profile| {
                assert!(profile.is_some());
                assert_eq!(hooks.0.borrow().as_slice(), expected);
                callback_completed.set(true);
            });
            assert!(completed.get(), "lookup did not complete for {name:?}");
        }
    }
    assert_eq!(hooks.0.borrow().as_slice(), ["", "work"]);
}

#[test]
fn releasing_a_private_session_settles_waiters_and_releases_the_late_profile() {
    let core = Rc::new(FakeCore::default());
    let registry = ProfileRegistry::new(core.clone(), Rc::new(NoEvents));
    let session = "incognito1@work";
    let replies = Rc::new(RefCell::new(Vec::new()));
    for _ in 0..2 {
        let replies = replies.clone();
        registry.with_profile(session, move |profile| {
            replies.borrow_mut().push(profile.is_some());
        });
    }
    let finish = core.pending_private.take().expect("private load started");
    registry.mark_private_released(session);
    assert_eq!(*replies.borrow(), [false, false]);

    let late: Profile = Rc::new(FakeProfile {
        directory: "Profile work".into(),
        private: true,
    });
    finish(Some(late.clone()), None);
    assert_eq!(*replies.borrow(), [false, false]);
    assert!(matches!(&core.released.borrow()[..], [one] if Rc::ptr_eq(one, &late)));
    assert!(registry.loaded_profile(session).is_none());
    let replies_again = replies.clone();
    registry.with_profile(session, move |profile| {
        replies_again.borrow_mut().push(profile.is_some());
    });
    assert_eq!(*replies.borrow(), [false, false, false]);
}

#[test]
fn releasing_a_cached_private_session_stops_lookup() {
    let core = Rc::new(FakeCore::default());
    let registry = ProfileRegistry::new(core.clone(), Rc::new(NoEvents));
    let session = "incognito1@work";
    let profile: Profile = Rc::new(FakeProfile {
        directory: "Profile work".into(),
        private: true,
    });
    let reply = Rc::new(RefCell::new(None));
    let first = reply.clone();
    registry.with_profile(session, move |resolved| {
        *first.borrow_mut() = Some(resolved)
    });
    core.pending_private.take().unwrap()(Some(profile.clone()), None);
    let resolved = reply.take().expect("the first lookup settled").unwrap();
    assert!(Rc::ptr_eq(&resolved, &profile));
    assert!(Rc::ptr_eq(
        &registry.loaded_profile(session).unwrap(),
        &profile
    ));
    registry.mark_private_released(session);
    assert!(registry.loaded_profile(session).is_none());
    let second = reply.clone();
    registry.with_profile(session, move |resolved| {
        *second.borrow_mut() = Some(resolved)
    });
    assert!(matches!(reply.take(), Some(None)));
    assert!(core.pending_private.borrow().is_none());
}

#[test]
fn releasing_a_session_before_its_base_loads_skips_private_creation() {
    let core = Rc::new(FakeCore {
        defer_base_load: true,
        ..FakeCore::default()
    });
    let registry = ProfileRegistry::new(core.clone(), Rc::new(NoEvents));
    let session = "incognito1@work";
    let settled = Rc::new(RefCell::new(None));
    let reply = settled.clone();
    registry.with_profile(session, move |profile| {
        *reply.borrow_mut() = Some(profile.is_some());
    });
    let finish = core.pending_base.take().expect("base load started");
    assert_eq!(*settled.borrow(), None);
    registry.mark_private_released(session);
    assert_eq!(*settled.borrow(), Some(false));
    finish(Some(Rc::new(FakeProfile {
        directory: "Profile work".into(),
        private: false,
    })));
    assert!(registry.loaded_profile("work").is_some());
    assert!(core.pending_private.borrow().is_none());
}

#[test]
fn permissions_and_downloads_go_to_the_engine_that_owns_the_id() {
    let rig = rig();
    run_table(&rig, "downloads", "pause", "", json!({"id": "wk-1"}));
    run_table(&rig, "downloads", "cancel", "", json!({"id": "7"}));
    run_table(
        &rig,
        "permissions",
        "resolve",
        "",
        json!({"id": "wk-2", "result": "deny", "remember": true}),
    );
    assert_eq!(
        *rig.webkit.commands.borrow(),
        vec!["pause wk-1", "permission wk-2 deny true"]
    );
    assert_eq!(*rig.chromium.commands.borrow(), vec!["cancel 7"]);
    let bad = run_table(
        &rig,
        "permissions",
        "resolve",
        "",
        json!({"id": "1", "result": "maybe", "remember": true}),
    );
    assert_eq!(
        bad.borrow_mut().take(),
        Some(Err(super::call_table::Rejection::new(
            "ERR_BAD_ARGS",
            "permissions.resolve: result isn't one of the values it takes"
        )))
    );
}

#[test]
fn autofill_settings_default_to_true_and_errors_pass_through() {
    let rig = rig();
    rig.services
        .answers
        .borrow_mut()
        .insert("profileToggles".into(), Ok(json!({"addresses": false})));
    let out = run_table(&rig, "autofill", "settings", "p", json!({}));
    assert_eq!(
        out.borrow_mut().take(),
        Some(Ok(r#"{"addresses":false,"cards":true}"#.to_owned()))
    );
    rig.services
        .answers
        .borrow_mut()
        .insert("profileToggles".into(), Ok(Value::Null));
    let out = run_table(&rig, "autofill", "settings", "p", json!({}));
    assert_eq!(
        out.borrow_mut().take(),
        Some(Err(super::call_table::Rejection::new(
            "ERR_ENGINE",
            "no autofill settings"
        )))
    );
    run_table(
        &rig,
        "autofill",
        "setSettings",
        "p",
        json!({"cards": false}),
    );
    let log = rig.services.log.borrow();
    assert_eq!(log.last().unwrap().2, json!({"set": {"cards": false}}));
}

#[test]
fn deleting_the_default_profile_lists_the_steps_that_did_not_finish() {
    let rig = rig();
    // Every list stays full, so the steps with a list report themselves; the chromium steps answer `ok`.
    ANSWERS.with(|a| a.borrow_mut().insert("*".into(), json!({"ok": true})));
    let full = json!({"passwords": [{"origin": "o", "username": "u"}], "addresses": [{"id": "a"}], "cards": [{"id": "c"}]});
    let mut answers = rig.services.answers.borrow_mut();
    answers.insert("listPasswords".into(), Ok(full.clone()));
    answers.insert("addresses".into(), Ok(full.clone()));
    answers.insert("cards".into(), Ok(full.clone()));
    answers.insert("neverSaveOrigins".into(), Ok(json!({"origins": []})));
    answers.insert("listExtensions".into(), Ok(json!({"extensions": []})));
    answers.insert("siteSettingsOrigins".into(), Ok(json!([])));
    answers.insert("zoomLevels".into(), Ok(json!({"a.com": 2})));
    drop(answers);
    let remaining = Rc::new(RefCell::new(None));
    let sink = remaining.clone();
    delete_default(
        rig.ctx.clone(),
        Box::new(move |r| *sink.borrow_mut() = Some(r)),
    );
    rig.queue.drain();
    for _ in 0..30 {
        fire_timers();
        rig.queue.drain();
    }
    assert_eq!(
        remaining.take(),
        Some(vec![
            "passwords".to_owned(),
            "addresses".into(),
            "cards".into(),
            "zoom levels".into()
        ])
    );
    let zoom = rig
        .services
        .log
        .borrow()
        .iter()
        .filter(|(op, ..)| op == "zoomLevels")
        .count();
    assert_eq!(zoom, 1 + 21, "the poll stops after 20 retries");
}

#[test]
fn a_failed_service_read_is_a_failed_step_not_empty_work() {
    let cases = [
        ("listPasswords", "passwords"),
        ("neverSaveOrigins", "passwords"),
        ("addresses", "addresses"),
        ("cards", "cards"),
        ("listExtensions", "extensions"),
        ("siteSettingsOrigins", "site settings"),
        ("zoomLevels", "zoom levels"),
    ];
    for (op, label) in cases {
        let rig = rig();
        ANSWERS.with(|a| a.borrow_mut().insert("*".into(), json!({"ok": true})));
        let mut answers = rig.services.answers.borrow_mut();
        answers.insert("listPasswords".into(), Ok(json!({"passwords": []})));
        answers.insert("addresses".into(), Ok(json!({"addresses": []})));
        answers.insert("cards".into(), Ok(json!({"cards": []})));
        answers.insert("neverSaveOrigins".into(), Ok(json!({"origins": []})));
        answers.insert("listExtensions".into(), Ok(json!({"extensions": []})));
        answers.insert("siteSettingsOrigins".into(), Ok(json!([])));
        answers.insert("zoomLevels".into(), Ok(json!({})));
        answers.insert(op.into(), Err("service failed".into()));
        drop(answers);
        let remaining = Rc::new(RefCell::new(None));
        let sink = remaining.clone();
        delete_default(
            rig.ctx.clone(),
            Box::new(move |r| *sink.borrow_mut() = Some(r)),
        );
        rig.queue.drain();
        for _ in 0..30 {
            fire_timers();
            rig.queue.drain();
        }
        assert_eq!(remaining.take(), Some(vec![label.to_owned()]), "{op}");
    }
}

#[test]
fn an_unknown_call_and_a_missing_argument_use_the_ardia_codes() {
    let rig = rig();
    let out = run_table(&rig, "sites", "nope", "", json!({}));
    assert_eq!(
        out.borrow_mut().take(),
        Some(Err(super::call_table::Rejection::new(
            "ERR_UNKNOWN_CALL",
            "sites.nope isn't a call"
        )))
    );
    let out = run_table(&rig, "sites", "setZoom", "", json!({"zoom": 2}));
    assert_eq!(
        out.borrow_mut().take(),
        Some(Err(super::call_table::Rejection::new(
            "ERR_BAD_ARGS",
            "sites.setZoom: host is missing"
        )))
    );
    let _ = Cell::new(0);
}
