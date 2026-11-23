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
    team_signed: bool,
) -> io::Result<Vec<CString>> {
    let mock_keychain = config.isolated || !team_signed;
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
    ];
    if mock_keychain {
        switches.push("--use-mock-keychain".into());
    }
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
    switches.extend(config.engine_switches());
    if mock_keychain {
        if let Some(port) = config.remote_debugging_port {
            switches.push(format!("--remote-debugging-port={port}"));
            switches.push("--remote-allow-origins=*".into());
        }
        if let Some(extra) = config.instance("chromiumSwitches") {
            switches.extend(
                extra
                    .trim()
                    .strip_prefix("--")
                    .into_iter()
                    .flat_map(|extra| {
                        extra
                            .split(" --")
                            .filter_map(|item| {
                                let item = item.trim();
                                (!item.is_empty()).then(|| format!("--{item}"))
                            })
                            .filter(|arg| forwarded(arg, config.background))
                    }),
            );
        }
        let mut arguments = arguments.into_iter().peekable();
        while let Some(argument) = arguments.next() {
            let [b'-', first, ..] = argument.as_bytes() else {
                switches.push(argument);
                continue;
            };
            if first.is_ascii_alphabetic()
                && !argument.starts_with("-psn_")
                && arguments.peek().is_some()
            {
                arguments.next();
            } else if forwarded(&argument, config.background) {
                switches.push(argument);
            }
        }
    }
    let mut argv = vec![CString::new(executable.as_os_str().as_encoded_bytes())?];
    for argument in merge_features(switches) {
        argv.push(CString::new(argument)?);
    }
    Ok(argv)
}

fn forwarded(arg: &str, background: bool) -> bool {
    // Keep the engine inside the directory that launch configuration admitted.
    let switch = arg.trim_start_matches('-');
    !switch.starts_with("ardia-")
        && !switch.starts_with("user-data-dir")
        && !switch.starts_with("profile-directory")
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
        for (enable, prefix) in [(true, "--enable-features="), (false, "--disable-features=")] {
            if let Some(list) = arg.strip_prefix(prefix) {
                features.extend(
                    list.split(',')
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
        if seen.insert(feature.split(['<', ':']).next().unwrap_or_default()) {
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
}
