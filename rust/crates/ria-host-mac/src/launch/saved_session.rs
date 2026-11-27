// Ported from ardia packages/ardiacore/ios/ArdiaSavedSession.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::path::Path;

use ria_engine::domains::data::{
    origins,
    profiles::{is_private_session_id, profile_directory_name},
};
use serde_json::{Map, Value, json};

use super::launch_config::LaunchConfig;
use super::saved_document::{self, SavedDocument};

pub const VERSION: i32 = 11;
const SESSION_FILE: &str = "session.json";
const STRIP_WINDOW_ID: &str = "ria-early-window";
pub(super) const BLANK_URL: &str = "about:blank";

#[derive(Clone, Copy)]
pub(super) struct StripTab<'a> {
    pub key: &'a str,
    pub url: &'a str,
    pub pinned: bool,
    pub active: bool,
}

pub struct LaunchTab {
    pub id: String,
    pub url: String,
    pub profile: String,
}

/// The saved fields that the native host needs before the app restores its state.
pub struct SavedSession {
    pub file_version: i32,
    pub focused_window_id: Option<String>,
    pub link_profile: Option<String>,
    pub launch_tab: Option<LaunchTab>,
    pub launch_backdrop: Option<Map<String, Value>>,
    pub settings: Map<String, Value>,
    pub profiles: Option<Value>,
    pub windows: Vec<Map<String, Value>>,
    pub tabs: Vec<Map<String, Value>>,
    pub groups: Vec<Map<String, Value>>,
    pub splits: Vec<Map<String, Value>>,
    orphaned_profile_data: Option<Value>,
}

impl SavedSession {
    pub fn read(config: &LaunchConfig) -> Option<Self> {
        saved_document::read(config, Path::new(SESSION_FILE), 0..=VERSION).map(Self::from_document)
    }

    #[cfg(test)]
    fn parse(data: &[u8]) -> Option<Self> {
        saved_document::parse(data, 0..=VERSION).map(Self::from_document)
    }

    fn from_document(
        SavedDocument {
            version,
            mut fields,
        }: SavedDocument,
    ) -> Self {
        let launch_tab = fields
            .remove("launchTab")
            .and_then(object)
            .and_then(|mut hint| {
                Some(LaunchTab {
                    id: take_string(&mut hint, "id")?,
                    url: take_string(&mut hint, "url")?,
                    profile: take_string(&mut hint, "profile")?,
                })
            });
        Self {
            file_version: version,
            focused_window_id: take_string(&mut fields, "focusedWindowId"),
            link_profile: take_string(&mut fields, "linkProfile"),
            launch_tab,
            launch_backdrop: fields.remove("launchBackdrop").and_then(object),
            settings: fields
                .remove("settings")
                .and_then(object)
                .unwrap_or_default(),
            profiles: fields.remove("profiles"),
            windows: take_objects(&mut fields, "windows"),
            tabs: take_objects(&mut fields, "tabs"),
            groups: take_objects(&mut fields, "groups"),
            splits: take_objects(&mut fields, "splits"),
            orphaned_profile_data: fields.remove("orphanedProfileData"),
        }
    }

    pub(super) fn launch_profile_directory(&self, root: &Path) -> Option<String> {
        let hint = self.launch_tab.as_ref()?;
        if hint.id.is_empty() || !is_web_url(&hint.url) {
            return None;
        }
        self.profile_directory(root, &hint.profile)
    }

    pub(super) fn focused_profile_directory(&self, root: &Path) -> Option<String> {
        let window = self.focused_regular_window()?;
        self.profile_directory(root, window.get("profileId")?.as_str()?)
    }

    fn profile_directory(&self, root: &Path, name: &str) -> Option<String> {
        if name == "Default"
            || name.starts_with("Profile ")
            || is_private_session_id(name)
            || self
                .orphaned_profile_data
                .as_ref()
                .and_then(Value::as_array)
                .is_some_and(|names| names.iter().any(|entry| entry.as_str() == Some(name)))
        {
            return None;
        }
        let directory = profile_directory_name(name);
        if Path::new(&directory)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(directory.as_str())
            || (!name.is_empty()
                && !super::profile::directory_exists(&root.join("Chromium").join(&directory))
                    .unwrap_or(false))
        {
            return None;
        }
        Some(directory)
    }

    fn focused_regular_window(&self) -> Option<&Map<String, Value>> {
        let window = self.window_with_id(self.focused_window_id.as_deref()?)?;
        (!window.contains_key("kind") && window.get("incognito") != Some(&Value::Bool(true)))
            .then_some(window)
    }

    pub(super) fn restore_strip(&self, profile: &str) -> Vec<StripTab<'_>> {
        let Some(window) = self
            .focused_regular_window()
            .filter(|window| window.get("profileId").and_then(Value::as_str) == Some(profile))
        else {
            return Vec::new();
        };
        let active = window
            .get("activeTabIds")
            .and_then(|ids| ids.get(profile))
            .and_then(Value::as_str)
            .or_else(|| self.launch_tab.as_ref().map(|tab| tab.id.as_str()));
        let mut by_id: HashMap<_, _> = self
            .tabs
            .iter()
            .filter_map(|tab| {
                if tab.get("windowId") != window.get("id")
                    || tab.get("profileId").and_then(Value::as_str) != Some(profile)
                {
                    return None;
                }
                Some((tab.get("id")?.as_str()?, tab))
            })
            .collect();
        let mut tabs: Vec<_> = array(window.get("tabIds"))
            .iter()
            .filter_map(|id| {
                let key = id.as_str()?;
                let tab = by_id.remove(key)?;
                let pin = self.pin_of_tab(tab);
                Some(StripTab {
                    key,
                    url: restorable_url(tab.get("url")?.as_str()?),
                    pinned: if self.file_version < 6 {
                        pin.and_then(Value::as_bool) == Some(true)
                    } else {
                        matches!(
                            pin.and_then(|pin| pin.get("as")).and_then(Value::as_str),
                            Some("row" | "tile")
                        )
                    },
                    active: active == Some(key),
                })
            })
            .collect();
        tabs.sort_by_key(|tab| !tab.pinned);
        tabs
    }

    pub fn pin_of_tab<'a>(&self, tab: &'a Map<String, Value>) -> Option<&'a Value> {
        tab.get(if self.file_version < 6 {
            "pinned"
        } else {
            "pin"
        })
    }

    pub fn tab_ids_of_split<'a>(
        &self,
        split: &'a Map<String, Value>,
    ) -> impl Iterator<Item = &'a str> + use<'a> {
        let (ids, slots) = if self.file_version < 7 {
            (split.get("tabIds"), None)
        } else {
            (None, split.get("slots"))
        };
        array(ids).iter().filter_map(Value::as_str).chain(
            array(slots)
                .iter()
                .filter_map(Value::as_object)
                .flat_map(|slot| {
                    slot.get("tabId")
                        .and_then(Value::as_str)
                        .into_iter()
                        .chain(array(slot.get("stack")).iter().filter_map(Value::as_str))
                }),
        )
    }

    pub fn window_with_id(&self, id: &str) -> Option<&Map<String, Value>> {
        self.windows
            .iter()
            .find(|window| window.get("id").and_then(Value::as_str) == Some(id))
    }
}

fn is_web_url(url: &str) -> bool {
    origins::of_url(url)
        .is_some_and(|origin| origin.starts_with("http://") || origin.starts_with("https://"))
}

fn restorable_url(url: &str) -> &str {
    // Do not run browser commands or open external apps from a saved document.
    if !url.chars().any(char::is_control)
        && (url.starts_with("file://") || url.starts_with("data:") || is_web_url(url))
    {
        url
    } else {
        BLANK_URL
    }
}

pub(super) fn write_strip<'a>(
    config: &LaunchConfig,
    profile: &str,
    tabs: impl IntoIterator<Item = StripTab<'a>>,
) -> io::Result<()> {
    let path = Path::new(SESSION_FILE);
    let SavedDocument {
        version,
        mut fields,
    } = match config.read_admitted_file(path) {
        Ok(bytes) => saved_document::parse(&bytes, 0..=VERSION).ok_or(ErrorKind::InvalidData)?,
        Err(error) if error.kind() == ErrorKind::NotFound => SavedDocument {
            version: VERSION,
            fields: Map::new(),
        },
        Err(error) => return Err(error),
    };
    update_strip(&mut fields, version, profile, tabs);
    config.replace_data_file(path, &serde_json::to_vec(&fields)?)
}

fn update_strip<'a>(
    fields: &mut Map<String, Value>,
    version: i32,
    profile: &str,
    tabs: impl IntoIterator<Item = StripTab<'a>>,
) {
    let mut tab_ids = Vec::new();
    let mut tab_rows = Vec::new();
    let mut launch = Value::Null;
    let mut active = None;
    for tab in tabs {
        let id = format!("ria-{}", tab.key);
        let url = restorable_url(tab.url);
        if tab.active {
            active = Some(id.clone());
            if is_web_url(url) {
                launch = json!({"id": id, "url": url, "profile": profile});
            }
        }
        let mut row = json!({
            "id": id, "windowId": STRIP_WINDOW_ID, "profileId": profile,
            "url": url, "page": {"kind": "idle"}
        });
        if version < 6 {
            row["pinned"] = tab.pinned.into();
            row["pinnedAsRow"] = tab.pinned.into();
        } else {
            row["pin"] = if tab.pinned {
                json!({"as": "row"})
            } else {
                Value::Null
            };
        }
        tab_ids.push(id);
        tab_rows.push(row);
    }
    let Some(active) = active.or_else(|| tab_ids.first().cloned()) else {
        return;
    };
    let window = json!({
        "id": STRIP_WINDOW_ID, "profileId": profile, "tabIds": tab_ids,
        "activeTabIds": { (profile): active },
    });
    let replace_rows = |field: Option<Value>, owner: &str, rows: Vec<Value>| {
        let mut existing = match field {
            Some(Value::Array(rows)) => rows,
            _ => Vec::new(),
        };
        existing.retain(|row| row.get(owner).and_then(Value::as_str) != Some(STRIP_WINDOW_ID));
        existing.extend(rows);
        Value::Array(existing)
    };
    let windows = replace_rows(fields.remove("windows"), "id", vec![window]);
    let tabs = replace_rows(fields.remove("tabs"), "windowId", tab_rows);
    fields.insert("windows".into(), windows);
    fields.insert("tabs".into(), tabs);
    let mut order = fields
        .remove("windowOrder")
        .and_then(|value| match value {
            Value::Array(order) => Some(order),
            _ => None,
        })
        .unwrap_or_default();
    if !order.iter().any(|id| id.as_str() == Some(STRIP_WINDOW_ID)) {
        order.push(STRIP_WINDOW_ID.into());
    }
    fields.insert("windowOrder".into(), order.into());
    fields.insert("version".into(), version.into());
    fields.insert("focusedWindowId".into(), STRIP_WINDOW_ID.into());
    fields.insert("launchTab".into(), launch);
}

fn array(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn object(value: Value) -> Option<Map<String, Value>> {
    match value {
        Value::Object(fields) => Some(fields),
        _ => None,
    }
}

fn take_string(fields: &mut Map<String, Value>, key: &str) -> Option<String> {
    match fields.remove(key)? {
        Value::String(value) => Some(value),
        _ => None,
    }
}

fn take_objects(fields: &mut Map<String, Value>, key: &str) -> Vec<Map<String, Value>> {
    match fields.remove(key) {
        Some(Value::Array(items)) => items.into_iter().filter_map(object).collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::SavedSession;
    use serde_json::{Value, json};

    fn session(doc: Value) -> SavedSession {
        SavedSession::parse(&serde_json::to_vec(&doc).unwrap()).unwrap()
    }

    // The pin and split fixtures come from apps/browser/src/state/session/fixtures/savedSessionProbe.mm.
    #[test]
    fn reads_pins_by_file_version() {
        let old = json!({"id":"a","pinned":true,"pinnedAsRow":true});
        for version in [4, 5] {
            let saved = session(json!({"version":version,"tabs":[old]}));
            assert_eq!(saved.file_version, version);
            assert_eq!(saved.pin_of_tab(&saved.tabs[0]), Some(&json!(true)));
        }
        let missing = session(json!({"tabs":[old]}));
        assert_eq!(missing.file_version, 0);
        assert_eq!(missing.pin_of_tab(&missing.tabs[0]), Some(&json!(true)));
        let six = session(
            json!({"version":6,"tabs":[{"id":"a","pin":{"as":"row"}},{"id":"b","pin":null}]}),
        );
        assert_eq!(six.file_version, 6);
        assert_eq!(six.pin_of_tab(&six.tabs[0]), Some(&json!({"as":"row"})));
        assert_eq!(six.pin_of_tab(&six.tabs[1]), Some(&Value::Null));
        assert_eq!(six.pin_of_tab(old.as_object().unwrap()), None);
    }

    #[test]
    fn reads_split_slots_in_order() {
        let old = json!({"id":"s","tabIds":["a","b","c"],"sizes":[0.5,0.5],"stack":{"index":1}});
        let new = json!({"id":"s","slots":[{"size":0.5,"tabId":"a"},{"size":0.5,"stack":["b","c"],"stackSizes":[0.5,0.5]}]});
        for version in [5, 6] {
            let saved = session(json!({"version":version,"splits":[old]}));
            assert!(saved.tab_ids_of_split(&saved.splits[0]).eq(["a", "b", "c"]));
        }
        let seven = session(json!({"version":7,"splits":[new,old]}));
        assert!(seven.tab_ids_of_split(&seven.splits[0]).eq(["a", "b", "c"]));
        assert!(seven.tab_ids_of_split(&seven.splits[1]).next().is_none());
        assert!(SavedSession::parse(br#"{"version":12,"tabs":[]}"#).is_none());
    }

    #[test]
    fn saves_the_current_strip_without_removing_other_session_data() {
        let home = std::env::temp_dir().join(format!("ria-session-write-{}", std::process::id()));
        let config =
            super::super::launch_config::LaunchConfig::from_environment([], &home, &home, false)
                .unwrap();
        let path = config.data_directory().join(super::SESSION_FILE);
        let original = json!({
            "version": 11,
            "settings": {"keep": true},
            "windows": [{"id": "other"}],
            "tabs": [{"id": "other-tab", "windowId": "other"}]
        });
        std::fs::write(&path, original.to_string()).unwrap();
        let tabs = [
            super::StripTab {
                key: "b",
                url: "https://example.com/b",
                pinned: true,
                active: false,
            },
            super::StripTab {
                key: "a",
                url: "https://example.com/a",
                pinned: false,
                active: false,
            },
            super::StripTab {
                key: "blank",
                url: "about:blank",
                pinned: false,
                active: true,
            },
        ];
        super::write_strip(&config, "", tabs).unwrap();
        let saved = SavedSession::read(&config).unwrap();
        assert!(saved.launch_tab.is_none());
        let restored: Vec<_> = saved
            .restore_strip("")
            .into_iter()
            .map(|tab| (tab.url, tab.pinned, tab.active))
            .collect();
        assert_eq!(
            restored,
            [
                ("https://example.com/b", true, false),
                ("https://example.com/a", false, false),
                ("about:blank", false, true),
            ]
        );
        super::write_strip(
            &config,
            "",
            [super::StripTab {
                active: true,
                ..tabs[1]
            }],
        )
        .unwrap();
        let saved = SavedSession::read(&config).unwrap();
        assert_eq!(saved.restore_strip("")[0].url, "https://example.com/a");
        assert_eq!(saved.restore_strip("").len(), 1);
        assert_eq!(
            saved.launch_tab.as_ref().unwrap().url,
            "https://example.com/a"
        );
        assert_eq!(saved.settings["keep"], true);
        assert!(saved.window_with_id("other").is_some());
        assert!(saved.tabs.iter().any(|tab| tab["id"] == "other-tab"));
        std::fs::write(&path, br#"{"version":12}"#).unwrap();
        assert!(super::write_strip(&config, "", tabs).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), br#"{"version":12}"#);
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn restores_only_unique_tabs_of_the_loaded_regular_profile() {
        let saved = session(json!({
            "version": 5,
            "focusedWindowId": "w",
            "windows": [{"id": "w", "profileId": "", "tabIds": ["a", "b", "a", "private"]}],
            "tabs": [
                {"id": "b", "windowId": "w", "profileId": "", "url": "chrome://quit"},
                {"id": "a", "windowId": "w", "profileId": "", "url": "https://example.com/", "pinned": true},
                {"id": "private", "windowId": "w", "profileId": "incognito1@", "url": "https://example.com/private"}
            ]
        }));
        let restored: Vec<_> = saved
            .restore_strip("")
            .into_iter()
            .map(|tab| (tab.key, tab.url, tab.pinned))
            .collect();
        assert_eq!(
            restored,
            [
                ("a", "https://example.com/", true),
                ("b", "about:blank", false)
            ]
        );
        assert!(saved.restore_strip("incognito1@").is_empty());
    }

    #[test]
    fn reads_a_real_session_file() {
        let home = std::env::temp_dir().join(format!("ria-session-test-{}", std::process::id()));
        let config =
            super::super::launch_config::LaunchConfig::from_environment([], &home, &home, false)
                .unwrap();
        let root = config.data_directory();
        let document = br#"{"version":11,"focusedWindowId":"w","windows":[null,{"id":"w","profile":"p"}],"launchTab":{"id":"t","url":"https://example.com","profile":"p"}}"#;
        std::fs::write(root.join("session.json"), document).unwrap();
        let saved = SavedSession::read(&config);
        let outside = root
            .join("..")
            .join(format!("ria-outside-session-{}", std::process::id()));
        std::fs::rename(root.join("session.json"), &outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("session.json")).unwrap();
        let linked = SavedSession::read(&config);
        std::fs::remove_file(outside).unwrap();
        std::fs::remove_dir_all(&home).unwrap();
        assert!(linked.is_none());
        let saved = saved.unwrap();
        assert_eq!(saved.focused_window_id.as_deref(), Some("w"));
        assert_eq!(saved.window_with_id("w").unwrap()["profile"], "p");
        assert_eq!(saved.windows.len(), 1);
        assert_eq!(saved.launch_tab.unwrap().id, "t");
    }

    #[test]
    fn selects_only_live_regular_profiles_for_web_hints() {
        let root = std::env::temp_dir().join(format!("ria-launch-hint-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Chromium/Profile work")).unwrap();
        std::fs::create_dir(root.join("Chromium/Profile incognito1@work")).unwrap();
        for (id, url, profile, orphaned, expected) in [
            ("a", "HTTPS://example.org", "", "", Some("Default")),
            ("a", "https://example.org", "work", "", Some("Profile work")),
            ("a", "https://example.org", "Default", "", None),
            ("a", "https://example.org", "Profile work", "", None),
            ("a", "https://example.org", "work", "work", None),
            ("a", "https://example.org", "missing", "", None),
            ("a", "https://example.org", "incognito1@work", "", None),
            ("a", "https://example.org", "../work", "", None),
            ("a", "file:///tmp/private", "", "", None),
            ("a", "https://", "", "", None),
            ("", "https://example.org", "", "", None),
        ] {
            let session = session(json!({
                "version": 11,
                "launchTab": {"id": id, "url": url, "profile": profile},
                "orphanedProfileData": if orphaned.is_empty() { vec![] } else { vec![orphaned] },
            }));
            assert_eq!(session.launch_profile_directory(&root).as_deref(), expected);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
