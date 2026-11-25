// Ported from ardia packages/ardiacore/ios/ArdiaHost+Launch.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::HashSet;
use std::ffi::CString;
use std::io;
use std::path::Path;

use super::launch_config::LaunchConfig;

pub(super) fn engine_arguments(
    config: &LaunchConfig,
    executable: &Path,
    arguments: impl IntoIterator<Item = String>,
) -> io::Result<Vec<CString>> {
    let data = config.data_directory().join("Chromium");
    let data = data
        .to_str()
        .ok_or_else(|| io::Error::other("Chromium data path is not UTF-8"))?;
    let mut switches = vec![
        format!("--user-data-dir={data}"),
        "--enable-smooth-scrolling".into(),
        "--disable-stack-profiler".into(),
        "--disable-features=MacAppCodeSignClone,DevToolsAcceptDebuggingConnections,AutofillAiWalletPrivatePasses,SettingsGlowupJourneys,AsyncLiveResize".into(),
        "--enable-features=WebContentsDiscard,BrowserInitiatedAutomaticPictureInPicture,MacOcclusionForceShutoff,ImprovedStartupBestEffortDelay:StartupDelayStopOnLoadingTimedOut/true,CPUMeasurementInFreezingPolicy:freezing_visible_protection_time/0s".into(),
        "--profile-directory=Default".into(),
        "--js-flags=--no-memory-reducer".into(),
        "--use-mock-keychain".into(),
    ];
    if config.background {
        if !config.instance_flag("allowOcclusion") {
            switches.push("--disable-backgrounding-occluded-windows".into());
        }
        if !config.instance_flag("allowAudio") {
            switches.push("--mute-audio".into());
        }
        switches.push("--use-fake-device-for-media-stream".into());
        switches.push("--disable-modal-animations".into());
    }
    switches.extend(config.engine_switches()?);
    if config.test_instance
        && let Some(port) = config.remote_debugging_port
    {
        switches.push(format!("--remote-debugging-port={port}"));
    }
    if let Some(extra) = config.instance("chromiumSwitches") {
        switches.extend(
            extra
                .trim_matches(chromium_whitespace)
                .strip_prefix("--")
                .into_iter()
                .flat_map(|extra| {
                    extra
                        .split(" --")
                        .filter_map(|item| {
                            let item = item.trim_matches(chromium_whitespace);
                            (!item.is_empty()).then(|| format!("--{item}"))
                        })
                        .filter(|arg| forwarded(arg, config.background))
                }),
        );
    }
    let mut arguments = arguments.into_iter().map(normalize_argument).peekable();
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            switches.push(argument);
            switches.extend(arguments);
            break;
        }
        let [b'-', first, ..] = argument.as_bytes() else {
            switches.push(argument);
            continue;
        };
        if !forwarded(&argument, config.background) {
            continue;
        }
        if first.is_ascii_alphabetic()
            && !argument.starts_with("-psn_")
            && arguments.peek().is_some_and(|next| next != "--")
        {
            arguments.next();
        } else {
            switches.push(argument);
        }
    }
    let mut argv = vec![CString::new(executable.as_os_str().as_encoded_bytes())?];
    for argument in merge_features(switches) {
        argv.push(CString::new(argument)?);
    }
    Ok(argv)
}

fn chromium_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t'..='\r')
}

fn normalize_argument(mut argument: String) -> String {
    let start = argument.len() - argument.trim_start_matches(chromium_whitespace).len();
    let end = argument.trim_end_matches(chromium_whitespace).len();
    if start >= end {
        argument.clear();
    } else {
        argument.truncate(end);
        argument.drain(..start);
    }
    argument
}

fn forwarded(arg: &str, background: bool) -> bool {
    // Keep the engine inside the directory that launch configuration admitted.
    let switch = arg.trim_start_matches('-');
    let name = switch.split_once('=').map_or(switch, |(name, _)| name);
    !switch.starts_with("ardia-")
        && !switch.starts_with("user-data-dir")
        && !switch.starts_with("profile-directory")
        && name != "type"
        && !name.starts_with("remote-debugging-")
        && name != "remote-allow-origins"
        && !(background
            && (switch.starts_with("use-fake-ui-for-media-stream")
                || switch.starts_with("auto-accept-camera-and-microphone-capture")))
}

fn merge_features(mut switches: Vec<String>) -> Vec<String> {
    let positional = switches
        .iter()
        .position(|arg| arg == "--")
        .map(|i| switches.split_off(i));
    let mut features = Vec::new();
    switches.retain(|arg| {
        let switch = arg
            .strip_prefix("--")
            .or_else(|| arg.strip_prefix('-'))
            .unwrap_or("");
        for (enable, prefix) in [(true, "enable-features="), (false, "disable-features=")] {
            if let Some(list) = switch.strip_prefix(prefix) {
                features.extend(
                    list.split(',')
                        .map(str::trim)
                        .filter(|feature| !feature.is_empty())
                        .map(|feature| (enable, feature.to_owned())),
                );
                return false;
            }
        }
        true
    });
    let mut seen = HashSet::new();
    let mut enabled = Vec::new();
    let mut disabled = Vec::new();
    for (enable, feature) in features.iter().rev() {
        let name = feature.split(['<', ':']).next().unwrap_or_default();
        if !(*enable && name == "DevToolsAcceptDebuggingConnections") && seen.insert(name) {
            if *enable {
                enabled.push(feature.as_str());
            } else {
                disabled.push(feature.as_str());
            }
        }
    }
    for (prefix, list) in [
        ("--enable-features=", &mut enabled),
        ("--disable-features=", &mut disabled),
    ] {
        if !list.is_empty() {
            list.reverse();
            switches.push(format!("{prefix}{}", list.join(",")));
        }
    }
    switches.extend(positional.into_iter().flatten());
    switches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_feature_setting_wins_before_argument_terminator() {
        let input = [
            "--enable-features=A,B:old/yes,C",
            "--disable-features=A",
            "--enable-features=B<Trial",
            "--",
            "--disable-features=C",
        ];
        assert_eq!(
            merge_features(input.map(String::from).into()),
            [
                "--enable-features=C,B<Trial",
                "--disable-features=A",
                "--",
                "--disable-features=C"
            ]
        );
    }

    #[test]
    fn extra_arguments_cannot_change_the_admitted_profile() {
        assert!(!forwarded("--user-data-dir=/foreign", false));
        assert!(!forwarded("--profile-directory=foreign", false));
        assert!(!forwarded(
            "--auto-accept-camera-and-microphone-capture",
            true
        ));
        assert!(forwarded("--enable-features=A", true));
    }

    #[test]
    fn engine_argv_reserves_process_type_and_debugging() {
        let home = std::env::temp_dir().join(format!("ria-startup-{}", std::process::id()));
        for (directory, distribution, debugging) in [
            (None, false, false),
            (Some("scratch"), true, false),
            (Some("scratch"), false, true),
        ] {
            let settings = [
                ("RIA_REMOTE_DEBUGGING_PORT", "9224"),
                (
                    "RIA_INSTANCE",
                    "chromiumSwitches=  --type=renderer --remote-debugging-port=9333 --remote-allow-origins=* --enable-features=DevToolsAcceptDebuggingConnections,ConfigFeature",
                ),
            ];
            let config = LaunchConfig::from_environment(
                settings
                    .into_iter()
                    .chain(directory.map(|dir| ("RIA_DATA_DIR", dir))),
                &home,
                &home,
                distribution,
            )
            .unwrap();
            let argv = engine_arguments(
                &config,
                Path::new("ria"),
                [
                    "\t --type=renderer ",
                    "  --user-data-dir=/foreign ",
                    "\u{b}-profile-directory=foreign ",
                    " \t--remote-debugging-pipe\r",
                    "\t --enable-features=CliFeature ",
                    "-remote-debugging-port=9444",
                    " \t--\r ",
                    " --disable-features=Positional ",
                ]
                .map(String::from),
            )
            .unwrap();
            let argv: Vec<_> = argv.iter().map(|arg| arg.to_str().unwrap()).collect();
            assert!(argv.contains(&"--use-mock-keychain"));
            assert!(argv.iter().all(|arg| !arg.starts_with("--type")));
            assert!(!argv.contains(&"--remote-allow-origins=*"));
            let profile: Vec<_> = argv
                .iter()
                .filter(|arg| arg.starts_with("--user-data-dir="))
                .collect();
            assert_eq!(profile.len(), 1);
            assert!(profile[0].starts_with("--user-data-dir="));
            assert!(!profile[0].starts_with("--user-data-dir=/.vol/"));
            assert!(profile[0].ends_with("/Chromium"));
            assert!(!argv.iter().any(|arg| arg.contains("foreign")));
            assert!(argv.ends_with(&["--", "--disable-features=Positional"]));
            let debugger: Vec<_> = argv
                .iter()
                .copied()
                .filter(|arg| arg.trim_start_matches('-').starts_with("remote-debugging-"))
                .collect();
            assert_eq!(
                debugger,
                if debugging {
                    &["--remote-debugging-port=9224"][..]
                } else {
                    &[]
                }
            );
            assert!(
                argv.iter()
                    .any(|arg| arg.ends_with(",ConfigFeature,CliFeature"))
            );
            assert!(argv.iter().any(|arg| {
                arg.starts_with("--disable-features=")
                    && arg.contains("DevToolsAcceptDebuggingConnections")
            }));
        }
        std::fs::remove_dir_all(home).unwrap();
    }
}
