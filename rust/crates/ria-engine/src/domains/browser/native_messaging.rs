// Ported from ardia packages/ardiacore/ios/ArdiaNativeMessaging.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Keeps the native messaging host manifests of the profile folder in step with the password managers that are installed.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

/// Where Chrome keeps the system manifests.
pub const CHROME_SYSTEM_DIR: &str = "/Library/Google/Chrome/NativeMessagingHosts";
const MANAGED_LIST: &str = ".ardia-managed.json";

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
        && manifest["allowed_origins"].is_array()
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
fn installed_hosts(system_dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut hosts = Vec::new();
    for manifest in known_hosts() {
        let name = manifest["name"].as_str().unwrap_or_default().to_owned();
        if let Some(system) = system_manifest(system_dir, &name) {
            hosts.push((name, system));
        } else if is_usable(&manifest, &name) {
            let pretty = serde_json::to_vec_pretty(&manifest).unwrap_or_default();
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

/// Writes `data` to `path` by a temporary file and a rename.
fn write_atomic(path: &Path, data: &[u8]) -> bool {
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, data).is_ok() && fs::rename(&temp, path).is_ok()
}

/// Updates `<user_data_dir>/NativeMessagingHosts`: writes the manifests of the installed hosts, and removes the ones
/// that this function wrote earlier (the `.ardia-managed.json` list) and that are no longer installed.
/// A manifest that this function did not write is never replaced or removed.
pub fn sync_hosts(user_data_dir: &Path, system_dir: &Path) {
    let dir = user_data_dir.join("NativeMessagingHosts");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let list_path = dir.join(MANAGED_LIST);
    let managed: BTreeSet<String> = fs::read(&list_path)
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or_default();

    let mut now_managed = BTreeSet::new();
    for (name, data) in installed_hosts(system_dir) {
        let path = manifest_path(&dir, &name);
        let ours = managed.contains(&name);
        if !ours && path.exists() {
            continue;
        }
        if fs::read(&path).ok().as_deref() != Some(&data) && !write_atomic(&path, &data) {
            continue;
        }
        now_managed.insert(name);
    }
    for name in managed.difference(&now_managed) {
        let _ = fs::remove_file(manifest_path(&dir, name));
    }
    if now_managed != managed {
        let names: Vec<&String> = now_managed.iter().collect();
        if let Ok(data) = serde_json::to_vec(&names) {
            let _ = write_atomic(&list_path, &data);
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
            "name": name, "type": "stdio", "allowed_origins": [], "path": path,
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
        let list: Vec<String> =
            serde_json::from_slice(&fs::read(hosts.join(MANAGED_LIST)).unwrap()).unwrap();
        assert_eq!(list, ["com.apple.passwordmanager"]);

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
        let manifest = json!({"name": "n", "type": "stdio", "allowed_origins": [], "path": file});
        assert!(!is_usable(&manifest, "n"));
        fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(is_usable(&manifest, "n"));
        let _ = fs::remove_dir_all(&dir);
    }
}
