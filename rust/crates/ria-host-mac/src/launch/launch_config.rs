// Ported from ardia packages/ardiacore/ios/ArdiaLaunchConfig.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, ErrorKind};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use serde_json::Value;

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
    "cliInstallScript",
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
    "cliInstallScript",
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
    pub data_directory: PathBuf,
    pub config_directory: PathBuf,
    pub background: bool,
    pub remote_debugging_port: Option<u16>,
    pub prewarm: bool,
    pub isolated: bool,
    pub test_instance: bool,
    pub child_environment: BTreeMap<String, String>,
    instance: BTreeMap<String, String>,
    switches: BTreeMap<String, bool>,
    cached_switches: OnceLock<BTreeMap<String, bool>>,
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
        let mut prewarm = false;
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
                "RIA_PREWARM" => prewarm = !value.is_empty(),
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
        let support = home.join("Library/Application Support");
        let default_data = standardize(&support.join(BUNDLE_ID))?;
        let default_config = standardize(&home.join(".config/ria"))?;
        let ardia_roots = [
            standardize(&support.join("com.ardia.browser"))?,
            standardize(&support.join("Ardia"))?,
            standardize(&home.join(".config/ardia"))?,
        ];
        let in_ardia = |path: &Path| ardia_roots.iter().any(|root| overlaps(path, root));
        let scratch = data_dir
            .map(|path| standardize(&current_dir.join(path)))
            .transpose()?
            .filter(|path| path != &default_data);
        let isolated = scratch.is_some();
        let (data_directory, config_directory) = if let Some(data) = scratch {
            let config = standardize(&data.join("config"))?;
            if [&data, &config].into_iter().any(|path| {
                in_ardia(path) || overlaps(path, &default_data) || overlaps(path, &default_config)
            }) {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "RIA_DATA_DIR overlaps an installed profile or config directory",
                ));
            }
            (data, config)
        } else {
            if in_ardia(&default_data) || in_ardia(&default_config) {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "a default Ria directory overlaps an installed Ardia directory",
                ));
            }
            (default_data, default_config)
        };
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
            background,
            remote_debugging_port,
            prewarm,
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
        self.switch_override(key)
            .or_else(|| {
                self.cached_switches
                    .get_or_init(|| cached_switches(&self.data_directory))
                    .get(key)
                    .copied()
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

    pub fn engine_switches(&self) -> Vec<String> {
        // The prebuilt engine still uses the Ardia switch names.
        let mut args = Vec::new();
        if self.background {
            args.push("--ardia-background".into());
        }
        if self.isolated {
            args.push(format!(
                "--ardia-data-dir={}",
                self.data_directory.display()
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
        args
    }
}

fn standardize(path: &Path) -> io::Result<PathBuf> {
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
                    Ok(metadata) if metadata.is_symlink() => {
                        result = result.canonicalize()?;
                        if !result.metadata()?.is_dir() {
                            return Err(ErrorKind::NotADirectory.into());
                        }
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

fn overlaps(a: &Path, b: &Path) -> bool {
    fn contains(root: &Path, path: &Path) -> bool {
        let root = root.as_os_str().as_encoded_bytes();
        path.ancestors()
            .any(|part| part.as_os_str().as_encoded_bytes().eq_ignore_ascii_case(root))
    }
    contains(a, b) || contains(b, a)
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

fn cached_switches(directory: &Path) -> BTreeMap<String, bool> {
    let cache = std::fs::read(directory.join("switches-cache.json"))
        .ok()
        .and_then(|data| serde_json::from_slice::<Value>(&data).ok());
    cache
        .as_ref()
        .and_then(|value| value.get("switches"))
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(name, value)| Some((name.clone(), value.as_bool()?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::LaunchConfig;
    use std::path::Path;
    use std::{fs, os::unix::fs::symlink};

    #[test]
    fn normal_ria_root_keeps_test_settings_disabled() {
        let home = Path::new("/home/ria-launch-test");
        let root = home.join("Library/Application Support/com.i098.ria");
        let config = LaunchConfig::from_environment(
            [
                ("RIA_DATA_DIR", root.to_str().unwrap()),
                ("RIA_TEST", "loginsDelay=25,pipHover"),
                ("RIA_TRACE", "pip"),
            ],
            home,
            Path::new("/"),
            false,
        )
        .unwrap();

        assert_eq!(config.data_directory, root);
        assert_eq!(config.config_directory, home.join(".config/ria"));
        assert!(!config.isolated);
        assert!(!config.test_instance);
        assert!(!config.test_flag("pipHover"));
        assert!(!config.traced("pip"));
        assert!(config.engine_switches().is_empty());
    }

    #[test]
    fn rejects_ardia_roots_without_falling_back_to_ria() {
        let home = std::env::temp_dir().join(format!("ria-ardia-test-{}", std::process::id()));
        let support = home.join("Library/Application Support");
        let ardia = support.join("com.ardia.browser");
        let ria = support.join("com.i098.ria");
        let alias = home.join("alias");
        fs::create_dir_all(&ardia).unwrap();
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
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(config.data_directory, expected);
        assert_eq!(config.config_directory, expected.join("config"));
        assert_eq!(
            config.engine_switches(),
            [format!("--ardia-data-dir={}", expected.display())]
        );
    }

    #[test]
    fn filters_engine_settings_and_child_environment() {
        let expected = Path::new("/tmp").canonicalize().unwrap().join("run/profile");
        let config = LaunchConfig::from_environment(
            [
                ("RIA_DATA_DIR", "profile"),
                ("RIA_BACKGROUND", "false"),
                (
                    "RIA_INSTANCE",
                    "downloadsDir=/tmp/a,b,allowAudio=off,unknown=42",
                ),
                ("RIA_SWITCHES", "two=off,one=true,bad=yes"),
                ("RIA_TEST", "nativeDragRefused,loginsDelay=25,pipHover"),
                ("RIA_TRACE", "pip,visibility,nope,pip"),
                ("LANG", "en_US.UTF-8"),
            ],
            Path::new("/home/user"),
            Path::new("/tmp/run"),
            false,
        )
        .unwrap();
        assert_eq!(config.config_directory, expected.join("config"));
        assert!(!config.instance_flag("allowAudio"));
        assert!(config.test_flag("pipHover"));
        assert_eq!(
            config.engine_switches(),
            [
                "--ardia-background".to_owned(),
                format!("--ardia-data-dir={}", expected.display()),
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
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("switches-cache.json"),
            br#"{"switches":{"cached":false,"overridden":false,"ignored":0}}"#,
        )
        .unwrap();
        let config = LaunchConfig::from_environment(
            [
                ("RIA_DATA_DIR", root.to_str().unwrap()),
                ("RIA_SWITCHES", "overridden=on"),
            ],
            Path::new("/home/user"),
            Path::new("/tmp/run"),
            false,
        )
        .unwrap();
        let cached = config.switch_on("cached");
        let overridden = config.switch_on("overridden");
        let ignored = config.switch_on("ignored");
        std::fs::remove_dir_all(root).unwrap();
        assert!(!cached);
        assert!(overridden);
        assert!(ignored);
        assert!(!config.experiment_on("cached"));
        assert!(config.experiment_on("overridden"));
    }

    #[test]
    fn disables_trace_and_test_values_without_unsigned_scratch_profile() {
        for (data_dir, distribution) in [("", false), ("profile", true)] {
            let config = LaunchConfig::from_environment(
                [
                    ("RIA_DATA_DIR", data_dir),
                    ("RIA_TRACE", "pip"),
                    ("RIA_TEST", "loginsDelay=25"),
                ],
                Path::new("/home/user"),
                Path::new("/tmp/run"),
                distribution,
            )
            .unwrap();
            assert!(!config.traced("pip"));
            assert_eq!(config.test_value("loginsDelay"), None);
            assert!(
                !config.engine_switches().iter().any(
                    |arg| arg.starts_with("--ardia-test=") || arg.starts_with("--ardia-trace=")
                )
            );
            if data_dir.is_empty() {
                assert_eq!(
                    config.data_directory,
                    Path::new("/home/user/Library/Application Support/com.i098.ria")
                );
                assert_eq!(config.config_directory, Path::new("/home/user/.config/ria"));
            }
        }
    }
}
