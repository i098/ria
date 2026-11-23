// Ported from ardia packages/ardiacore/ios/ArdiaContentBlockerLite.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Moves the settings of uBlock Origin Lite (the content blocker of older builds) into uBlock Origin once for each
//! profile: a small bundled reader extension reads the Lite storage, then a script applies it to uBlock Origin.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use serde_json::{Value, json};

use super::content_blocker::{ContentBlocker, LITE_ID, LITE_MOVED_MARKER, LITE_READER_NAME};
use super::content_blocker_calls::script_with;
use super::host::Profile;

/// `LiteListMap`: Lite ruleset ids that do not match a uBlock Origin list id, and the lists that stand for them.
fn lite_list_map() -> Value {
    json!({
        "ublock-filters": ["ublock-filters", "ublock-unbreak", "ublock-quick-fixes"],
        "pgl": ["plowe-0"],
        "urlhaus-full": ["urlhaus-1"],
        "annoyances-ai": ["fanboy-ai-suggestions"],
        "annoyances-cookies": ["fanboy-cookiemonster", "ublock-cookies-easylist"],
        "annoyances-overlays": ["easylist-newsletters", "ublock-annoyances"],
        "annoyances-social": ["fanboy-social"],
        "annoyances-widgets": ["easylist-chat"],
        "annoyances-others": ["easylist-annoyances"],
        "annoyances-notifications": ["easylist-notifications"],
    })
}

const MOVE_LITE: &str = concat!(
    "Promise.all([lists(), trusted()]).then(([l, t]) => {",
    "  const lite = %@;",
    "  const map = %@;",
    "  const none = Array.isArray(lite.filteringModeDetails?.none) ? lite.filteringModeDetails.none : [];",
    "  const off = none.includes('all-urls');",
    "  const hosts = none.filter((h) => typeof h === 'string' && h !== 'all-urls' && /^[a-z0-9.\\-_[\\]:]+$/.test(h));",
    "  const enabled = Array.isArray(lite.rulesetConfig?.enabledRulesets) ? lite.rulesetConfig.enabledRulesets : null;",
    "  const defaults = Array.isArray(lite.defaultRulesetIds) ? lite.defaultRulesetIds : [];",
    "  const keys = Object.keys(l.available);",
    "  const lookup = (id) => (map[id] || keys.filter((k) => k.toLowerCase() === String(id).toLowerCase())).filter((k) => l.available[k]);",
    "  const pick = new Set(selected(l));",
    "  if (enabled) {",
    "    for (const id of defaults) if (!enabled.includes(id)) for (const k of lookup(id)) pick.delete(k);",
    "    for (const id of enabled) if (!defaults.includes(id)) for (const k of lookup(id)) pick.add(k);",
    "  }",
    "  const whitelist = [...new Set(t.whitelist.filter((d) => d !== offDirective).concat(hosts, off ? [offDirective] : []))];",
    "  let written = Promise.resolve(null);",
    "  if (!sameSet(pick, selected(l)))",
    "    written = written.then(() => applyLists([...pick]));",
    "  if (!sameSet(whitelist, t.whitelist))",
    "    written = written.then(() => send({what: 'setWhitelist', whitelist: whitelist.join('\\n')}));",
    "  if (off) written = written.then(() => send({what: 'userSettings', name: 'autoUpdate', value: false}));",
    "  return written.then(() => ({off, hosts: hosts.length, lists: pick.size}));",
    "})",
);

const READ_LITE: &str = concat!(
    "chrome.storage.local.get(['filteringModeDetails', 'rulesetConfig', 'defaultRulesetIds'])",
    ".then((v) => post('result', JSON.stringify({value: v})), (e) => post('result', JSON.stringify({error: String(e && e.message || e)})))",
);

const LITE_READ_TIME: Duration = Duration::from_secs(10);

fn lite_marker(p: &Profile) -> Option<PathBuf> {
    let path = p.path();
    (!path.is_empty()).then(|| PathBuf::from(path).join(LITE_MOVED_MARKER))
}

/// `LiteSettingsPending`: the profile has Lite storage and no marker that it moved.
pub(super) fn lite_settings_pending(p: &Profile) -> bool {
    let Some(marker) = lite_marker(p) else {
        return false;
    };
    if marker.exists() {
        return false;
    }
    PathBuf::from(p.path())
        .join("Local Extension Settings")
        .join(LITE_ID)
        .exists()
}

fn mark_lite_settings_moved(p: &Profile, summary: &str) {
    eprintln!("[blocker] uBlock Origin Lite's settings: {summary}");
    if let Some(marker) = lite_marker(p) {
        let _ = std::fs::write(marker, summary);
    }
}

/// `LiteSettingsUsable`: the read settings have a shape that the move script knows.
pub(super) fn lite_settings_usable(lite: &Value) -> bool {
    let shaped = |key: &str, inner: &str| {
        lite.get(key)
            .and_then(Value::as_object)
            .is_some_and(|o| o.get(inner).is_none_or(Value::is_array))
    };
    shaped("filteringModeDetails", "none") || shaped("rulesetConfig", "enabledRulesets")
}

impl ContentBlocker {
    fn lite_reader_path(&self) -> Option<PathBuf> {
        let path = self
            .0
            .host
            .resource_path()?
            .join("Extensions")
            .join(LITE_READER_NAME);
        path.join("manifest.json").exists().then_some(path)
    }

    fn read_lite_settings(
        &self,
        profile: &str,
        p: &Profile,
        done: Box<dyn FnOnce(Option<Value>, Option<String>)>,
    ) {
        let Some(reader) = self.lite_reader_path() else {
            return done(None, Some("the reader isn't in the app".into()));
        };
        if p.load_component_extension(&reader.to_string_lossy()) != LITE_ID {
            return done(None, Some("the reader didn't load".into()));
        }
        let key = format!("lite:{profile}");
        let page = self.hidden_page(&key, profile, p, LITE_ID, "read.html");
        let answered = Rc::new(Cell::new(false));
        let done = Rc::new(Cell::new(Some(done)));
        let finish = {
            let (blocker, page, p) = (self.clone(), page.clone(), p.clone());
            Rc::new(move |lite: Option<Value>, error: Option<String>| {
                if answered.replace(true) {
                    return;
                }
                if let Some(page) = &page {
                    page.fail("read");
                }
                blocker.0.state.borrow_mut().pages.remove(&key);
                blocker.0.host.unload_component_extension(&p, LITE_ID);
                if let Some(done) = done.take() {
                    done(lite, error);
                }
            })
        };
        let Some(page) = page else {
            return finish(None, Some("no reader page".into()));
        };
        let timeout = finish.clone();
        self.0.host.after(
            LITE_READ_TIME,
            Box::new(move || timeout(None, Some("the reader timed out".into()))),
        );
        page.run(
            READ_LITE.to_owned(),
            Box::new(move |lite, error| {
                let lite = lite.filter(Value::is_object);
                let error = error.or_else(|| lite.is_none().then(|| "unreadable".to_owned()));
                finish(lite, error);
            }),
        );
    }

    /// `MoveLiteSettings`: moves the Lite settings of `p` once, then runs `done`. Failures wait for the next launch.
    pub(super) fn move_lite_settings(&self, profile: &str, p: &Profile, done: Box<dyn FnOnce()>) {
        if !lite_settings_pending(p) {
            return done();
        }
        let (blocker, owned, p2) = (self.clone(), profile.to_owned(), p.clone());
        self.read_lite_settings(
            profile,
            p,
            Box::new(move |lite, error| {
                let lite = match (lite, error) {
                    (_, Some(error)) => {
                        eprintln!("[blocker] uBlock Origin Lite's settings weren't read ({error}); trying again next launch");
                        return done();
                    }
                    (Some(lite), None) if lite_settings_usable(&lite) => lite,
                    (lite, None) => {
                        let keys: Vec<String> = lite
                            .as_ref()
                            .and_then(Value::as_object)
                            .map(|o| o.keys().cloned().collect())
                            .unwrap_or_default();
                        eprintln!(
                            "[blocker] uBlock Origin Lite's settings aren't in a shape we know ({keys:?}); trying again next launch"
                        );
                        return done();
                    }
                };
                blocker.call(
                    &owned,
                    script_with(MOVE_LITE, &[lite, lite_list_map()]),
                    Box::new(move |result, error| {
                        if let Some(error) = error {
                            eprintln!("[blocker] uBlock Origin Lite's settings didn't move ({error}); trying again next launch");
                            return done();
                        }
                        mark_lite_settings_moved(&p2, &format!("moved ({})", result.unwrap_or(Value::Null)));
                        done();
                    }),
                    true,
                );
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_lite_shapes_are_usable() {
        assert!(lite_settings_usable(
            &json!({"filteringModeDetails": {"none": ["a.com"]}})
        ));
        assert!(lite_settings_usable(&json!({"filteringModeDetails": {}})));
        assert!(lite_settings_usable(
            &json!({"rulesetConfig": {"enabledRulesets": []}})
        ));
        assert!(!lite_settings_usable(
            &json!({"rulesetConfig": {"enabledRulesets": "x"}})
        ));
        assert!(!lite_settings_usable(
            &json!({"filteringModeDetails": {"none": 1}})
        ));
        assert!(!lite_settings_usable(&json!({})));
    }
}
