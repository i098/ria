// Ported from ardia packages/ardiacore/ios/ArdiaNavigationDownloads.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The URLs that a navigation turned into a download, kept for 7 days (200 URLs at most) in
//! `NavigationDownloads.json`. A private profile keeps its URLs in memory only.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

const TTL: f64 = 7.0 * 86400.0;
const MAX_SAVED: usize = 200;

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

pub struct NavigationDownloads {
    path: PathBuf,
    saved: RefCell<Option<HashMap<String, f64>>>,
    private: RefCell<HashMap<String, HashMap<String, f64>>>,
}

impl NavigationDownloads {
    /// `data_directory` is `ArdiaHost.dataDirectory`.
    pub fn new(data_directory: &std::path::Path) -> Self {
        Self {
            path: data_directory.join("NavigationDownloads.json"),
            saved: RefCell::default(),
            private: RefCell::default(),
        }
    }

    /// Reads the file once; an entry that is not a number or is older than the TTL is dropped.
    fn with_saved<R>(&self, now: f64, body: impl FnOnce(&mut HashMap<String, f64>) -> R) -> R {
        let mut saved = self.saved.borrow_mut();
        let entries = saved.get_or_insert_with(|| {
            let read = std::fs::read(&self.path)
                .ok()
                .and_then(|data| serde_json::from_slice::<Value>(&data).ok());
            let Some(Value::Object(object)) = read else {
                return HashMap::new();
            };
            object
                .into_iter()
                .filter_map(|(url, when)| Some((url, when.as_f64()?)))
                .filter(|(_, when)| now - when <= TTL)
                .collect()
        });
        body(entries)
    }

    pub fn note(&self, url: &str, profile: &str, is_private: bool) {
        self.note_at(url, profile, is_private, now());
    }

    fn note_at(&self, url: &str, profile: &str, is_private: bool, now: f64) {
        if url.is_empty() {
            return;
        }
        if is_private {
            self.private
                .borrow_mut()
                .entry(profile.to_owned())
                .or_default()
                .insert(url.to_owned(), now);
            return;
        }
        let json = self.with_saved(now, |entries| {
            entries.insert(url.to_owned(), now);
            if entries.len() > MAX_SAVED {
                let mut by_age: Vec<_> = entries.iter().map(|(u, w)| (u.clone(), *w)).collect();
                by_age.sort_by(|a, b| a.1.total_cmp(&b.1));
                let excess = entries.len() - MAX_SAVED;
                for (url, _) in by_age.into_iter().take(excess) {
                    entries.remove(&url);
                }
            }
            let object: Map<String, Value> = entries
                .iter()
                .map(|(u, w)| (u.clone(), Value::from(*w)))
                .collect();
            Value::Object(object).to_string()
        });
        // The write is best effort, as `writeToFile:atomically:` without an error check is in Ardia.
        let temporary = self.path.with_extension("json.tmp");
        if std::fs::write(&temporary, json).is_ok() {
            let _ = std::fs::rename(&temporary, &self.path);
        }
    }

    pub fn was(&self, url: &str, profile: &str, is_private: bool) -> bool {
        self.was_at(url, profile, is_private, now())
    }

    fn was_at(&self, url: &str, profile: &str, is_private: bool, now: f64) -> bool {
        if url.is_empty() {
            return false;
        }
        let mut when = self.with_saved(now, |entries| entries.get(url).copied());
        if when.is_none() && is_private {
            when = self
                .private
                .borrow()
                .get(profile)
                .and_then(|m| m.get(url))
                .copied();
        }
        when.is_some_and(|when| now - when < TTL)
    }

    pub fn forget_private(&self, profile: &str) {
        self.private.borrow_mut().remove(profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_expire_after_7_days_and_private_ones_stay_in_memory() {
        let dir = std::env::temp_dir().join(format!("ria-navdl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = NavigationDownloads::new(&dir);
        store.note_at("https://a/f.zip", "", false, 1000.0);
        assert!(store.was_at("https://a/f.zip", "", false, 1000.0 + TTL - 1.0));
        assert!(!store.was_at("https://a/f.zip", "", false, 1000.0 + TTL));
        store.note_at("https://p/f.zip", "incognito1@x", true, 1000.0);
        assert!(store.was_at("https://p/f.zip", "incognito1@x", true, 1001.0));
        assert!(!store.was_at("https://p/f.zip", "x", false, 1001.0));
        store.forget_private("incognito1@x");
        assert!(!store.was_at("https://p/f.zip", "incognito1@x", true, 1001.0));
        assert!(
            !dir.join("NavigationDownloads.json").exists() || {
                let again = NavigationDownloads::new(&dir);
                !again.was_at("https://p/f.zip", "x", false, 1001.0)
            }
        );
        let again = NavigationDownloads::new(&dir);
        assert!(again.was_at("https://a/f.zip", "", false, 1001.0));
        for i in 0..205 {
            store.note_at(&format!("https://b/{i}"), "", false, 2000.0 + f64::from(i));
        }
        let kept = NavigationDownloads::new(&dir);
        assert!(!kept.was_at("https://b/0", "", false, 2300.0));
        assert!(kept.was_at("https://b/204", "", false, 2300.0));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
