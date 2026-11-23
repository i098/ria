// Ported from ardia packages/ardiacore/ios/ArdiaNativeMessaging.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Keeps the native messaging host manifests of the profile folder in step with the password managers that are installed.

use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::path::Path;

use rustix::fs::{
    Access, AtFlags, CWD, Mode, OFlags, RenameFlags, XattrFlags, accessat, fgetxattr_query,
    fsetxattr, mkdirat, open, openat, renameat, renameat_with, unlinkat,
};
use serde_json::{Value, json};

use super::extension_package::is_extension_id;

/// Where Chrome keeps the system manifests.
const CHROME_SYSTEM_DIR: &str = "/Library/Google/Chrome/NativeMessagingHosts";
const MANAGED_ATTR: &std::ffi::CStr = c"user.ria.native-messaging";

fn known_hosts() -> Vec<Value> {
    vec![
        json!({
            "name": "com.1password.1password",
            "description": "1Password BrowserSupport",
            "path": "/Applications/1Password.app/Contents/Library/LoginItems/1Password Browser Helper.app/Contents/MacOS/1Password-BrowserSupport",
            "type": "stdio",
            "allowed_origins": [
                "chrome-extension://hjlinigoblmkhjejkmbegnoaljkphmgo/",
                "chrome-extension://gejiddohjgogedgjnonbofjigllpkmbf/",
                "chrome-extension://khgocmkkpikpnmmkgmdnfckapcdkgfaf/",
                "chrome-extension://aeblfdkhhhdcdjpifhhbdiojplfjncoa/",
                "chrome-extension://dppgmdbiimibapkepcbdbmkaabgiofem/",
            ],
        }),
        json!({
            "name": "com.8bit.bitwarden",
            "description": "Bitwarden desktop <-> browser bridge",
            "path": "/Applications/Bitwarden.app/Contents/MacOS/desktop_proxy",
            "type": "stdio",
            "allowed_origins": [
                "chrome-extension://nngceckbapebfimnlniiiahkandclblb/",
                "chrome-extension://hccnnhgbibccigepcmlgppchkpfdophk/",
                "chrome-extension://jbkfoedolllekgbhcbcoahefnbanhhlh/",
                "chrome-extension://ccnckbpmaceehanjmeomladnmlffdjgn/",
            ],
        }),
        json!({
            "name": "org.keepassxc.keepassxc_browser",
            "description": "KeePassXC integration with native messaging support",
            "path": "/Applications/KeePassXC.app/Contents/MacOS/keepassxc-proxy",
            "type": "stdio",
            "allowed_origins": [
                "chrome-extension://pdffhmdngciaglkoonimfcmckehcpafo/",
                "chrome-extension://oboonakemofpalcgghocfoadofidjkkk/",
            ],
        }),
        json!({
            "name": "me.proton.pass.nm",
            "description": "Proton Pass host for native messaging with the desktop app",
            "path": "/Applications/Proton Pass.app/Contents/Resources/assets/proton_pass_nm_host",
            "type": "stdio",
            "allowed_origins": [
                "chrome-extension://ghmbeldphafepmbegfdlkpapadhbakde/",
                "chrome-extension://hlaiofkbmjenhgeinjlmkafaipackfjh/",
                "chrome-extension://gcllgfdnfnllodcaambdaknbipemelie/",
            ],
        }),
    ]
}

/// Hosts that only the system folder can register.
const SYSTEM_REGISTERED_HOSTS: &[&str] = &["com.apple.passwordmanager"];

fn manifest_path(dir: &Path, name: &str) -> std::path::PathBuf {
    dir.join(format!("{name}.json"))
}

fn is_executable(path: &Path) -> std::io::Result<bool> {
    match fs::metadata(path) {
        Ok(metadata) if !metadata.is_file() => return Ok(false),
        Ok(_) => {}
        Err(error) if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
            return Ok(false);
        }
        Err(error) => return Err(error),
    }
    use rustix::io::Errno;
    match accessat(CWD, path, Access::EXEC_OK, AtFlags::EACCESS) {
        Ok(()) => Ok(true),
        Err(Errno::ACCESS | Errno::NOENT | Errno::NOTDIR) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn is_usable(manifest: &Value, name: &str) -> std::io::Result<bool> {
    let valid = manifest["name"] == name
        && manifest["type"] == "stdio"
        && manifest["description"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
        && manifest["allowed_origins"]
            .as_array()
            .is_some_and(|origins| {
                origins.iter().all(|origin| {
                    origin
                        .as_str()
                        .and_then(|s| s.strip_prefix("chrome-extension://"))
                        .and_then(|s| s.split_once('/'))
                        .is_some_and(|(id, _)| is_extension_id(id))
                })
            })
        && manifest
            .get("supports_native_initiated_connections")
            .is_none_or(Value::is_boolean);
    match manifest["path"].as_str().map(Path::new) {
        Some(path)
            if valid && path.is_absolute() && !path.as_os_str().as_encoded_bytes().contains(&0) =>
        {
            is_executable(path)
        }
        _ => Ok(false),
    }
}

fn system_manifest(system_dir: &Path, name: &str) -> std::io::Result<Option<Vec<u8>>> {
    let mut file = match File::open(manifest_path(system_dir, name)) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let Ok(parsed) = serde_json::from_slice(&data) else {
        return Ok(None);
    };
    Ok(is_usable(&parsed, name)?.then_some(data))
}

fn open_regular(dir: &File, name: &str) -> std::io::Result<File> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = File::from(openat(dir, name, flags, Mode::empty())?);
    if !file.metadata()?.is_file() {
        return Err(ErrorKind::InvalidData.into());
    }
    Ok(file)
}

fn mark_owned(file: &File) -> std::io::Result<()> {
    Ok(fsetxattr(file, MANAGED_ATTR, &[], XattrFlags::empty())?)
}

fn is_owned(file: &File) -> bool {
    fgetxattr_query(file, MANAGED_ATTR) == Ok(0)
}

fn write_manifest(
    dir: &File,
    name: &str,
    data: &[u8],
    replace: bool,
    mark: impl FnOnce(&File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let temp = format!(".ria-{:016x}.tmp", fastrand::u64(..));
    let flags = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC;
    let mut file = File::from(openat(dir, temp.as_str(), flags, Mode::RUSR | Mode::WUSR)?);
    let result = (|| {
        file.write_all(data)?;
        match mark(&file) {
            Ok(()) => {}
            Err(error) if !replace && error.kind() == ErrorKind::Unsupported => {}
            Err(error) => return Err(error),
        }
        if replace {
            renameat(dir, temp.as_str(), dir, name)?;
        } else {
            renameat_with(dir, temp.as_str(), dir, name, RenameFlags::NOREPLACE)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = unlinkat(dir, temp.as_str(), AtFlags::empty());
    }
    result
}

/// Updates native messaging manifests under the Chromium user data directory.
///
/// For an existing manifest, only the `user.ria.native-messaging` attribute marks Ria ownership.
/// The Ardia `.ardia-managed.json` inventory can name a file that the user replaced.
/// An I/O error that prevents reading the source or checking its executable leaves the registration unchanged.
pub fn sync_hosts(user_data_dir: &Path) {
    sync_hosts_from(user_data_dir, Path::new(CHROME_SYSTEM_DIR));
}

fn sync_hosts_from(user_data_dir: &Path, system_dir: &Path) {
    let Ok(dir) = open_hosts_dir(user_data_dir) else {
        return;
    };
    let known = known_hosts();
    for (name, builtin) in known
        .iter()
        .filter_map(|manifest| Some((manifest["name"].as_str()?, Some(manifest))))
        .chain(SYSTEM_REGISTERED_HOSTS.iter().map(|&name| (name, None)))
    {
        let Ok(source) = system_manifest(system_dir, name) else {
            continue;
        };
        let source = match (source, builtin) {
            (Some(data), _) => Some(data),
            (None, Some(manifest)) => {
                let Ok(usable) = is_usable(manifest, name) else {
                    continue;
                };
                if !usable {
                    None
                } else {
                    let Ok(data) = serde_json::to_vec_pretty(manifest) else {
                        continue;
                    };
                    Some(data)
                }
            }
            _ => None,
        };
        let file_name = format!("{name}.json");
        let existing = open_regular(&dir, &file_name);
        match (existing, source.as_deref()) {
            (Ok(mut file), Some(data)) if is_owned(&file) => {
                let mut old = Vec::new();
                if file.read_to_end(&mut old).is_err() || old != data {
                    let _ = write_manifest(&dir, &file_name, data, true, mark_owned);
                }
            }
            (Ok(file), None) if is_owned(&file) => {
                let _ = unlinkat(&dir, file_name.as_str(), AtFlags::empty());
            }
            (Err(error), Some(data)) if error.kind() == ErrorKind::NotFound => {
                let _ = write_manifest(&dir, &file_name, data, false, mark_owned);
            }
            _ => {}
        }
    }
}

fn open_hosts_dir(user_data_dir: &Path) -> std::io::Result<File> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    fs::create_dir_all(user_data_dir)?;
    let root = open(user_data_dir, flags, Mode::empty())?;
    let name = "NativeMessagingHosts";
    match mkdirat(&root, name, Mode::from_raw_mode(0o777)) {
        Ok(()) | Err(rustix::io::Errno::EXIST) => {}
        Err(error) => return Err(error.into()),
    }
    Ok(File::from(openat(
        &root,
        name,
        flags | OFlags::NOFOLLOW,
        Mode::empty(),
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn sync_never_writes_to_a_linked_hosts_directory() {
        let user = tempfile::tempdir().unwrap();
        let system = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let program = system.path().join("proxy");
        fs::write(&program, "").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
        system_manifest_file(system.path(), "com.apple.passwordmanager", &program);
        std::os::unix::fs::symlink(other.path(), user.path().join("NativeMessagingHosts")).unwrap();

        sync_hosts_from(user.path(), system.path());

        assert!(fs::read_dir(other.path()).unwrap().next().is_none());
    }

    #[test]
    fn unmarked_install_is_published_and_not_claimed_on_the_next_sync() {
        let user = tempfile::tempdir().unwrap();
        let system = tempfile::tempdir().unwrap();
        let dir = open_hosts_dir(user.path()).unwrap();
        let name = "com.apple.passwordmanager.json";
        let data = br#"{"name":"com.apple.passwordmanager"}"#;
        let unsupported = |_: &File| Err(rustix::io::Errno::NOTSUP.into());
        write_manifest(&dir, name, data, false, unsupported).unwrap();
        let hosts = user.path().join("NativeMessagingHosts");
        assert_eq!(fs::read(hosts.join(name)).unwrap(), data);

        sync_hosts_from(user.path(), system.path());

        assert_eq!(fs::read(hosts.join(name)).unwrap(), data);
    }

    #[test]
    fn failed_ownership_mark_leaves_no_manifest_for_the_next_sync() {
        let user = tempfile::tempdir().unwrap();
        let dir = open_hosts_dir(user.path()).unwrap();
        let hosts = user.path().join("NativeMessagingHosts");
        let name = "com.apple.passwordmanager.json";
        let transient = |_: &File| Err(rustix::io::Errno::IO.into());

        assert!(write_manifest(&dir, name, b"new", false, transient).is_err());
        assert!(fs::read_dir(&hosts).unwrap().next().is_none());
        write_manifest(&dir, name, b"first", false, mark_owned).unwrap();
        assert!(write_manifest(&dir, name, b"new", true, transient).is_err());
        assert_eq!(fs::read(hosts.join(name)).unwrap(), b"first");
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ria-nm-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn system_manifest_file(system: &Path, name: &str, path: &Path) {
        let manifest = json!({
            "name": name, "description": "Test host", "type": "stdio",
            "allowed_origins": ["chrome-extension://abcdefghijklmnopabcdefghijklmnop/"],
            "path": path,
        });
        fs::write(manifest_path(system, name), manifest.to_string()).unwrap();
    }

    #[test]
    fn sync_keeps_a_host_on_source_errors_and_removes_it_when_absent() {
        let (user, system) = (temp("user"), temp("system"));
        let program = system.join("proxy");
        fs::write(&program, "").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        system_manifest_file(&system, "com.apple.passwordmanager", &program);

        sync_hosts_from(&user, &system);
        let hosts = user.join("NativeMessagingHosts");
        assert!(hosts.join("com.apple.passwordmanager.json").exists());

        let source = manifest_path(&system, "com.apple.passwordmanager");
        let installed = fs::read(hosts.join("com.apple.passwordmanager.json")).unwrap();
        fs::remove_file(&program).unwrap();
        std::os::unix::fs::symlink("proxy", &program).unwrap();
        sync_hosts_from(&user, &system);
        assert_eq!(
            fs::read(hosts.join("com.apple.passwordmanager.json")).unwrap(),
            installed
        );

        fs::remove_file(&source).unwrap();
        fs::create_dir(&source).unwrap();
        sync_hosts_from(&user, &system);
        assert_eq!(
            fs::read(hosts.join("com.apple.passwordmanager.json")).unwrap(),
            installed
        );
        fs::remove_dir(source).unwrap();

        sync_hosts_from(&user, &system);
        assert!(!hosts.join("com.apple.passwordmanager.json").exists());
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&system);
    }

    #[test]
    fn a_manifest_that_the_app_did_not_write_is_kept() {
        let (user, system) = (temp("user2"), temp("system2"));
        let program = system.join("proxy");
        fs::write(&program, "").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        system_manifest_file(&system, "com.apple.passwordmanager", &program);
        let hosts = user.join("NativeMessagingHosts");
        fs::create_dir_all(&hosts).unwrap();
        fs::write(hosts.join("com.apple.passwordmanager.json"), "mine").unwrap();

        sync_hosts_from(&user, &system);
        assert_eq!(
            fs::read(hosts.join("com.apple.passwordmanager.json")).unwrap(),
            b"mine"
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&system);
    }

    #[test]
    fn a_host_that_is_not_executable_is_not_usable() {
        let dir = temp("exec");
        let file = dir.join("p");
        fs::write(&file, "").unwrap();
        let manifest = json!({
            "name": "n", "description": "Test host", "type": "stdio",
            "allowed_origins": [], "path": file,
        });
        assert!(!is_usable(&manifest, "n").unwrap());
        fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(is_usable(&manifest, "n").unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_updates_and_removes_owned_manifests_without_using_fixed_temp_paths() {
        use std::os::unix::fs::symlink;

        let user = tempfile::tempdir().unwrap();
        let system = tempfile::tempdir().unwrap();
        let program = system.path().join("proxy");
        fs::write(&program, "").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let name = "com.apple.passwordmanager";
        system_manifest_file(system.path(), name, &program);

        let hosts = user.path().join("NativeMessagingHosts");
        fs::create_dir(&hosts).unwrap();
        let manifest = manifest_path(&hosts, name);
        let outside = user.path().join("outside.json");
        fs::write(&outside, b"keep").unwrap();
        symlink(&outside, manifest.with_extension("json.tmp")).unwrap();
        sync_hosts_from(user.path(), system.path());
        assert_eq!(fs::read(&outside).unwrap(), b"keep");
        let source = manifest_path(system.path(), name);
        let mut updated: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        updated["description"] = json!("Updated host");
        fs::write(&source, updated.to_string()).unwrap();
        sync_hosts_from(user.path(), system.path());
        assert_eq!(fs::read_to_string(&manifest).unwrap(), updated.to_string());
        assert_eq!(fs::read(&outside).unwrap(), b"keep");
        fs::remove_file(manifest_path(system.path(), name)).unwrap();
        sync_hosts_from(user.path(), system.path());
        assert!(!manifest.exists());

        fs::write(&manifest, "replacement").unwrap();
        sync_hosts_from(user.path(), system.path());
        assert_eq!(fs::read(&manifest).unwrap(), b"replacement");
    }

    #[test]
    fn sync_does_not_adopt_a_replacement_or_stop_on_a_corrupt_list() {
        let user = tempfile::tempdir().unwrap();
        let system = tempfile::tempdir().unwrap();
        let program = system.path().join("proxy");
        fs::write(&program, "").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let name = "com.apple.passwordmanager";
        system_manifest_file(system.path(), name, &program);
        let hosts = user.path().join("NativeMessagingHosts");
        let legacy = hosts.join(".ardia-managed.json");
        fs::create_dir_all(&hosts).unwrap();
        let manifest = manifest_path(&hosts, name);
        fs::write(&manifest, "replacement").unwrap();
        fs::write(&legacy, json!([name]).to_string()).unwrap();
        sync_hosts_from(user.path(), system.path());
        assert_eq!(fs::read(&manifest).unwrap(), b"replacement");

        fs::write(&legacy, json!([name]).to_string()).unwrap();
        fs::remove_file(manifest_path(system.path(), name)).unwrap();
        sync_hosts_from(user.path(), system.path());
        assert_eq!(fs::read(&manifest).unwrap(), b"replacement");

        fs::remove_file(&manifest).unwrap();
        fs::write(&legacy, "[").unwrap();
        system_manifest_file(system.path(), name, &program);
        sync_hosts_from(user.path(), system.path());
        assert!(manifest.is_file());
        assert_eq!(fs::read(legacy).unwrap(), b"[");
    }

    #[test]
    fn chromium_manifest_fields_are_checked_before_copying_a_system_host() {
        let user = tempfile::tempdir().unwrap();
        let system = tempfile::tempdir().unwrap();
        let program = system.path().join("proxy");
        fs::write(&program, "").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let name = "com.apple.passwordmanager";
        system_manifest_file(system.path(), name, &program);
        let source = manifest_path(system.path(), name);
        let target = manifest_path(&user.path().join("NativeMessagingHosts"), name);
        let valid: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        for (key, invalid) in [
            ("description", Value::Null),
            ("allowed_origins", json!([17])),
            ("allowed_origins", json!(["<all_urls>"])),
            ("allowed_origins", json!(["chrome-extension://*/"])),
            ("allowed_origins", json!(["chrome-extension://abcd/"])),
            ("supports_native_initiated_connections", json!("true")),
        ] {
            let mut value = valid.clone();
            value[key] = invalid;
            fs::write(&source, value.to_string()).unwrap();
            sync_hosts_from(user.path(), system.path());
            assert!(!target.exists(), "{value}");
        }
    }
}
