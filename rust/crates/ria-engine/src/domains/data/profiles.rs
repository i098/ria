// Ported from ardia packages/ardiacore/ios/ArdiaProfiles.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The registry of loaded profiles: the name of a profile, its private session names, and loading on demand.
//! A profile name is the app name (`""` for the personal profile); the Chromium folder name is `Default` or `Profile <name>`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde_json::json;

use super::host::{CoreEngine, EventSink, Profile, ProfileLoadHooks};

type Waiter = Box<dyn FnOnce(Option<Profile>)>;

/// The Chromium folder name of an app profile name.
pub fn profile_directory_name(name: &str) -> String {
    if name.is_empty() {
        "Default".to_owned()
    } else {
        format!("Profile {name}")
    }
}

/// The app profile name of a Chromium folder name; `""` when it is not a `Profile <name>` folder.
pub fn profile_name_of_directory_name(directory: &str) -> String {
    directory
        .strip_prefix("Profile ")
        .unwrap_or_default()
        .to_owned()
}

/// A private session id looks like `incognito<id>@<profile>`.
pub fn is_private_session_id(name: &str) -> bool {
    name.starts_with("incognito")
}

/// The profile that a private session id belongs to; the name itself when it is not a private session id.
pub fn original_profile_name(name: &str) -> String {
    if !is_private_session_id(name) {
        return name.to_owned();
    }
    name.split_once('@')
        .map(|(_, original)| original.to_owned())
        .unwrap_or_default()
}

/// The name that `loadProfile` takes: a Chromium folder name is not a profile name (`Default`, `Profile x`).
fn directory_for_load(name: &str) -> Option<String> {
    if !name.is_empty() && (name == "Default" || name.starts_with("Profile ")) {
        return None;
    }
    Some(profile_directory_name(name))
}

/// `ardia_host::AppDisposition`: the app name of a Chromium window disposition.
pub fn app_disposition(chrome: &str) -> &'static str {
    match chrome {
        "foreground_tab" | "singleton_tab" => "foreground",
        "background_tab" => "background",
        "popup" => "popup",
        "window" => "window",
        "off_the_record" => "incognito",
        "current_tab" => "current",
        "split_view" => "split",
        _ => "foreground",
    }
}

#[derive(Default)]
struct State {
    profiles: HashMap<String, Profile>,
    waiters: HashMap<String, Vec<Waiter>>,
    deleted: HashSet<String>,
    released_private: HashSet<String>,
    deletions_after_load: HashMap<String, Waiter>,
}

struct Inner {
    core: Rc<dyn CoreEngine>,
    events: Rc<dyn EventSink>,
    hooks: RefCell<Option<Rc<dyn ProfileLoadHooks>>>,
    state: RefCell<State>,
}

/// The loaded profiles. Cheap to clone; every clone shares one state. Main thread only.
#[derive(Clone)]
pub struct ProfileRegistry(Rc<Inner>);

impl ProfileRegistry {
    pub fn new(core: Rc<dyn CoreEngine>, events: Rc<dyn EventSink>) -> Self {
        Self(Rc::new(Inner {
            core,
            events,
            hooks: RefCell::default(),
            state: RefCell::default(),
        }))
    }

    /// Sets what runs when a data profile is loaded (the content blocker and the download watch).
    pub fn set_load_hooks(&self, hooks: Rc<dyn ProfileLoadHooks>) {
        *self.0.hooks.borrow_mut() = Some(hooks);
    }

    fn remember(&self, name: &str, profile: Profile) {
        let fresh = {
            let mut state = self.0.state.borrow_mut();
            let fresh = !state
                .profiles
                .get(name)
                .is_some_and(|known| Rc::ptr_eq(known, &profile));
            state.profiles.insert(name.to_owned(), profile.clone());
            fresh
        };
        if fresh && !profile.off_the_record() && !is_private_session_id(name) {
            self.0
                .events
                .emit("profiles.loaded", json!({ "profile": name }));
            let hooks = self.0.hooks.borrow().clone();
            if let Some(hooks) = hooks {
                hooks.profile_loaded(name);
            }
        }
    }

    pub fn forget_destroyed(&self) {
        self.0
            .state
            .borrow_mut()
            .profiles
            .retain(|_, profile| !profile.destroyed());
    }

    pub fn loaded_profile_names(&self) -> Vec<String> {
        self.forget_destroyed();
        self.0.state.borrow().profiles.keys().cloned().collect()
    }

    /// The names of the loaded profiles that keep data (no private session).
    pub fn loaded_data_profile_names(&self) -> Vec<String> {
        self.forget_destroyed();
        self.0
            .state
            .borrow()
            .profiles
            .iter()
            .filter(|(name, profile)| !is_private_session_id(name) && !profile.off_the_record())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Each loaded profile once, even when it has two names.
    pub fn loaded_profiles(&self) -> Vec<Profile> {
        self.forget_destroyed();
        let mut unique: Vec<Profile> = Vec::new();
        for profile in self.0.state.borrow().profiles.values() {
            if !unique.iter().any(|known| Rc::ptr_eq(known, profile)) {
                unique.push(profile.clone());
            }
        }
        unique
    }

    pub fn loaded_profile(&self, name: &str) -> Option<Profile> {
        let mut state = self.0.state.borrow_mut();
        match state.profiles.get(name) {
            Some(profile) if !profile.destroyed() => Some(profile.clone()),
            Some(_) => {
                state.profiles.remove(name);
                None
            }
            None => None,
        }
    }

    pub fn is_private_profile(&self, name: &str) -> bool {
        let loaded = if name.is_empty() {
            None
        } else {
            self.loaded_profile(name)
        };
        loaded.map_or_else(
            || is_private_session_id(name),
            |profile| profile.off_the_record(),
        )
    }

    pub fn original_profile_name_of(&self, profile: &Profile) -> String {
        if !profile.off_the_record() {
            return self.profile_name(profile);
        }
        profile_name_of_directory_name(&profile.directory_name())
    }

    /// True when `profile` is the private profile of the session `name`.
    pub fn is_off_the_record_of(&self, profile: &Profile, name: &str) -> bool {
        profile.off_the_record()
            && is_private_session_id(name)
            && directory_for_load(&original_profile_name(name))
                .is_some_and(|directory| profile.directory_name() == directory)
    }

    pub fn is_deleted_profile(&self, name: &str) -> bool {
        self.0.state.borrow().deleted.contains(name)
    }

    /// Marks a profile name as deleted: `with_profile` answers `None` for it from now on.
    pub fn mark_deleted(&self, name: &str) {
        self.0.state.borrow_mut().deleted.insert(name.to_owned());
    }

    /// A private session name that the app released: no new private profile is made for it.
    pub fn mark_private_released(&self, name: &str) {
        let waiters = {
            let mut state = self.0.state.borrow_mut();
            state.released_private.insert(name.to_owned());
            state.profiles.remove(name);
            state.waiters.remove(name)
        };
        for waiter in waiters.into_iter().flatten() {
            waiter(None);
        }
    }

    /// Runs `deletion` when the load of `name` finishes, before the load waiters (`DeletionsAfterLoad`).
    pub fn delete_after_load(&self, name: &str, deletion: impl FnOnce(Option<Profile>) + 'static) {
        self.0
            .state
            .borrow_mut()
            .deletions_after_load
            .insert(name.to_owned(), Box::new(deletion));
    }

    /// The app name of a loaded profile.
    pub fn profile_name(&self, profile: &Profile) -> String {
        let known = self
            .0
            .state
            .borrow()
            .profiles
            .iter()
            .find(|(_, loaded)| Rc::ptr_eq(loaded, profile))
            .map(|(name, _)| name.clone());
        known.unwrap_or_else(|| profile_name_of_directory_name(&profile.directory_name()))
    }

    fn personal_if_loaded(&self) -> Option<Profile> {
        self.0
            .core
            .default_profile()
            .filter(|profile| profile.directory_name() == "Default")
    }

    /// Runs `completion` with the profile `name`, loading it first when it is not loaded. `None` when the profile
    /// is deleted, the engine is gone, or the name is a Chromium folder name.
    pub fn with_profile(&self, name: &str, completion: impl FnOnce(Option<Profile>) + 'static) {
        self.with_profile_boxed(name, Box::new(completion));
    }

    fn with_profile_boxed(&self, name: &str, completion: Waiter) {
        if self.is_deleted_profile(name) {
            return completion(None);
        }
        if let Some(profile) = self.loaded_profile(name) {
            return completion(Some(profile));
        }
        if is_private_session_id(name) {
            return self.with_private_profile(name, completion);
        }
        if name.is_empty()
            && let Some(personal) = self.personal_if_loaded()
        {
            self.remember("", personal.clone());
            return completion(Some(personal));
        }
        let Some(directory) = directory_for_load(name) else {
            eprintln!(
                "[ardiacore] no profile named \"{name}\": that's a Chromium folder name, not a profile"
            );
            return completion(None);
        };
        {
            let mut state = self.0.state.borrow_mut();
            if let Some(waiters) = state.waiters.get_mut(name) {
                waiters.push(completion);
                return;
            }
            state.waiters.insert(name.to_owned(), vec![completion]);
        }
        let (registry, owned) = (self.clone(), name.to_owned());
        self.0.core.load_profile(
            &directory,
            Box::new(move |loaded| registry.loaded(&owned, loaded)),
        );
    }

    fn loaded(&self, name: &str, loaded: Option<Profile>) {
        let (pending, deletion) = {
            let mut state = self.0.state.borrow_mut();
            (
                state.waiters.remove(name).unwrap_or_default(),
                state.deletions_after_load.remove(name),
            )
        };
        if let Some(deletion) = deletion {
            deletion(loaded.clone());
        }
        let profile = if self.is_deleted_profile(name) {
            None
        } else {
            loaded
        };
        if let Some(profile) = &profile {
            self.remember(name, profile.clone());
        }
        for waiter in pending {
            waiter(profile.clone());
        }
    }

    fn with_private_profile(&self, name: &str, completion: Waiter) {
        {
            let mut state = self.0.state.borrow_mut();
            if state.released_private.contains(name) {
                drop(state);
                return completion(None);
            }
            if let Some(waiters) = state.waiters.get_mut(name) {
                waiters.push(completion);
                return;
            }
            state.waiters.insert(name.to_owned(), vec![completion]);
        }
        let original = original_profile_name(name);
        let (registry, owned) = (self.clone(), name.to_owned());
        self.with_profile(&original.clone(), move |base| {
            if !registry.0.state.borrow().waiters.contains_key(&owned) {
                return;
            }
            let answer = {
                let (registry, owned, original) =
                    (registry.clone(), owned.clone(), original.clone());
                Box::new(move |otr: Option<Profile>, error: Option<String>| {
                    registry.private_profile_ready(&owned, &original, otr, error);
                })
            };
            match base {
                None => answer(None, Some(format!("profile \"{original}\" isn't loaded"))),
                Some(base) => registry.0.core.off_the_record_profile_for(&base, answer),
            }
        });
    }

    fn private_profile_ready(
        &self,
        name: &str,
        original: &str,
        otr: Option<Profile>,
        error: Option<String>,
    ) {
        let pending = self.0.state.borrow_mut().waiters.remove(name);
        let is_known = |profile: &Profile| {
            self.0
                .state
                .borrow()
                .profiles
                .values()
                .any(|known| Rc::ptr_eq(known, profile))
        };
        if self.is_deleted_profile(original) {
            if let Some(otr) = &otr
                && !is_known(otr)
            {
                self.0.core.release_profile(otr);
            }
            if pending.is_some() {
                eprintln!("[ardiacore] no private profile for {name}: its profile was deleted");
            }
            for waiter in pending.unwrap_or_default() {
                waiter(None);
            }
            return;
        }
        let Some(pending) = pending else {
            if let Some(otr) = &otr
                && !is_known(otr)
            {
                self.0.core.release_profile(otr);
            }
            return;
        };
        match &otr {
            Some(otr) if !otr.destroyed() => {
                self.0
                    .state
                    .borrow_mut()
                    .profiles
                    .insert(name.to_owned(), otr.clone());
            }
            _ => eprintln!(
                "[ardiacore] no private profile for {name}: {}",
                error.as_deref().unwrap_or("it went")
            ),
        }
        for waiter in pending {
            let current = self.0.state.borrow().profiles.get(name).cloned();
            waiter(current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_map_both_ways() {
        assert_eq!(profile_directory_name(""), "Default");
        assert_eq!(profile_directory_name("work"), "Profile work");
        assert_eq!(profile_name_of_directory_name("Profile work"), "work");
        assert_eq!(profile_name_of_directory_name("Default"), "");
        assert_eq!(original_profile_name("incognito7@work"), "work");
        assert_eq!(original_profile_name("incognito7"), "");
        assert_eq!(original_profile_name("work"), "work");
        assert_eq!(app_disposition("off_the_record"), "incognito");
        assert_eq!(app_disposition("unknown"), "foreground");
    }

    #[test]
    fn a_chromium_folder_name_is_not_a_profile_name() {
        assert_eq!(directory_for_load("Default"), None);
        assert_eq!(directory_for_load("Profile x"), None);
        assert_eq!(directory_for_load(""), Some("Default".to_owned()));
        assert_eq!(directory_for_load("x"), Some("Profile x".to_owned()));
    }
}
