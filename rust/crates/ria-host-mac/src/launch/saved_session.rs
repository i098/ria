// Ported from ardia packages/ardiacore/ios/ArdiaSavedSession.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::path::Path;

use ria_engine::domains::data::{
    origins,
    profiles::{is_private_session_id, profile_directory_name},
};
use serde_json::{Map, Value};

use super::saved_document::{self, SavedDocument};

pub const VERSION: i32 = 11;

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
    pub fn read(config: &super::launch_config::LaunchConfig) -> Option<Self> {
        saved_document::read(config, Path::new("session.json"), 0..=VERSION)
            .map(Self::from_document)
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
        if hint.id.is_empty()
            || !origins::of_url(&hint.url)
                .is_some_and(|origin| origin.starts_with("http://") || origin.starts_with("https://"))
        {
            return None;
        }
        let name = hint.profile.as_str();
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
        if Path::new(&directory).file_name().and_then(|name| name.to_str()) != Some(directory.as_str())
            || (!name.is_empty()
                && !super::profile::directory_exists(&root.join("Chromium").join(&directory))
                    .unwrap_or(false))
        {
            return None;
        }
        Some(directory)
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
