// Ported from ardia packages/ardiacore/ios/ArdiaStartup.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::ffi::{c_char, c_int};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::Path;

use serde_json::Value;

use super::profile;

const MARKER: &str = ".ria-website-languages-kept";

unsafe extern "C" {
    fn kill(pid: c_int, signal: c_int) -> c_int;
    fn openat(directory: c_int, path: *const c_char, flags: c_int, ...) -> c_int;
    fn renameat(from_dir: c_int, from: *const c_char, to_dir: c_int, to: *const c_char) -> c_int;
    fn unlinkat(directory: c_int, path: *const c_char, flags: c_int) -> c_int;
}

pub(super) fn prepare(data: &Path) -> io::Result<()> {
    let chromium = data.join("Chromium");
    match fs::create_dir(&chromium) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let directory = profile::open_directory(&chromium)?;
    if profile::open_child(&directory, MARKER.as_ref()).is_err() && !running(&chromium) {
        for entry in fs::read_dir(&chromium)?.filter_map(Result::ok) {
            let Ok(folder) = profile::open_child(&directory, &entry.file_name()) else {
                continue;
            };
            if folder.metadata().is_ok_and(|meta| meta.is_dir()) {
                migrate(&folder);
            }
        }
        // SAFETY: this is a new private marker entry under the opened directory.
        let _ = create(&directory, c".ria-website-languages-kept");
    }
    ria_engine::domains::browser::native_messaging::sync_hosts(&chromium);
    Ok(())
}

fn running(chromium: &Path) -> bool {
    let Ok(target) = fs::read_link(chromium.join("SingletonLock")) else {
        return false;
    };
    let Some(pid) = target
        .to_str()
        .and_then(|text| text.rsplit('-').next()?.parse::<i32>().ok())
        .filter(|pid| *pid > 0)
    else {
        return false;
    };
    // SAFETY: signal zero checks a single positive PID without sending a signal.
    (unsafe { kill(pid, 0) }) == 0
        || io::Error::last_os_error().kind() == io::ErrorKind::PermissionDenied
}

fn migrate(folder: &File) {
    let Ok(mut file) = profile::open_child(folder, "Preferences".as_ref()) else {
        return;
    };
    if !file.metadata().is_ok_and(|meta| meta.is_file()) {
        return;
    }
    let mut text = String::new();
    if file.read_to_string(&mut text).is_err() {
        return;
    }
    if let Some(updated) = preserve_languages(&text) {
        let temp = c".ria-startup-preferences";
        let Ok(mut out) = create(folder, temp) else {
            return;
        };
        let fd = folder.as_raw_fd();
        // SAFETY: rename two entries under the same opened profile directory.
        if out.write_all(updated.as_bytes()).is_err()
            || unsafe { renameat(fd, temp.as_ptr(), fd, c"Preferences".as_ptr()) } != 0
        {
            // SAFETY: only the temporary entry we just created can be removed.
            unsafe { unlinkat(fd, temp.as_ptr(), 0) };
        }
    }
}

fn preserve_languages(text: &str) -> Option<String> {
    let mut prefs: Value = serde_json::from_str(text).ok()?;
    let intl = prefs.get("intl")?;
    let accept = intl.get("accept_languages")?.as_str()?;
    let selected = intl.get("selected_languages")?.as_str()?;
    if accept == selected {
        return None;
    }
    let old = format!(
        "\"selected_languages\":{}",
        serde_json::to_string(selected).ok()?
    );
    if text.matches(&old).count() != 1 {
        return None;
    }
    let new = format!(
        "\"selected_languages\":{}",
        serde_json::to_string(accept).ok()?
    );
    let updated = text.replacen(&old, &new, 1);
    let accept = accept.to_owned();
    prefs["intl"]["selected_languages"] = Value::String(accept);
    (serde_json::from_str::<Value>(&updated).ok()? == prefs).then_some(updated)
}

fn create(directory: &File, name: &std::ffi::CStr) -> io::Result<File> {
    const FLAGS: c_int = 0x0100_0000 | 0x0100 | 0x0800 | 0x0200 | 1;
    // SAFETY: the FD is open, name is NUL-terminated, and mode has the C variadic integer type.
    let fd = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), FLAGS, 0o600 as c_int) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat returned one new owned file descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(test)]
mod tests {
    use super::preserve_languages;

    #[test]
    fn changes_only_unambiguous_selected_languages() {
        let text = r#"{"intl":{"accept_languages":"ja,en","selected_languages":"en"},"font":42}"#;
        assert_eq!(
            preserve_languages(text).as_deref(),
            Some(r#"{"intl":{"accept_languages":"ja,en","selected_languages":"ja,en"},"font":42}"#)
        );
        assert_eq!(
            preserve_languages(
                r#"{"intl":{"accept_languages":"ja","selected_languages":"en"},"other":{"selected_languages":"en"}}"#
            ),
            None
        );
        assert_eq!(
            preserve_languages(
                r#"{"intl":{"accept_languages":"ja","selected_languages": "en"},"other":{"selected_languages":"en"}}"#
            ),
            None
        );
    }
}
