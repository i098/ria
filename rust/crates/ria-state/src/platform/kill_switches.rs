// Ported from ardia apps/browser/src/platform/killSwitches.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The kill switches: the table of switches and experiments, the cached values, and the rules that read and refresh them.
//!
//! Changes from Ardia: the fetch of `https://ardia.dev/switches.json`, the timeout and the document read and write are the
//! host's work (they are native calls). [`KillSwitches::load`] takes the text of the cache document, and
//! [`KillSwitches::refresh`] takes the fetched response and returns the text to write to the cache document. The
//! override of a name comes from the launch config, so [`KillSwitches::switch_on`] and [`experiment_on`] take it as an
//! argument. `startKillSwitches` (`killSwitchesLaunch.ts`) waits for the first window and the updater state, which the
//! app runs. The cache document keeps the order of `SWITCHES`, not the order of the response.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// The most switches that the table may have.
pub const MAX_SWITCHES: usize = 9;

const SWITCHES_CACHE_DOCUMENT: &str = "switches-cache.json";
const MAX_BYTES: usize = 4096;

/// One kill switch. A `live` switch takes its value from a fetch in the running app; the others only at the next launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Switch {
    pub name: &'static str,
    pub about: &'static str,
    pub owner: &'static str,
    pub remove_by: &'static str,
    pub live: bool,
}

/// One experiment: off unless the launch config turns it on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Experiment {
    pub name: &'static str,
    pub about: &'static str,
    pub owner: &'static str,
    pub remove_by: &'static str,
}

pub const SWITCHES: [Switch; MAX_SWITCHES] = [
    Switch {
        name: "newTabPrewarm",
        about: "A New Tab page's engine tab is made a moment after it opens, so Enter only navigates it (state/tabs/prewarm.ts).",
        owner: "command bar",
        remove_by: "2026-12-15",
        live: true,
    },
    Switch {
        name: "lazySidebarRows",
        about: "A launch mounts only the sidebar rows its window shows, and the profile pages beside it a frame later (components/sidebar/SidebarPage.tsx).",
        owner: "sidebar",
        remove_by: "2026-12-15",
        live: false,
    },
    Switch {
        name: "updatePrewarm",
        about: "An update staged for install-on-quit is run once, hidden, so its first launch skips macOS's first-exec checks (packages/shell UpdatePrewarm.swift).",
        owner: "updater",
        remove_by: "2026-12-15",
        live: true,
    },
    Switch {
        name: "launchTab",
        about: "The focused window's page starts loading as the app starts, before its window is up (state/tabs/launchTab.ts).",
        owner: "launch",
        remove_by: "2026-12-15",
        live: false,
    },
    Switch {
        name: "lazyProfiles",
        about: "A launch loads only the profiles its windows show; the others, and their extensions, load when first opened (state/profiles/profileLoading.ts).",
        owner: "profiles",
        remove_by: "2027-01-15",
        live: false,
    },
    Switch {
        name: "sidebarSlide",
        about: "Hiding or showing the sidebar or the top tab strip (⌘S) slides it and the card's edge (Arc's motions and toolbar-less card with the address bar in the sidebar); off, they jump (components/window/SidebarDock.tsx).",
        owner: "sidebar",
        remove_by: "2026-12-15",
        live: true,
    },
    Switch {
        name: "sidebarSlideLean",
        about: "The sidebar's slide draws the traffic lights' moves and lays them out once they stop (packages/shell ChromeWindows.swift).",
        owner: "sidebar",
        remove_by: "2026-12-15",
        live: false,
    },
    Switch {
        name: "serverWindowDrag",
        about: "The window server moves the window from our drag areas in the title-bar band, so those drags tile, snap and cross displays as macOS's own, and our pointer-following loop holds AppKit's re-fits of the frames it places; off, every drag is the loop with AppKit's constrain, as before (packages/shell ServerWindowDrag.swift, WindowMove.swift).",
        owner: "window",
        remove_by: "2026-12-15",
        live: false,
    },
    Switch {
        name: "peekGenie",
        about: "A Peek opens and closes with the Genie warp, a private Core Animation mesh; off, it zooms from the link and the mesh runtime is never inspected (packages/shell PeekStage.swift, ArdiaPeekMesh.m).",
        owner: "peek",
        remove_by: "2027-02-15",
        live: true,
    },
];

pub const EXPERIMENTS: [Experiment; 4] = [
    Experiment {
        name: "freezeWatchOff",
        about: "Turns off the freeze watch: the run-loop observers on the main and JavaScript threads that count stalls for perf_memory (packages/ardiacore ArdiaFreezeWatch.mm, state/telemetry/freezes.ts).",
        owner: "perf",
        remove_by: "2027-02-28",
    },
    Experiment {
        name: "serverWindowDragAllOpen",
        about: "With serverWindowDrag, every view in the titlebar band is also left open to window moves (a probe for the geometry work).",
        owner: "window",
        remove_by: "2026-12-15",
    },
    Experiment {
        name: "dragRegionEveryMount",
        about: "With serverWindowDrag, every React Native mount rebuilds each window's title-bar drag region, as it used to, instead of only the window whose title bar the pointer is in (packages/shell ServerWindowDrag.swift).",
        owner: "window",
        remove_by: "2026-12-31",
    },
    Experiment {
        name: "officialPluginsAi",
        about: "Lists the AI plugin as coming soon among the official plugins (state/plugins/official.ts), for Settings and onboarding.",
        owner: "plugins",
        remove_by: "2027-03-31",
    },
];

/// The name of the document that keeps the last fetched values.
pub const CACHE_DOCUMENT: &str = SWITCHES_CACHE_DOCUMENT;

/// The URL that the host fetches.
pub const SWITCHES_URL: &str = "https://ardia.dev/switches.json";

/// The fetch timeout of the host, in milliseconds.
pub const FETCH_TIMEOUT_MS: u64 = 5000;

type Values = BTreeMap<&'static str, bool>;

fn known(name: &str) -> Option<&'static str> {
    SWITCHES.iter().find(|s| s.name == name).map(|s| s.name)
}

/// The known boolean switches in `json`, or none for text that is not the expected JSON.
pub fn parse_switches(json: Option<&str>) -> BTreeMap<&'static str, bool> {
    let mut out = Values::new();
    let Some(Value::Object(root)) = json.and_then(|text| serde_json::from_str(text).ok()) else {
        return out;
    };
    if let Some(Value::Object(switches)) = root.get("switches") {
        for (name, value) in switches {
            if let (Some(name), Value::Bool(on)) = (known(name), value) {
                out.insert(name, *on);
            }
        }
    }
    out
}

/// The result of [`KillSwitches::refresh`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshResult {
    Updated,
    Failed,
    Already,
}

/// The fetched response: the status was `ok`, and the body text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub ok: bool,
    pub text: String,
}

/// The cached switch values of one launch.
#[derive(Debug, Default)]
pub struct KillSwitches {
    cached: Values,
    loaded: bool,
    fetched_this_launch: bool,
}

impl KillSwitches {
    /// Reads the cache from the text of the cache document (`None` when it is missing or unreadable).
    pub fn load(&mut self, document: Option<&str>) {
        self.loaded = true;
        self.cached = parse_switches(document);
    }

    /// Whether switch `name` is on: the launch config override, then the cache, then on. `document` is read once, when
    /// nothing is loaded yet.
    pub fn switch_on(
        &mut self,
        name: &str,
        over: Option<bool>,
        document: impl FnOnce() -> Option<String>,
    ) -> bool {
        if !self.loaded {
            self.load(document().as_deref());
        }
        over.or_else(|| self.cached.get(name).copied())
            .unwrap_or(true)
    }

    /// Forgets the cache, so the next [`Self::switch_on`] loads it again.
    pub fn reload(&mut self) {
        self.loaded = false;
        self.cached.clear();
    }

    /// Takes a fetch (`None` when the fetch failed or timed out). Returns the result and, on an update, the text for the
    /// cache document. A launch takes one fetch; `document` is read when nothing is loaded yet.
    pub fn refresh(
        &mut self,
        response: Option<Fetched>,
        document: impl FnOnce() -> Option<String>,
    ) -> (RefreshResult, Option<String>) {
        if self.fetched_this_launch {
            return (RefreshResult::Already, None);
        }
        self.fetched_this_launch = true;
        if !self.loaded {
            self.load(document().as_deref());
        }
        let Some(Fetched { ok: true, text }) = response else {
            return (RefreshResult::Failed, None);
        };
        // `String.length` of the source counts UTF-16 units.
        if text.encode_utf16().count() > MAX_BYTES {
            return (RefreshResult::Failed, None);
        }
        let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
            return (RefreshResult::Failed, None);
        };
        let version_one = parsed.get("version").and_then(Value::as_f64) == Some(1.0);
        if !version_one
            || !matches!(
                parsed.get("switches"),
                Some(Value::Object(_) | Value::Array(_))
            )
        {
            return (RefreshResult::Failed, None);
        }
        let values = parse_switches(Some(&text));
        let document = json!({ "version": 1, "switches": values }).to_string();
        for switch in SWITCHES.iter().filter(|s| s.live) {
            match values.get(switch.name) {
                Some(on) => self.cached.insert(switch.name, *on),
                None => self.cached.remove(switch.name),
            };
        }
        (RefreshResult::Updated, Some(document))
    }

    /// Lets the next launch fetch again; the TypeScript tests need this (`resetRefreshForTests`).
    pub fn reset_refresh(&mut self) {
        self.fetched_this_launch = false;
    }
}

/// Whether experiment `name` is on: only the launch config override turns it on.
pub fn experiment_on(over: Option<bool>) -> bool {
    over.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetched(text: &str) -> Option<Fetched> {
        Some(Fetched {
            ok: true,
            text: text.into(),
        })
    }

    #[test]
    fn table_matches_the_limit_and_names_are_unique() {
        assert!(SWITCHES.len() <= MAX_SWITCHES);
        for (i, s) in SWITCHES.iter().enumerate() {
            assert!(SWITCHES[i + 1..].iter().all(|t| t.name != s.name));
        }
    }

    #[test]
    fn parse_keeps_only_known_booleans() {
        let got = parse_switches(Some(
            r#"{"switches":{"launchTab":false,"peekGenie":"no","nope":true,"lazyProfiles":true}}"#,
        ));
        assert_eq!(
            got,
            BTreeMap::from([("launchTab", false), ("lazyProfiles", true)])
        );
        assert!(parse_switches(Some("not json")).is_empty());
        assert!(parse_switches(None).is_empty());
    }

    #[test]
    fn override_beats_cache_and_default_is_on() {
        let mut k = KillSwitches::default();
        let doc = || Some(r#"{"switches":{"launchTab":false}}"#.to_string());
        assert!(!k.switch_on("launchTab", None, doc));
        assert!(k.switch_on("launchTab", Some(true), || None));
        assert!(k.switch_on("peekGenie", None, || None));
    }

    #[test]
    fn refresh_updates_live_switches_only_and_once() {
        let mut k = KillSwitches::default();
        k.load(Some(
            r#"{"switches":{"launchTab":false,"newTabPrewarm":false}}"#,
        ));
        let body = r#"{"version":1,"switches":{"launchTab":true,"newTabPrewarm":true}}"#;
        let (result, document) = k.refresh(fetched(body), || None);
        assert_eq!(result, RefreshResult::Updated);
        assert_eq!(
            document.unwrap(),
            r#"{"switches":{"launchTab":true,"newTabPrewarm":true},"version":1}"#
        );
        assert!(
            !k.switch_on("launchTab", None, || None),
            "launchTab is not live: it keeps the launch value"
        );
        assert!(k.switch_on("newTabPrewarm", None, || None));
        assert_eq!(
            k.refresh(fetched(body), || None),
            (RefreshResult::Already, None)
        );
    }

    #[test]
    fn refresh_refuses_bad_responses() {
        for response in [
            None,
            Some(Fetched {
                ok: false,
                text: "{}".into(),
            }),
            fetched(r#"{"version":2,"switches":{}}"#),
            fetched(r#"{"version":1}"#),
            fetched("null"),
            fetched(&format!(
                r#"{{"version":1,"switches":{{}},"pad":"{}"}}"#,
                "é".repeat(MAX_BYTES)
            )),
        ] {
            let mut k = KillSwitches::default();
            assert_eq!(k.refresh(response, || None), (RefreshResult::Failed, None));
        }
    }
}
