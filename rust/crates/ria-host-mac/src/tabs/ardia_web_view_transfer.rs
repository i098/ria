// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Transfer.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The parts of `ArdiaWebView+Transfer.mm` that need no `ArdiaWebView` object: the saved tab history of a transfer
//! key, the keys bound to a restore, and the rule for which tab a view may take. `ensureTab`, `cloneTab`,
//! `takeTransferredTab`, `closeBrowser`, `StartParkedPage` and the `LiveViews` loops need the view object and stay
//! open (see `rust/MAP.md`). The two class methods `prepareTransfer:` and `releaseTransfer:` are in `page_engine.rs`.

use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

use serde_json::Value;

const MAX_SAVED_HISTORY_BYTES: u64 = 32 * 1024 * 1024;

/// `ardia_web_view::SamePage`: both URLs exist and are equal when the fragment is cut off.
pub fn same_page(a: Option<&str>, b: Option<&str>) -> bool {
    let strip = |url: &str| url.split('#').next().unwrap_or_default().to_owned();
    matches!((a, b), (Some(a), Some(b)) if strip(a) == strip(b))
}

/// `BoundRestoreKeys`: the transfer keys that a live tab holds for restore.
#[derive(Default)]
pub struct RestoreKeys(HashSet<String>);

impl RestoreKeys {
    /// `bindRestoreKey` (the set part).
    pub fn bind(&mut self, key: &str) {
        self.0.insert(key.to_owned());
    }

    /// `forgetRestoreKey`.
    pub fn forget(&mut self, key: &str) {
        self.0.remove(key);
    }

    pub fn contains(&self, key: &str) -> bool {
        self.0.contains(key)
    }
}

/// `SavedHistoryGeneration`, from the answer of `ac_tab_history_generation`. `marker` reads the file
/// `Ardia Tab History Cleared` of the profile: `None` when the file does not exist, `Some(text)` when it does.
pub fn history_generation(
    result: &Value,
    marker: impl FnOnce() -> Option<String>,
) -> Option<String> {
    if result.get("error").is_some() {
        return None;
    }
    if let Some(generation) = result.get("generation").and_then(Value::as_str) {
        return Some(generation.to_owned());
    }
    if result.get("fromMarker") == Some(&Value::Bool(true)) {
        return marker().map(|text| text.trim().to_owned());
    }
    None
}

/// The name of the history file of a key: the upper-case hex of its bytes.
pub fn history_file_name(key: &str) -> String {
    key.bytes().fold(String::new(), |mut name, byte| {
        let _ = write!(name, "{byte:02X}");
        name
    })
}

/// `RestoreSavedHistory`, the part before the window call: the saved tab state of `key`, or `None`. The file must be
/// version 1, no larger than 32 MiB, name this page (committed or shown URL) and carry the current `generation`.
/// `generation` is the answer of `history_generation`.
pub fn saved_history_state(
    profile_dir: &Path,
    key: &str,
    url: &str,
    generation: impl FnOnce() -> Option<String>,
) -> Option<String> {
    if key.is_empty() || url.is_empty() {
        return None;
    }
    let file = profile_dir
        .join("Ardia Tab History")
        .join(history_file_name(key));
    let length = std::fs::metadata(&file).ok()?.len();
    if length == 0 || length > MAX_SAVED_HISTORY_BYTES {
        return None;
    }
    let saved: Value = serde_json::from_slice(&std::fs::read(&file).ok()?).ok()?;
    if saved.get("version") != Some(&Value::from(1)) {
        return None;
    }
    let text = |name: &str| saved.get(name).and_then(Value::as_str);
    let state = text("state").filter(|state| !state.is_empty())?;
    if !(same_page(text("url"), Some(url)) || same_page(text("virtualUrl"), Some(url))) {
        return None;
    }
    let generation = generation()?;
    (text("generation") == Some(generation.as_str())).then(|| state.to_owned())
}

/// What `canTake:` reads of a tab and of the view.
pub struct TakeInput<'a> {
    pub tab_has_profile: bool,
    pub tab_closed: bool,
    pub tab_off_the_record: bool,
    pub view_private: bool,
    /// `IsOffTheRecordOf(tab profile, view profile)`.
    pub off_the_record_of_view: bool,
    pub tab_profile_name: &'a str,
    pub view_profile: &'a str,
}

/// `canTake:`.
pub fn can_take(tab: &TakeInput) -> bool {
    if !tab.tab_has_profile || tab.tab_closed || tab.tab_off_the_record != tab.view_private {
        return false;
    }
    if tab.tab_off_the_record {
        return tab.off_the_record_of_view;
    }
    tab.tab_profile_name == tab.view_profile
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn same_page_ignores_the_fragment_only() {
        assert!(same_page(Some("https://a/x#1"), Some("https://a/x#2")));
        assert!(!same_page(Some("https://a/x"), Some("https://a/y")));
        assert!(!same_page(None, Some("https://a/x")));
    }

    #[test]
    fn generation_prefers_the_engine_then_the_marker() {
        assert_eq!(
            history_generation(&json!({"generation": "g"}), || panic!("no marker")),
            Some("g".into())
        );
        assert_eq!(history_generation(&json!({"error": "x"}), || None), None);
        let from_marker = json!({"fromMarker": true});
        assert_eq!(
            history_generation(&from_marker, || Some(" m\n".into())),
            Some("m".into())
        );
        assert_eq!(history_generation(&from_marker, || None), None);
    }

    fn profile_with(file: &Value) -> tempdir::Dir {
        let dir = tempdir::Dir::new();
        let folder = dir.path().join("Ardia Tab History");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(history_file_name("k1")), file.to_string()).unwrap();
        dir
    }

    #[test]
    fn saved_history_needs_version_page_and_generation() {
        let file = json!({"version": 1, "state": "S", "url": "https://a/x#f", "generation": "g"});
        let dir = profile_with(&file);
        let read = |url: &str, key: &str, generation: &str| {
            let generation = generation.to_owned();
            saved_history_state(dir.path(), key, url, || Some(generation))
        };
        assert_eq!(read("https://a/x", "k1", "g"), Some("S".into()));
        assert_eq!(read("https://a/y", "k1", "g"), None);
        assert_eq!(read("https://a/x", "k1", "other"), None);
        assert_eq!(read("https://a/x", "k2", "g"), None);
        let old = profile_with(&json!({"version": 2, "state": "S", "url": "https://a/x"}));
        assert_eq!(
            saved_history_state(old.path(), "k1", "https://a/x", || Some("g".into())),
            None
        );
    }

    #[test]
    fn a_view_takes_only_a_live_tab_of_its_own_profile() {
        let own = TakeInput {
            tab_has_profile: true,
            tab_closed: false,
            tab_off_the_record: false,
            view_private: false,
            off_the_record_of_view: false,
            tab_profile_name: "p",
            view_profile: "p",
        };
        assert!(can_take(&own));
        assert!(!can_take(&TakeInput {
            view_profile: "q",
            ..own
        }));
    }

    mod tempdir {
        pub struct Dir(std::path::PathBuf);

        impl Dir {
            pub fn new() -> Self {
                let path = std::env::temp_dir().join(format!(
                    "ria-transfer-{}-{:?}",
                    std::process::id(),
                    std::thread::current().id()
                ));
                std::fs::create_dir_all(&path).unwrap();
                Self(path)
            }

            pub fn path(&self) -> &std::path::Path {
                &self.0
            }
        }

        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}
