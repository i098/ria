// Ria original, no Ardia source.

//! Tests and fakes of the content blocker port.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use super::*;
use crate::domains::data::content_blocker_page::{ExtensionTab, PageDelegate};

type Work = Box<dyn FnOnce()>;

pub struct FakeHost {
    pub started: Cell<bool>,
    now: Cell<Duration>,
    timers: RefCell<Vec<(Duration, Work)>>,
    posted: RefCell<Vec<Work>>,
    pub profiles: RefCell<HashMap<String, Profile>>,
    resources: PathBuf,
    pub changes: RefCell<Vec<Value>>,
    pub events: RefCell<Vec<(String, Value)>>,
}

fn temp_dir() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ria-blocker-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

impl FakeHost {
    pub fn new() -> Rc<Self> {
        let resources = temp_dir();
        let ublock = resources.join("Extensions").join("ublock");
        std::fs::create_dir_all(&ublock).unwrap();
        std::fs::write(ublock.join("manifest.json"), r#"{"version":"1.2.3"}"#).unwrap();
        Rc::new(Self {
            started: Cell::new(true),
            now: Cell::default(),
            timers: RefCell::default(),
            posted: RefCell::default(),
            profiles: RefCell::default(),
            resources,
            changes: RefCell::default(),
            events: RefCell::default(),
        })
    }

    pub fn advance(&self, by: Duration) {
        self.now.set(self.now.get() + by);
    }

    /// Advances the clock and runs the timers that are due, in order.
    pub fn fire(&self, by: Duration) {
        self.advance(by);
        loop {
            let due = {
                let mut timers = self.timers.borrow_mut();
                let Some(i) = (0..timers.len())
                    .filter(|&i| timers[i].0 <= self.now.get())
                    .min_by_key(|&i| timers[i].0)
                else {
                    break;
                };
                timers.remove(i).1
            };
            due();
            self.run_posted();
        }
    }

    pub fn run_posted(&self) {
        loop {
            let posted = std::mem::take(&mut *self.posted.borrow_mut());
            if posted.is_empty() {
                return;
            }
            for work in posted {
                work();
            }
        }
    }
}

impl BlockerHost for FakeHost {
    fn is_started(&self) -> bool {
        self.started.get()
    }
    fn loaded_profile(&self, name: &str) -> Option<Profile> {
        self.profiles.borrow().get(name).cloned()
    }
    fn is_deleted_profile(&self, _: &str) -> bool {
        false
    }
    fn is_private_profile(&self, name: &str) -> bool {
        name.starts_with("private")
    }
    fn with_profile(&self, name: &str, done: Box<dyn FnOnce(Option<Profile>)>) {
        done(self.loaded_profile(name))
    }
    fn resource_path(&self) -> Option<PathBuf> {
        Some(self.resources.clone())
    }
    fn data_directory(&self) -> PathBuf {
        self.resources.join("data")
    }
    fn unload_component_extension(&self, _: &Profile, _: &str) {}
    fn did_change(&self, change: Value) {
        self.changes.borrow_mut().push(change);
    }
    fn emit(&self, name: &str, data: Value) {
        self.events.borrow_mut().push((name.to_owned(), data));
    }
    fn now(&self) -> Duration {
        self.now.get()
    }
    fn after(&self, delay: Duration, work: Work) {
        self.timers
            .borrow_mut()
            .push((self.now.get() + delay, work));
    }
    fn post(&self, work: Work) {
        self.posted.borrow_mut().push(work);
    }
}

#[derive(Default)]
pub struct FakeProfile {
    pub path: String,
    pub log: RefCell<Vec<String>>,
}

impl CoreProfile for FakeProfile {
    fn path(&self) -> String {
        self.path.clone()
    }
    fn directory_name(&self) -> String {
        "Default".into()
    }
    fn off_the_record(&self) -> bool {
        false
    }
    fn destroyed(&self) -> bool {
        false
    }
    fn fetch_icon(&self, _: &str, _: usize, done: Box<dyn FnOnce(Option<Vec<u8>>)>) {
        done(None)
    }
    fn load_component_extension(&self, _: &str) -> String {
        EXTENSION_ID.into()
    }
    fn hold_navigations_until_rulesets_of(&self, id: &str) {
        self.log.borrow_mut().push(format!("hold {id}"));
    }
    fn release_navigation_hold_ready(&self) {
        self.log.borrow_mut().push("ready".into());
    }
    fn release_navigation_hold(&self, reason: &str) {
        self.log.borrow_mut().push(format!("release {reason}"));
    }
}

pub struct FakeTab {
    pub url: RefCell<String>,
    pub loading: Cell<bool>,
    pub closed: Cell<bool>,
    delegate: RefCell<Option<PageDelegate>>,
    pub scripts: RefCell<Vec<String>>,
    pending: RefCell<VecDeque<EvalDone>>,
}

impl FakeTab {
    pub fn new(url: &str) -> Rc<Self> {
        Rc::new(Self {
            url: RefCell::new(url.into()),
            loading: Cell::new(false),
            closed: Cell::new(false),
            delegate: RefCell::default(),
            scripts: RefCell::default(),
            pending: RefCell::default(),
        })
    }

    pub fn delegate(&self) -> PageDelegate {
        self.delegate.borrow().clone().expect("a page took the tab")
    }

    pub fn load(&self) {
        self.loading.set(true);
        self.delegate().did_change_loading();
        self.loading.set(false);
        self.delegate().did_change_loading();
    }

    /// Answers the oldest script that runs.
    pub fn answer(&self, json: &str) {
        let done = self
            .pending
            .borrow_mut()
            .pop_front()
            .expect("a script runs");
        done(json.into());
    }
}

impl ExtensionTab for FakeTab {
    fn url(&self) -> String {
        self.url.borrow().clone()
    }
    fn loading(&self) -> bool {
        self.loading.get()
    }
    fn closed(&self) -> bool {
        self.closed.get()
    }
    fn set_delegate(&self, delegate: PageDelegate) {
        *self.delegate.borrow_mut() = Some(delegate);
    }
    fn evaluate(&self, script: &str, done: Box<dyn FnOnce(String)>) {
        self.scripts.borrow_mut().push(script.into());
        self.pending.borrow_mut().push_back(done);
    }
    fn close_now(&self) {
        self.closed.set(true);
    }
}

#[derive(Default)]
pub struct FakeScripts {
    tabs: RefCell<Vec<Rc<FakeTab>>>,
}

impl FakeScripts {
    pub fn tab(&self, i: usize) -> Rc<FakeTab> {
        self.tabs.borrow()[i].clone()
    }
}

impl ExtensionScripts for FakeScripts {
    fn open_tab(&self, _: &str, _: &Profile, url: &str) -> Option<Rc<dyn ExtensionTab>> {
        let tab = FakeTab::new(url);
        self.tabs.borrow_mut().push(tab.clone());
        Some(tab)
    }
}

type EvalDone = Box<dyn FnOnce(String)>;
type Answer = (Option<Value>, Option<String>);

#[derive(Default, Clone)]
pub struct Answers(Rc<RefCell<Vec<Answer>>>);

impl Answers {
    pub fn done(&self) -> Done {
        let sink = self.0.clone();
        Box::new(move |v, e| sink.borrow_mut().push((v, e)))
    }
    pub fn take(&self) -> Vec<(Option<Value>, Option<String>)> {
        self.0.take()
    }
}

/// A blocker with a loaded default profile.
pub fn setup() -> (ContentBlocker, Rc<FakeHost>, Rc<FakeScripts>) {
    let host = FakeHost::new();
    let profile: Profile = Rc::new(FakeProfile::default());
    host.profiles.borrow_mut().insert(String::new(), profile);
    let scripts = Rc::new(FakeScripts::default());
    (
        ContentBlocker::new(host.clone(), scripts.clone()),
        host,
        scripts,
    )
}

#[test]
fn lists_map_to_categories() {
    assert_eq!(category("x", "custom"), "custom");
    assert_eq!(category("x", "regions"), "regional");
    assert_eq!(category("fanboy-cookiemonster", "annoyances"), "cookies");
    assert_eq!(category("easyprivacy", "privacy"), "trackers");
    assert_eq!(category("ublock-privacy", "default"), "trackers");
    assert_eq!(category("urlhaus-1", "malware"), "security");
    assert_eq!(category("ublock-badware", "default"), "security");
    assert_eq!(category("fanboy-social", "social"), "annoyances");
    assert_eq!(category("easylist", "ads"), "ads");
}

#[test]
fn the_version_comes_from_the_manifest() {
    assert_eq!(
        manifest_version(r#"{"version":"1.2","version_name":"1.2 beta"}"#),
        "1.2 beta"
    );
    assert_eq!(manifest_version(r#"{"version":"1.2"}"#), "1.2");
    assert_eq!(manifest_version(r#"{"version":3}"#), "");
    assert_eq!(manifest_version("nope"), "");
}

#[test]
fn a_state_read_is_shaped_and_published() {
    let read = json!({
        "lists": [{"id": "easylist", "group": "ads", "enabled": true, "filters": 9},
                  {"id": "x", "title": "X", "group": "regions", "enabled": false}, {"title": "no id"}],
        "allowedHosts": ["a.com", 3], "enabled": true, "networkFilters": 5,
    });
    let (state, publish) = shape_state(Some(&read), None, "1.0");
    assert_eq!(state["lists"][0]["title"], "easylist");
    assert_eq!(state["lists"][1]["category"], "regional");
    assert_eq!(state["lists"].as_array().unwrap().len(), 2);
    assert_eq!(state["stats"]["networkFilters"], 5.0);
    assert_eq!(
        publish,
        Some(json!({"enabled": true, "lists": ["easylist"], "allowedHosts": ["a.com"]}))
    );
    let (state, publish) = shape_state(None, Some("boom"), "1.0");
    assert_eq!(state["stats"]["error"], "boom");
    assert_eq!(state["version"], "1.0");
    assert_eq!(publish, None);
}

#[test]
fn the_default_profile_settles_then_state_reads() {
    let (blocker, host, scripts) = setup();
    blocker.load("");
    let got = Rc::new(RefCell::new(None));
    let sink = got.clone();
    blocker.state(Box::new(move |s| *sink.borrow_mut() = Some(s)));
    assert!(
        scripts.tabs.borrow().is_empty(),
        "state waits for the default profile"
    );
    host.fire(Duration::from_millis(1500));
    let tab = scripts.tab(0);
    tab.load();
    tab.answer(r#"{"value":{"lists":[],"enabled":true}}"#);
    let state = got.take().expect("state answers");
    assert_eq!(state["version"], "1.2.3");
    assert_eq!(state["stats"]["ready"], true);
    assert_eq!(host.changes.borrow().len(), 1);
}

#[test]
fn private_profiles_keep_hidden_elements_to_themselves() {
    let (blocker, _, _) = setup();
    let got = Rc::new(RefCell::new(None));
    let sink = got.clone();
    blocker.hide_elements(
        &["a.com##.x".into()],
        "private-1",
        Box::new(move |e| *sink.borrow_mut() = Some(e)),
    );
    assert_eq!(
        got.take(),
        Some(Some(
            "a private window keeps its hidden elements for the session".into()
        ))
    );
}
