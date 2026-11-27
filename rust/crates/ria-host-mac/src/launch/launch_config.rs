// Ported from ardia packages/ardiacore/ios/ArdiaLaunchConfig.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia:
//! - Ria admits the data directory once and opens it by handle. Admission, the `Chromium` link scan and the data
//!   file reads use that handle and never reopen the path; a swapped path is refused (`profile.rs`).
//! - A path that differs from a reserved Ardia path only by case is refused, because an absent path cannot say
//!   whether the volume ignores case.
//! - A switch cache that cannot be read from the admitted directory turns every switch without an override off.
//!   A missing cache still leaves the defaults.
//! - The engine gets the profile at the normal data path plus `Chromium`, and an isolated launch also passes
//!   the admitted directory as `/.vol/<device>/<inode>` for `--ardia-data-dir`. A swap of the path after
//!   admission cannot redirect the scratch data argument.
//! - After Chromium opens the profile, Ria checks the profile path again.
//!   Ria also requires the `SingletonLock` in the admitted `Chromium` directory to name this process.
//!   If the profile path changed, these checks reject the launch before the first window opens.
//!   Chromium can still open the replacement path before these checks.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, ErrorKind, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use serde_json::Value;

use super::profile;

pub const BUNDLE_ID: &str = "com.i098.ria";

const INSTANCE_KEYS: &[&str] = &[
    "chromiumSwitches",
    "downloadsDir",
    "allowAudio",
    "allowOcclusion",
    "releaseNotes",
    "cliHome",
    "cliBinDir",
    "cliUrl",
    "syncFolder",
    "sharedFolders",
    "importSourceDir",
    "importSafariHome",
    "officialPluginsFrom",
];
const ENGINE_TEST_KEYS: &[&str] = &[
    "keychain",
    "keychainHiddenReads",
    "keychainFail",
    "safeStorageAnswer",
    "reauth",
    "bluetoothChooser",
    "externalProtocolNoLaunch",
    "externalProtocolApp",
    "mediaRouteProvider",
    "systemMediaPermission",
    "applyProfileGate",
    "contextMenu",
    "loginsDelay",
    "loginsReleaseAfter",
    "cdpTabsVisible",
    "nativeDragRefused",
];
const APP_TEST_KEYS: &[&str] = &[
    "pipSelfTest",
    "pipHover",
    "fakeFullScreenMs",
    "shadersKeyWindow",
    "peekStrict",
    "peekMesh",
    "peekReady",
    "peekReduceMotion",
    "peekSlow",
    "importSecret",
    "importDiaBundleId",
    "importDiaWindows",
    "legacySource",
    "legacyCrashAfter",
    "bangsUrl",
    "onboarding",
    "whatsNew",
    "telemetry",
    "liveMock",
    "calendarFixture",
    "appMenu",
    "appDialog",
    "loginsTimeout",
    "screenRecording",
    "systemLocationPermission",
    "officialPin",
    "applePasswordsPanel",
    "systemPanelRace",
];
const COMMA_KEYS: &[&str] = &[
    "chromiumSwitches",
    "downloadsDir",
    "cliHome",
    "cliBinDir",
    "cliUrl",
    "syncFolder",
    "sharedFolders",
    "importSourceDir",
    "importSafariHome",
    "importSecret",
    "legacySource",
    "bangsUrl",
    "liveMock",
    "officialPin",
    "officialPluginsFrom",
];
const TRACE_AREAS: &[&str] = &[
    "visibility",
    "pip",
    "pipZoom",
    "trafficLights",
    "profileGate",
];

/// Launch settings for one process, read before Chromium starts.
pub struct LaunchConfig {
    data_directory: PathBuf,
    config_directory: PathBuf,
    data_handle: File,
    config_handle: Option<File>,
    pub background: bool,
    pub remote_debugging_port: Option<u16>,
    pub isolated: bool,
    pub test_instance: bool,
    pub child_environment: BTreeMap<String, String>,
    instance: BTreeMap<String, String>,
    switches: BTreeMap<String, bool>,
    cached_switches: OnceLock<Option<BTreeMap<String, bool>>>,
    trace: BTreeSet<String>,
    test: BTreeMap<String, String>,
}

impl LaunchConfig {
    /// Pass the result of the platform signature check as `signed_for_distribution`.
    pub fn from_environment<'a>(
        environment: impl IntoIterator<Item = (&'a str, &'a str)>,
        home: &Path,
        current_dir: &Path,
        signed_for_distribution: bool,
    ) -> io::Result<Self> {
        let mut data_dir = None;
        let mut background = false;
        let mut remote_debugging_port = None;
        let mut instance = BTreeMap::new();
        let mut switches = BTreeMap::new();
        let mut trace = BTreeSet::new();
        let mut test = BTreeMap::new();
        let mut child_environment = BTreeMap::new();
        for (name, value) in environment {
            match name {
                "RIA_DATA_DIR" => data_dir = present(value),
                "RIA_BACKGROUND" => background = !matches!(value, "" | "0"),
                "RIA_REMOTE_DEBUGGING_PORT" => remote_debugging_port = port(value),
                "RIA_INSTANCE" => instance = keyed_entries(value, INSTANCE_KEYS, &[], name),
                "RIA_SWITCHES" => {
                    switches = value.split(',').filter_map(switch_entry).collect();
                }
                "RIA_TRACE" => {
                    trace = value
                        .split(',')
                        .map(str::trim)
                        .filter(|area| !area.is_empty())
                        .filter(|area| {
                            let known = TRACE_AREAS.contains(area);
                            if !known {
                                eprintln!("[ria] RIA_TRACE: unknown area {area} ignored");
                            }
                            known
                        })
                        .map(str::to_owned)
                        .collect();
                }
                "RIA_TEST" => test = keyed_entries(value, ENGINE_TEST_KEYS, APP_TEST_KEYS, name),
                _ if name.starts_with("RIA_") => {}
                _ => {
                    child_environment.insert(name.to_owned(), value.to_owned());
                }
            }
        }
        let (data_directory, config_directory, isolated) =
            launch_directories(data_dir, home, current_dir)?;
        let config_handle = profile::directory_exists(&config_directory)?
            .then(|| profile::open_directory(&config_directory))
            .transpose()?;
        let data_handle = profile::claim(&data_directory)?;
        profile::check_tree(
            &data_handle,
            &[
                "SingletonLock",
                "SingletonSocket",
                "SingletonCookie",
                "RunningChromeVersion",
            ],
        )?;
        profile::ensure_identity(&data_directory, &data_handle)?;
        let test_instance = isolated && !signed_for_distribution;
        if !test_instance && (!trace.is_empty() || !test.is_empty()) {
            let reason = if isolated {
                "signed for distribution"
            } else {
                "no scratch RIA_DATA_DIR"
            };
            eprintln!("[ria] RIA_TEST and RIA_TRACE ignored: not a test instance ({reason})");
            trace.clear();
            test.clear();
        }
        Ok(Self {
            data_directory,
            config_directory,
            data_handle,
            config_handle,
            background,
            remote_debugging_port,
            isolated,
            test_instance,
            child_environment,
            instance,
            switches,
            cached_switches: OnceLock::new(),
            trace,
            test,
        })
    }

    pub fn data_directory(&self) -> &Path {
        &self.data_directory
    }

    /// The identity path for the isolated launch's `--ardia-data-dir`, not Chromium's `--user-data-dir`.
    pub(super) fn handoff_directory(&self) -> io::Result<PathBuf> {
        profile::identity_path(&self.data_handle)
    }

    /// Requires the engine's singleton lock in the admitted directory to name this process.
    pub(super) fn check_singleton(&self) -> io::Result<()> {
        profile::check_singleton(&self.data_handle)
    }

    pub(super) fn check_profile_directory(&self) -> io::Result<()> {
        profile::ensure_identity(&self.data_directory, &self.data_handle)?;
        profile::directory_exists(&self.data_directory.join("Chromium"))?;
        Ok(())
    }

    pub(super) fn read_data_file(&self, name: &Path) -> Option<Vec<u8>> {
        self.read_admitted_file(name).ok()
    }

    /// Reads a file of the admitted data directory.
    pub(super) fn read_admitted_file(&self, name: &Path) -> io::Result<Vec<u8>> {
        if name.file_name().map(Path::new) != Some(name) || !profile::has_marker(&self.data_handle)
        {
            return Err(ErrorKind::InvalidInput.into());
        }
        profile::read(&self.data_directory, &self.data_handle, name)
    }

    /// Replace a file without following a swapped profile path or leaving a partial document.
    pub(super) fn replace_data_file(&self, name: &Path, bytes: &[u8]) -> io::Result<()> {
        if name.file_name().map(Path::new) != Some(name) || !profile::has_marker(&self.data_handle)
        {
            return Err(ErrorKind::InvalidInput.into());
        }
        profile::ensure_identity(&self.data_directory, &self.data_handle)?;
        let root = self.handoff_directory()?;
        let temporary = root.join(format!(".ria-session-{}.tmp", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            profile::ensure_identity(&self.data_directory, &self.data_handle)?;
            std::fs::rename(&temporary, root.join(name))
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temporary);
        }
        result
    }

    pub fn read_config_file(&self, name: &Path) -> io::Result<Vec<u8>> {
        let directory = self.config_handle.as_ref().ok_or(ErrorKind::NotFound)?;
        profile::read(&self.config_directory, directory, name)
    }

    pub fn instance(&self, key: &str) -> Option<&str> {
        self.instance.get(key).and_then(|value| present(value))
    }

    pub fn instance_flag(&self, key: &str) -> bool {
        truthy(self.instance(key))
    }

    pub fn switch_override(&self, key: &str) -> Option<bool> {
        self.switches.get(key).copied()
    }

    pub fn switch_on(&self, key: &str) -> bool {
        // A cache that cannot be read from the admitted directory turns every switch off.
        self.switch_override(key)
            .or_else(|| {
                let cache = self.cached_switches.get_or_init(|| cached_switches(self));
                Some(
                    cache
                        .as_ref()
                        .is_some_and(|cache| cache.get(key).copied().unwrap_or(true)),
                )
            })
            .unwrap_or(true)
    }

    pub fn experiment_on(&self, key: &str) -> bool {
        self.switch_override(key).unwrap_or(false)
    }

    pub fn traced(&self, area: &str) -> bool {
        self.trace.contains(area)
    }

    pub fn test_value(&self, key: &str) -> Option<&str> {
        self.test.get(key).map(String::as_str)
    }

    pub fn test_flag(&self, key: &str) -> bool {
        truthy(self.test_value(key))
    }

    pub fn engine_switches(&self) -> io::Result<Vec<String>> {
        // The prebuilt engine still uses the Ardia switch names.
        let mut args = Vec::new();
        if self.background {
            args.push("--ardia-background".into());
        }
        if self.isolated {
            args.push(format!(
                "--ardia-data-dir={}",
                self.handoff_directory()?.display()
            ));
        }
        if let Some(downloads) = self.instance("downloadsDir") {
            args.push(format!("--ardia-downloads-dir={downloads}"));
        }
        if !self.switches.is_empty() {
            let switches = self
                .switches
                .iter()
                .map(|(key, &on)| {
                    let state = if on { "on" } else { "off" };
                    format!("{key}={state}")
                })
                .collect::<Vec<_>>();
            args.push(format!("--ardia-switches={}", switches.join(",")));
        }
        let test = ENGINE_TEST_KEYS
            .iter()
            .filter_map(|&key| {
                let value = self.test_value(key)?;
                Some(if value == "1" {
                    key.to_owned()
                } else {
                    format!("{key}={value}")
                })
            })
            .collect::<Vec<_>>();
        if !test.is_empty() {
            args.push(format!("--ardia-test={}", test.join(",")));
        }
        if !self.trace.is_empty() {
            let areas = self.trace.iter().map(String::as_str).collect::<Vec<_>>();
            args.push(format!("--ardia-trace={}", areas.join(",")));
        }
        Ok(args)
    }
}

fn launch_directories(
    override_path: Option<&str>,
    home: &Path,
    current_dir: &Path,
) -> io::Result<(PathBuf, PathBuf, bool)> {
    let installed_data = home.join("Library/Application Support").join(BUNDLE_ID);
    let installed_config = home.join(".config/ria");
    let ardia_data = home.join("Library/Application Support/com.ardia.browser");
    let ardia_config = home.join(".config/ardia");
    let (data, isolated) = if let Some(path) = override_path {
        let requested = current_dir.join(path);
        let data = standardize(&requested)?;
        let normal = requested.file_name() == installed_data.file_name()
            && requested
                .parent()
                .zip(installed_data.parent())
                .is_some_and(|(a, b)| {
                    standardize(a)
                        .ok()
                        .zip(standardize(b).ok())
                        .is_some_and(|(a, b)| a == b)
                });
        if !normal {
            reject_overlap(&data, &installed_data)?;
        }
        (data, !normal)
    } else {
        (standardize(&installed_data)?, false)
    };
    reject_overlap(&data, &installed_config)?;
    let config = if isolated {
        data.join("config")
    } else {
        standardize(&home.join(".config"))?.join("ria")
    };
    for selected in [&data, &config] {
        reject_overlap(selected, &ardia_data)?;
        reject_overlap(selected, &ardia_config)?;
    }
    Ok((data, config, isolated))
}

fn standardize(path: &Path) -> io::Result<PathBuf> {
    resolve_directory(path, false, &mut 40)
}

fn resolve_directory(path: &Path, allow_missing_link: bool, links: &mut u8) -> io::Result<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(name) => {
                result.push(name);
                match result.symlink_metadata() {
                    Ok(metadata) if metadata.is_dir() => {}
                    Ok(metadata) if metadata.is_symlink() && !allow_missing_link => {
                        if !result.metadata()?.is_dir() {
                            return Err(ErrorKind::NotADirectory.into());
                        }
                        result = result.canonicalize()?;
                    }
                    Ok(metadata) if metadata.is_symlink() => {
                        *links = links.checked_sub(1).ok_or(ErrorKind::InvalidInput)?;
                        let target = result.read_link()?;
                        result.pop();
                        result =
                            resolve_directory(&result.join(target), allow_missing_link, links)?;
                    }
                    Ok(_) => return Err(ErrorKind::NotADirectory.into()),
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            _ => result.push(component),
        }
    }
    Ok(result)
}

fn reject_overlap(path: &Path, reserved: &Path) -> io::Result<()> {
    let overlaps = |root: &Path| {
        path.ancestors().any(|parent| same_directory(parent, root))
            || root.ancestors().any(|parent| same_directory(parent, path))
    };
    let aliased = match resolve_directory(reserved, true, &mut 40) {
        Ok(destination) => destination != reserved && overlaps(&destination),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::NotADirectory | ErrorKind::InvalidInput
            ) =>
        {
            false
        }
        Err(error) => return Err(error),
    };
    if overlaps(reserved) || aliased {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "a launch directory overlaps a reserved profile or config directory",
        ));
    }
    Ok(())
}

fn same_directory(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.metadata(), b.metadata()) {
        (Ok(a), Ok(b)) => (a.dev(), a.ino()) == (b.dev(), b.ino()),
        (Err(a_err), Err(b_err))
            if a_err.kind() == ErrorKind::NotFound && b_err.kind() == ErrorKind::NotFound =>
        {
            // The volume may ignore case, and an absent path cannot say so.
            a.file_name().map(fold) == b.file_name().map(fold)
                && a.parent()
                    .zip(b.parent())
                    .is_some_and(|(a, b)| same_directory(a, b))
        }
        _ => false,
    }
}

fn fold(name: &std::ffi::OsStr) -> String {
    name.to_string_lossy().to_lowercase()
}

fn present(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

fn truthy(value: Option<&str>) -> bool {
    value.is_some_and(|value| !matches!(value, "0" | "off" | "false"))
}

fn port(value: &str) -> Option<u16> {
    let value = value.trim();
    let value = value.strip_prefix('+').unwrap_or(value);
    let end = value
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(value.len());
    let number: u16 = value[..end].parse().ok()?;
    (number != 0).then_some(number)
}

fn keyed_entries(
    text: &str,
    keys: &[&str],
    extra_keys: &[&str],
    variable: &str,
) -> BTreeMap<String, String> {
    fn head(raw: &str) -> &str {
        raw.split_once('=').map_or(raw, |(key, _)| key).trim()
    }
    let known = |key: &str| keys.contains(&key) || extra_keys.contains(&key);
    let mut entries: BTreeMap<String, String> = BTreeMap::new();
    let mut pieces = text.split(',').peekable();
    while let Some(piece) = pieces.next() {
        let mut piece = Cow::Borrowed(piece);
        if COMMA_KEYS.contains(&head(&piece)) {
            while let Some(next) = pieces.next_if(|next| !known(head(next))) {
                let joined = piece.to_mut();
                joined.push(',');
                joined.push_str(next);
            }
        }
        let (key, value) = piece.split_once('=').unwrap_or((&piece, "1"));
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        if !known(key) {
            eprintln!("[ria] {variable}: unknown key {key} ignored");
            continue;
        }
        entries.insert(key.to_owned(), value.trim().to_owned());
    }
    entries
}

fn switch_entry(piece: &str) -> Option<(String, bool)> {
    let (name, value) = piece.split_once('=')?;
    let name = present(name.trim())?;
    let on = match value.trim() {
        "on" | "1" | "true" => true,
        "off" | "0" | "false" => false,
        _ => return None,
    };
    Some((name.to_owned(), on))
}

fn cached_switches(config: &LaunchConfig) -> Option<BTreeMap<String, bool>> {
    let read = config.read_admitted_file(Path::new("switches-cache.json"));
    // A swapped directory is not a missing cache: an unchecked default could enable a disabled switch.
    config.check_profile_directory().ok()?;
    let cache = read
        .ok()
        .and_then(|data| serde_json::from_slice::<Value>(&data).ok());
    Some(
        cache
            .as_ref()
            .and_then(|value| value.get("switches"))
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(name, value)| Some((name.clone(), value.as_bool()?)))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::LaunchConfig;
    use std::{fs, os::unix::fs::symlink};

    #[test]
    fn normal_ria_root_keeps_test_settings_disabled() {
        let home = std::env::temp_dir().join(format!("ria-normal-test-{}", std::process::id()));
        fs::create_dir(&home).unwrap();
        let home = home.canonicalize().unwrap();
        let root = home.join("Library/Application Support/com.i098.ria");
        let config = LaunchConfig::from_environment(
            [
                ("RIA_DATA_DIR", root.to_str().unwrap()),
                ("RIA_TEST", "loginsDelay=25,pipHover"),
                ("RIA_TRACE", "pip"),
            ],
            &home,
            &home,
            false,
        )
        .unwrap();
        fs::remove_dir_all(&home).unwrap();

        assert_eq!(config.data_directory, root);
        assert_eq!(config.config_directory, home.join(".config/ria"));
        assert!(!config.isolated);
        assert!(!config.test_instance);
        assert!(!config.test_flag("pipHover"));
        assert!(!config.traced("pip"));
        assert!(config.engine_switches().unwrap().is_empty());
    }

    #[test]
    fn rejects_ardia_roots_without_falling_back_to_ria() {
        let home = std::env::temp_dir().join(format!("ria-ardia-test-{}", std::process::id()));
        let support = home.join("Library/Application Support");
        fs::create_dir_all(&support).unwrap();
        let ardia = home.join("old-scratch");
        let ria = support.join("com.i098.ria");
        let alias = support.join("com.ardia.browser");
        fs::create_dir(&ardia).unwrap();
        fs::write(ardia.join("session.json"), br#"{"version":11}"#).unwrap();
        symlink("missing-engine-data", ardia.join("Chromium")).unwrap();
        symlink(&ardia, &alias).unwrap();
        let scratch = home.join("scratch");
        fs::create_dir(&scratch).unwrap();
        symlink(&ardia, scratch.join("config")).unwrap();
        let dangling = home.join("dangling");
        symlink(ardia.join("missing"), &dangling).unwrap();

        let overrides = [&ardia, &alias, &scratch, &dangling].map(|directory| {
            LaunchConfig::from_environment(
                [("RIA_DATA_DIR", directory.to_str().unwrap())],
                &home,
                &home,
                false,
            )
        });
        symlink(&ardia, &ria).unwrap();
        let aliased_default = LaunchConfig::from_environment([], &home, &home, false);
        fs::remove_dir_all(&home).unwrap();

        for result in overrides {
            assert!(result.is_err());
        }
        assert!(aliased_default.is_err());
    }

    #[test]
    fn never_claims_the_installed_ardia_data_root() {
        let home = std::env::temp_dir().join(format!("ria-installed-ardia-{}", std::process::id()));
        let support = home.join("Library/Application Support");
        let ardia = support.join("com.ardia.browser");
        fs::create_dir_all(&support).unwrap();
        let launch = |path: &std::path::Path| {
            LaunchConfig::from_environment(
                [("RIA_DATA_DIR", path.to_str().unwrap())],
                &home,
                &home,
                false,
            )
        };
        assert!(launch(&ardia).is_err());
        assert!(!ardia.exists());
        fs::create_dir(&ardia).unwrap();
        assert!(launch(&ardia).is_err());
        symlink(&ardia, support.join("com.i098.ria")).unwrap();
        assert!(LaunchConfig::from_environment([], &home, &home, false).is_err());
        assert!(fs::read_dir(&ardia).unwrap().next().is_none());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn readers_refuse_replaced_profile_and_config_directories() {
        use super::super::saved_session::SavedSession;
        use std::path::Path;

        let home =
            std::env::temp_dir().join(format!("ria-replaced-profile-{}", std::process::id()));
        let launch = |name| {
            LaunchConfig::from_environment([("RIA_DATA_DIR", name)], &home, &home, false).unwrap()
        };
        let original = launch("original");
        fs::create_dir(&original.config_directory).unwrap();
        let original = launch("original");
        assert!(original.check_profile_directory().is_ok());
        let chromium = original.data_directory.join("Chromium");
        symlink(&original.data_directory, &chromium).unwrap();
        assert!(original.check_profile_directory().is_err());
        fs::remove_file(chromium).unwrap();
        let other = launch("other");
        fs::create_dir(&other.config_directory).unwrap();
        fs::write(other.config_directory.join("config.json"), b"{}").unwrap();
        fs::write(
            other.data_directory.join("session.json"),
            br#"{"version":11}"#,
        )
        .unwrap();
        fs::write(
            other.data_directory.join("switches-cache.json"),
            br#"{"switches":{"foreign":false}}"#,
        )
        .unwrap();

        fs::rename(&original.config_directory, home.join("old-config")).unwrap();
        fs::rename(&other.config_directory, &original.config_directory).unwrap();
        assert!(original.read_config_file(Path::new("config.json")).is_err());

        fs::rename(&original.data_directory, home.join("old-data")).unwrap();
        symlink(&other.data_directory, &original.data_directory).unwrap();
        assert!(original.check_profile_directory().is_err());
        assert!(SavedSession::read(&original).is_none());
        fs::remove_file(&original.data_directory).unwrap();
        fs::rename(&other.data_directory, &original.data_directory).unwrap();
        assert!(original.check_profile_directory().is_err());
        assert!(SavedSession::read(&original).is_none());
        assert!(!original.switch_on("foreign"));
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn case_alias_of_the_absent_ardia_root_is_rejected() {
        let home = std::env::temp_dir().join(format!("ria-case-alias-{}", std::process::id()));
        let support = home.join("Library/Application Support");
        fs::create_dir_all(&support).unwrap();
        let alias = support.join("com.ARDIA.browser");
        let result = LaunchConfig::from_environment(
            [("RIA_DATA_DIR", alias.to_str().unwrap())],
            &home,
            &home,
            false,
        );
        let created = alias.exists() || support.join("com.ardia.browser").exists();
        fs::remove_dir_all(home).unwrap();
        assert!(result.is_err());
        assert!(!created);
    }

    #[test]
    fn unreadable_switch_cache_turns_switches_off_but_a_missing_one_does_not() {
        let home = std::env::temp_dir().join(format!("ria-cache-closed-{}", std::process::id()));
        let launch = || {
            LaunchConfig::from_environment([("RIA_DATA_DIR", "p")], &home, &home, false).unwrap()
        };
        let config = launch();
        assert!(config.switch_on("x"));
        let swapped = launch();
        fs::write(
            swapped.data_directory.join("switches-cache.json"),
            br#"{"switches":{"x":false}}"#,
        )
        .unwrap();
        fs::rename(&swapped.data_directory, home.join("old")).unwrap();
        fs::create_dir(&swapped.data_directory).unwrap();
        assert!(!swapped.switch_on("x"));
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn dangling_unused_paths_do_not_block_launch() {
        let home = std::env::temp_dir().join(format!("ria-dangling-test-{}", std::process::id()));
        fs::create_dir_all(home.join(".config")).unwrap();
        symlink("missing", home.join(".config/ardia")).unwrap();
        let normal = LaunchConfig::from_environment([], &home, &home, false).unwrap();
        let ardia_config = home.join(".config/ardia");
        fs::remove_file(&ardia_config).unwrap();
        fs::write(&ardia_config, b"not a directory").unwrap();
        assert!(LaunchConfig::from_environment([], &home, &home, false).is_ok());
        fs::remove_file(&ardia_config).unwrap();
        symlink("ardia", &ardia_config).unwrap();
        symlink("missing", home.join(".config/ria")).unwrap();
        fs::remove_dir_all(&normal.data_directory).unwrap();
        symlink("missing", normal.data_directory).unwrap();
        let scratch =
            LaunchConfig::from_environment([("RIA_DATA_DIR", "scratch")], &home, &home, false)
                .unwrap();
        let resolved_home = home.canonicalize().unwrap();
        fs::remove_dir_all(&home).unwrap();

        assert!(!normal.isolated);
        assert_eq!(normal.config_directory, resolved_home.join(".config/ria"));
        assert_eq!(scratch.data_directory, resolved_home.join("scratch"));
        assert!(scratch.test_instance);
    }

    #[test]
    fn confines_config_roots_and_requested_files() {
        let home = std::env::temp_dir().join(format!("ria-shared-config-{}", std::process::id()));
        fs::create_dir_all(home.join(".config")).unwrap();
        fs::create_dir_all(home.join("old-scratch/config")).unwrap();
        fs::write(home.join("old-scratch/config/hooks.json"), b"{}").unwrap();
        symlink("../old-scratch/config", home.join(".config/ria")).unwrap();
        let aliased = LaunchConfig::from_environment([], &home, &home, false);
        fs::remove_file(home.join(".config/ria")).unwrap();
        fs::create_dir_all(home.join(".config/ria/themes")).unwrap();
        fs::write(home.join(".config/ria/config.json"), b"{}").unwrap();
        symlink("missing", home.join(".config/ria/themes/old-theme.json")).unwrap();
        symlink(
            "../../old-scratch/config/hooks.json",
            home.join(".config/ria/hooks.json"),
        )
        .unwrap();
        symlink("../../old-scratch/config", home.join(".config/ria/linked")).unwrap();
        let normal = LaunchConfig::from_environment([], &home, &home, false).unwrap();
        let local = normal.read_config_file(std::path::Path::new("config.json"));
        let escaped = [
            "hooks.json",
            "linked/hooks.json",
            "../../old-scratch/config/hooks.json",
        ]
        .map(|name| normal.read_config_file(std::path::Path::new(name)));

        let scratch =
            LaunchConfig::from_environment([("RIA_DATA_DIR", "scratch")], &home, &home, false)
                .unwrap();
        assert!(!scratch.config_directory.exists());
        symlink(&scratch.config_directory, home.join(".config/ardia")).unwrap();
        let shared_scratch =
            LaunchConfig::from_environment([("RIA_DATA_DIR", "scratch")], &home, &home, false);
        fs::create_dir(&scratch.config_directory).unwrap();
        let existing_shared_scratch =
            LaunchConfig::from_environment([("RIA_DATA_DIR", "scratch")], &home, &home, false);
        fs::remove_dir_all(&home).unwrap();

        assert!(aliased.is_err());
        assert_eq!(local.unwrap(), b"{}");
        assert!(escaped.iter().all(Result::is_err));
        assert!(shared_scratch.is_err());
        assert!(existing_shared_scratch.is_err());
    }

    #[test]
    fn rejects_reserved_aliases_before_creating_a_data_root() {
        let home = std::env::temp_dir().join(format!("ria-reserved-test-{}", std::process::id()));
        let support = home.join("Library/Application Support");
        let installed = support.join("com.i098.ria");
        fs::create_dir_all(&support).unwrap();
        fs::create_dir(home.join(".config")).unwrap();
        symlink("../../first-profile", &installed).unwrap();
        symlink("../config-target", home.join(".config/ria")).unwrap();
        symlink("../ardia-config-target", home.join(".config/ardia")).unwrap();

        for target in ["first-profile", "config-target", "ardia-config-target"] {
            let result =
                LaunchConfig::from_environment([("RIA_DATA_DIR", target)], &home, &home, false);
            assert!(result.is_err());
            assert!(!home.join(target).exists());
        }

        fs::remove_file(&installed).unwrap();
        symlink("../../caf\u{e9}", &installed).unwrap();
        let unrelated =
            LaunchConfig::from_environment([("RIA_DATA_DIR", "東京")], &home, &home, false)
                .unwrap();
        assert!(unrelated.data_directory().ends_with("東京"));
        assert!(!home.join("caf\u{e9}").exists());
        fs::remove_file(&installed).unwrap();
        fs::create_dir(home.join("caf\u{e9}")).unwrap();
        symlink("../../caf\u{e9}", &installed).unwrap();
        if home.join("cafe\u{301}").is_dir() {
            assert!(
                LaunchConfig::from_environment(
                    [("RIA_DATA_DIR", "cafe\u{301}")],
                    &home,
                    &home,
                    false,
                )
                .is_err()
            );
            assert!(!home.join("caf\u{e9}/.ria-profile").exists());
        }
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn rejects_nested_chromium_links_outside_a_marked_profile() {
        let home = std::env::temp_dir().join(format!("ria-chromium-link-{}", std::process::id()));
        let ardia = home.join("old-scratch/Chromium/Default");
        fs::create_dir_all(&ardia).unwrap();

        for override_path in ["", "scratch"] {
            let launch = || {
                LaunchConfig::from_environment(
                    [("RIA_DATA_DIR", override_path)],
                    &home,
                    &home,
                    false,
                )
            };
            let config = launch().unwrap();
            let default = config.data_directory.join("Chromium/Default");
            fs::create_dir_all(&default).unwrap();
            fs::write(default.join("Preferences"), b"{}").unwrap();
            symlink("missing", default.join("Dangling")).unwrap();
            assert!(launch().is_err());
            fs::remove_file(default.join("Dangling")).unwrap();
            symlink(
                "host-123",
                config.data_directory.join("Chromium/SingletonLock"),
            )
            .unwrap();
            symlink(
                "158.0.8127.36",
                config.data_directory.join("Chromium/RunningChromeVersion"),
            )
            .unwrap();
            assert!(launch().is_ok());
            symlink(
                ardia.join("Preferences"),
                default.join("RunningChromeVersion"),
            )
            .unwrap();
            fs::write(ardia.join("Preferences"), b"{}").unwrap();
            assert!(launch().is_err());
            fs::remove_dir_all(&default).unwrap();
            symlink(&ardia, &default).unwrap();
            assert!(launch().is_err());
        }

        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn resolves_a_symlink_before_parent_traversal_to_a_new_profile() {
        let root = std::env::temp_dir().join(format!("ria-path-test-{}", std::process::id()));
        fs::create_dir_all(root.join("other/nested")).unwrap();
        fs::create_dir(root.join("run")).unwrap();
        symlink("../other/nested", root.join("run/link")).unwrap();

        let config = LaunchConfig::from_environment(
            [("RIA_DATA_DIR", "link/../profile")],
            &root,
            &root.join("run"),
            false,
        )
        .unwrap();
        let expected = root.canonicalize().unwrap().join("other/profile");
        let expected_identity = {
            use std::os::unix::fs::MetadataExt;
            let meta = fs::metadata(&expected).unwrap();
            format!("/.vol/{}/{}", meta.dev(), meta.ino())
        };
        symlink("removed/../profile", root.join("other/dangling")).unwrap();
        assert!(
            LaunchConfig::from_environment(
                [("RIA_DATA_DIR", "other/dangling")],
                &root,
                &root,
                false,
            )
            .is_err()
        );
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(config.data_directory, expected);
        assert_eq!(config.config_directory, expected.join("config"));
        assert_eq!(
            config.engine_switches().unwrap(),
            [format!("--ardia-data-dir={expected_identity}")]
        );
    }

    #[test]
    fn filters_engine_settings_and_child_environment() {
        let home = std::env::temp_dir().join(format!("ria-filter-test-{}", std::process::id()));
        fs::create_dir(&home).unwrap();
        let home = home.canonicalize().unwrap();
        let expected = home.join("profile");
        let config = LaunchConfig::from_environment(
            [
                ("RIA_DATA_DIR", "profile"),
                ("RIA_BACKGROUND", "false"),
                (
                    "RIA_INSTANCE",
                    "downloadsDir=/tmp/a,b,allowAudio=off,unknown=42",
                ),
                ("RIA_SWITCHES", "two=off,one=true,bad=yes"),
                (
                    "RIA_TEST",
                    "cliInstallScript=/tmp/install,nativeDragRefused,loginsDelay=25,pipHover",
                ),
                ("RIA_TRACE", "pip,visibility,nope,pip"),
                ("RIA_PREWARM", "1"),
                ("LANG", "en_US.UTF-8"),
            ],
            &home,
            &home,
            false,
        )
        .unwrap();
        let expected_identity = {
            use std::os::unix::fs::MetadataExt;
            let meta = fs::metadata(&expected).unwrap();
            format!("/.vol/{}/{}", meta.dev(), meta.ino())
        };
        fs::remove_dir_all(&home).unwrap();
        assert_eq!(config.config_directory, expected.join("config"));
        assert!(!config.instance_flag("allowAudio"));
        assert!(config.test_flag("pipHover"));
        assert_eq!(config.test_value("cliInstallScript"), None);
        assert_eq!(
            config.engine_switches().unwrap(),
            [
                "--ardia-background".to_owned(),
                format!("--ardia-data-dir={expected_identity}"),
                "--ardia-downloads-dir=/tmp/a,b".to_owned(),
                "--ardia-switches=one=on,two=off".to_owned(),
                "--ardia-test=loginsDelay=25,nativeDragRefused".to_owned(),
                "--ardia-trace=pip,visibility".to_owned(),
            ]
        );
        assert_eq!(config.child_environment.len(), 1);
        assert_eq!(config.child_environment["LANG"], "en_US.UTF-8");
    }

    #[test]
    fn cached_switches_use_only_booleans_and_explicit_overrides_win() {
        let root = std::env::temp_dir().join(format!("ria-switches-test-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let home = root.join("home");
        let data = root.join("profile");
        let config = LaunchConfig::from_environment(
            [
                ("RIA_DATA_DIR", "profile"),
                ("RIA_SWITCHES", "overridden=on"),
            ],
            &home,
            &root,
            false,
        )
        .unwrap();
        std::fs::write(
            data.join("switches-cache.json"),
            br#"{"switches":{"cached":false,"overridden":false,"ignored":0}}"#,
        )
        .unwrap();
        let reopened =
            LaunchConfig::from_environment([("RIA_DATA_DIR", "profile")], &home, &root, false)
                .unwrap();
        let reopened_value = reopened.switch_on("overridden");
        let cached = config.switch_on("cached");
        let overridden = config.switch_on("overridden");
        let ignored = config.switch_on("ignored");
        let outside = root.join("outside.json");
        fs::rename(data.join("switches-cache.json"), &outside).unwrap();
        symlink(outside, data.join("switches-cache.json")).unwrap();
        let linked =
            LaunchConfig::from_environment([("RIA_DATA_DIR", "profile")], &home, &root, false)
                .unwrap();
        let ignored_link = linked.switch_on("cached");
        std::fs::remove_dir_all(root).unwrap();
        assert!(ignored_link);
        assert!(!cached);
        assert!(!reopened_value);
        assert!(overridden);
        assert!(ignored);
        assert!(!config.experiment_on("cached"));
        assert!(config.experiment_on("overridden"));
    }

    #[test]
    fn disables_trace_and_test_values_without_unsigned_scratch_profile() {
        let home = std::env::temp_dir().join(format!("ria-test-gate-{}", std::process::id()));
        fs::create_dir(&home).unwrap();
        let home = home.canonicalize().unwrap();
        for (data_dir, distribution) in [("", false), ("profile", true)] {
            let config = LaunchConfig::from_environment(
                [
                    ("RIA_DATA_DIR", data_dir),
                    ("RIA_TRACE", "pip"),
                    ("RIA_TEST", "loginsDelay=25"),
                ],
                &home,
                &home,
                distribution,
            )
            .unwrap();
            assert!(!config.traced("pip"));
            assert_eq!(config.test_value("loginsDelay"), None);
            assert!(
                !config.engine_switches().unwrap().iter().any(|arg| arg
                    .starts_with("--ardia-test=")
                    || arg.starts_with("--ardia-trace="))
            );
            if data_dir.is_empty() {
                assert_eq!(
                    config.data_directory,
                    home.join("Library/Application Support/com.i098.ria")
                );
                assert_eq!(config.config_directory, home.join(".config/ria"));
            }
        }
        fs::remove_dir_all(&home).unwrap();
    }
}
