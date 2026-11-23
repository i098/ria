// Ported from ardia packages/ardiacore/ios/ArdiaNativeMessaging.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Keeps the native messaging host manifests of the profile folder in step with the password managers that are installed.

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use serde_json::{Value, json};

use super::extension_package::is_extension_id;

/// Where Chrome keeps the system manifests.
pub const CHROME_SYSTEM_DIR: &str = "/Library/Google/Chrome/NativeMessagingHosts";
const MANAGED_LIST: &str = ".ardia-managed.json";
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

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

fn is_usable(manifest: &Value, name: &str) -> bool {
    manifest["name"] == name
        && manifest["type"] == "stdio"
        && manifest["description"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
        && manifest["allowed_origins"].as_array().is_some_and(|origins| {
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
            .is_none_or(Value::is_boolean)
        && manifest["path"]
            .as_str()
            .is_some_and(|p| Path::new(p).is_absolute() && is_executable(Path::new(p)))
}

fn system_manifest(system_dir: &Path, name: &str) -> Option<Vec<u8>> {
    let data = fs::read(manifest_path(system_dir, name)).ok()?;
    let parsed: Value = serde_json::from_slice(&data).ok()?;
    is_usable(&parsed, name).then_some(data)
}

/// The manifests to install, by host name: the system manifest when it is usable, else the built-in one.
fn installed_hosts(system_dir: &Path, known: &[Value]) -> Vec<(String, Vec<u8>)> {
    let mut hosts = Vec::new();
    for manifest in known {
        let name = manifest["name"].as_str().unwrap_or_default().to_owned();
        if let Some(system) = system_manifest(system_dir, &name) {
            hosts.push((name, system));
        } else if is_usable(manifest, &name) {
            let pretty = serde_json::to_vec_pretty(manifest).unwrap_or_default();
            hosts.push((name, pretty));
        }
    }
    for name in SYSTEM_REGISTERED_HOSTS {
        if let Some(system) = system_manifest(system_dir, name) {
            hosts.push(((*name).to_owned(), system));
        }
    }
    hosts
}

fn open_regular(path: &Path) -> std::io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(ErrorKind::InvalidData.into());
    }
    Ok(file)
}

fn mark_owned(file: &File) -> std::io::Result<()> {
    let fd = file.as_raw_fd();
    let name = MANAGED_ATTR.as_ptr();
    let value = std::ptr::null();
    // SAFETY: The descriptor and name are valid, and the attribute has no data.
    let status = unsafe {
        #[cfg(target_os = "macos")]
        {
            libc::fsetxattr(fd, name, value, 0, 0, 0)
        }
        #[cfg(not(target_os = "macos"))]
        {
            libc::fsetxattr(fd, name, value, 0, 0)
        }
    };
    if status == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn is_owned(file: &File) -> bool {
    let fd = file.as_raw_fd();
    let name = MANAGED_ATTR.as_ptr();
    let value = std::ptr::null_mut();
    // SAFETY: The descriptor and name are valid. A null buffer requests only the size.
    let size = unsafe {
        #[cfg(target_os = "macos")]
        {
            libc::fgetxattr(fd, name, value, 0, 0, 0)
        }
        #[cfg(not(target_os = "macos"))]
        {
            libc::fgetxattr(fd, name, value, 0)
        }
    };
    size == 0
}

fn write_owned(path: &Path, data: &[u8], replace: bool) -> std::io::Result<()> {
    let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    temp.write_all(data)?;
    mark_owned(temp.as_file())?;
    if replace {
        temp.persist(path)?;
    } else {
        temp.persist_noclobber(path)?;
    }
    Ok(())
}

fn migrate_ownership(dir: &Path, known: &[Value]) -> std::io::Result<()> {
    let list = dir.join(MANAGED_LIST);
    let file = match open_regular(&list) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let managed: BTreeSet<String> = serde_json::from_reader(file)?;
    for name in managed {
        if !SYSTEM_REGISTERED_HOSTS.contains(&name.as_str())
            && !known.iter().any(|manifest| manifest["name"] == name.as_str())
        {
            continue;
        }
        match open_regular(&manifest_path(dir, &name)) {
            Ok(file) => mark_owned(&file)?,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    fs::remove_file(list)
}

/// Updates the profile's native messaging manifests without replacing or removing unmanaged files.
pub fn sync_hosts(user_data_dir: &Path, system_dir: &Path) {
    let dir = user_data_dir.join("NativeMessagingHosts");
    let known = known_hosts();
    if fs::create_dir_all(&dir).is_err() || migrate_ownership(&dir, &known).is_err() {
        return;
    }
    let installed = installed_hosts(system_dir, &known);
    for name in known
        .iter()
        .filter_map(|manifest| manifest["name"].as_str())
        .chain(SYSTEM_REGISTERED_HOSTS.iter().copied())
    {
        let path = manifest_path(&dir, name);
        let existing = open_regular(&path);
        let source = installed.iter().find(|(host, _)| host == name);
        match (existing, source) {
            (Ok(mut file), Some((_, data))) if is_owned(&file) => {
                let mut old = Vec::new();
                if file.read_to_end(&mut old).is_err() || old != *data {
                    let _ = write_owned(&path, data, true);
                }
            }
            (Ok(file), None) if is_owned(&file) => {
                let _ = fs::remove_file(path);
            }
            (Err(error), Some((_, data))) if error.kind() == ErrorKind::NotFound => {
                let _ = write_owned(&path, data, false);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

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
    fn syncs_a_system_host_then_removes_it_when_it_goes() {
        let (user, system) = (temp("user"), temp("system"));
        let program = system.join("proxy");
        fs::write(&program, "").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        system_manifest_file(&system, "com.apple.passwordmanager", &program);

        sync_hosts(&user, &system);
        let hosts = user.join("NativeMessagingHosts");
        assert!(hosts.join("com.apple.passwordmanager.json").exists());

        fs::remove_file(manifest_path(&system, "com.apple.passwordmanager")).unwrap();
        sync_hosts(&user, &system);
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

        sync_hosts(&user, &system);
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
        assert!(!is_usable(&manifest, "n"));
        fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(is_usable(&manifest, "n"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_migrates_legacy_ownership_without_following_paths() {
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
        let managed = hosts.join(MANAGED_LIST);
        let outside = user.path().join("outside.json");
        fs::write(&outside, b"keep").unwrap();
        symlink(&outside, manifest.with_extension("json.tmp")).unwrap();
        symlink(&outside, managed.with_extension("json.tmp")).unwrap();
        fs::copy(manifest_path(system.path(), name), &manifest).unwrap();
        fs::write(&managed, format!(r#"["../outside","{name}"]"#)).unwrap();
        sync_hosts(user.path(), system.path());
        assert_eq!(fs::read(&outside).unwrap(), b"keep");
        assert!(!managed.exists());
        let source = manifest_path(system.path(), name);
        let mut updated: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        updated["description"] = json!("Updated host");
        fs::write(&source, updated.to_string()).unwrap();
        sync_hosts(user.path(), system.path());
        assert_eq!(fs::read_to_string(&manifest).unwrap(), updated.to_string());
        assert_eq!(fs::read(&outside).unwrap(), b"keep");
        fs::remove_file(manifest_path(system.path(), name)).unwrap();
        sync_hosts(user.path(), system.path());
        assert!(!manifest.exists());

        fs::write(&manifest, "replacement").unwrap();
        sync_hosts(user.path(), system.path());
        assert_eq!(fs::read(&manifest).unwrap(), b"replacement");
    }

    #[test]
    fn sync_does_not_publish_a_manifest_if_legacy_ownership_cannot_be_retired() {
        let user = tempfile::tempdir().unwrap();
        let system = tempfile::tempdir().unwrap();
        let program = system.path().join("proxy");
        fs::write(&program, "").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let name = "com.apple.passwordmanager";
        system_manifest_file(system.path(), name, &program);
        let hosts = user.path().join("NativeMessagingHosts");
        let legacy = hosts.join(MANAGED_LIST);
        fs::create_dir_all(&legacy).unwrap();
        let manifest = manifest_path(&hosts, name);

        sync_hosts(user.path(), system.path());
        assert!(!manifest.exists());
        fs::remove_dir(&legacy).unwrap();
        sync_hosts(user.path(), system.path());
        assert!(manifest.is_file());

        fs::remove_file(&manifest).unwrap();
        fs::write(&manifest, "replacement").unwrap();
        fs::remove_file(manifest_path(system.path(), name)).unwrap();
        sync_hosts(user.path(), system.path());
        assert_eq!(fs::read(&manifest).unwrap(), b"replacement");
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
            sync_hosts(user.path(), system.path());
            assert!(!target.exists(), "{value}");
        }
    }
}
