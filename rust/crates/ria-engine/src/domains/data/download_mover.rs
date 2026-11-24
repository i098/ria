// Ported from ardia packages/ardiacore/ios/ArdiaDownloadMover.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Moves a finished download into an approved folder under the home directory.
//! Moves between folders stage the file, verify its size and SHA-256, and rename it without replacing an existing file.
//! After an interruption, the journal lets a new mover recover a file it finds at a recorded path.
//!
//! Changes from Ardia (`ArdiaDownloadMover.mm` at 551d3c13c), each approved by the owner as a parity deviation
//! (issues 62 to 65 of `i098/ria`):
//! - A move uses the shortest approved root that matches, where Ardia uses the longest (`:347-348`, `:354`),
//!   so a nested approved folder never stops the move from creating the folders above it (issue 62).
//! - The claimed entry opens with `O_NONBLOCK`, where Ardia has none (`:414`), so a FIFO put in its place cannot
//!   block the mover before the type check (issue 63).
//! - A move is refused with `failed` when the staging path has no UTF-8 form, where Ardia journals an empty
//!   path (`:380`, `:388`) that recovery then joins with the file name (issue 64).
//! - A move into the folder that holds the file checks size and SHA-256, where Ardia returns `ok` at `:373`
//!   before any check (those run at `:416-420`); the file is not renamed (issue 65).
//! - The journal records each destination candidate before the rename to it, and recovery checks size and SHA-256
//!   of a staged or placed file, where Ardia records the path after the rename (`:694`) and recovers without a
//!   check (`:383-393`); a file that does not match is never put back or reported as the move (issue 52).
//! - A move reads the approved folders when it creates folders and before it places the file, where Ardia
//!   passes a copy taken at acceptance (`:305-325`), so revoking an approval stops a running move (issue 61).
//! - After the rename into the destination, a move checks again that the folder is still at the approved path and
//!   hashes the placed file again; a folder that left the approved root (checked before the rename too) or an edit
//!   through a descriptor that stayed open after the first hash puts the file back, where Ardia places it
//!   at `:426-427` and publishes it with no further check (issue 69).

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::fs::File;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, Read, Seek};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::MetadataExt;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::download_mover_names::name_with_suffix;
pub use super::download_mover_names::safe_file_name;

const MOVED_DEFAULT: &str = "ArdiaMovedDownloads";
const STAGING_PREFIX: &str = ".ardia-moving-";
const JOURNAL_DEFAULT: &str = "ArdiaDownloadMoveJournal";
const MAX_MOVED: usize = 200;
const MAX_RECORDS: usize = 1000;
const MAX_COLLISIONS: u32 = 999;
const MAX_STAGING_TRIES: u32 = 16;
const HASH_CHUNK: usize = 1 << 20;
const DIR_FLAGS: libc::c_int =
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;

/// The persistent key-value store of the mover (`NSUserDefaults` in the original).
pub trait Defaults: Send + Sync {
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&self, key: &str, value: Value);
    fn remove(&self, key: &str);
}

/// A `Defaults` that keeps values in memory.
#[cfg(test)]
#[derive(Default)]
pub struct MemoryDefaults(Mutex<HashMap<String, Value>>);

#[cfg(test)]
impl Defaults for MemoryDefaults {
    fn get(&self, key: &str) -> Option<Value> {
        self.0.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, value: Value) {
        self.0.lock().unwrap().insert(key.to_owned(), value);
    }
    fn remove(&self, key: &str) {
        self.0.lock().unwrap().remove(key);
    }
}

/// Called with the current step name and the staging folder path.
#[cfg(test)]
pub type StepHook = Arc<dyn Fn(&str, &str) + Send + Sync>;

struct Identity {
    sha256: String,
    size: i64,
}

#[derive(Default)]
struct State {
    approved: Vec<Vec<String>>,
    records: HashMap<String, Value>,
    record_order: Vec<String>,
    identities: HashMap<String, Identity>,
    moved: Vec<(String, String, String)>,
    in_flight: HashSet<String>,
}

pub struct DownloadMover {
    home: String,
    defaults: Arc<dyn Defaults>,
    state: Mutex<State>,
    persistence: Mutex<()>,
    #[cfg(test)]
    pub(super) step_hook: Mutex<Option<StepHook>>,
}

fn failure(reason: &str) -> Value {
    json!({"ok": false, "reason": reason})
}

fn staged_failure(reason: &str, staged_path: Option<String>) -> Value {
    match staged_path {
        Some(path) => json!({"ok": false, "reason": reason, "stagedPath": path}),
        None => failure(reason),
    }
}

fn folder_components(folder: &str) -> Option<Vec<String>> {
    let rest = folder.strip_prefix("~/")?.trim_end_matches('/');
    if rest.is_empty() {
        return None;
    }
    let parts: Vec<String> = rest.split('/').map(str::to_owned).collect();
    let bad = |part: &String| part.is_empty() || part == "." || part == ".." || part.contains('\0');
    if parts.iter().any(bad) {
        None
    } else {
        Some(parts)
    }
}

fn join(directory: &str, name: &str) -> String {
    if name.is_empty() {
        directory.to_owned()
    } else {
        format!("{}/{name}", directory.trim_end_matches('/'))
    }
}

fn last_component(path: &str) -> &str {
    path.trim_end_matches('/').rsplit('/').next().unwrap_or("")
}

fn c(text: &str) -> CString {
    // A name with a NUL becomes "", which every call below refuses.
    CString::new(text).unwrap_or_default()
}

fn check(result: libc::c_int) -> io::Result<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn errno(error: &io::Error) -> i32 {
    error.raw_os_error().unwrap_or(0)
}

fn open_at(dir: RawFd, name: &str, flags: libc::c_int) -> io::Result<File> {
    // SAFETY: `name` is a valid C string; a returned fd is owned by nobody else.
    let fd = unsafe { libc::openat(dir, c(name).as_ptr(), flags) };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

fn mkdir_at(dir: RawFd, name: &str, mode: libc::mode_t) -> io::Result<()> {
    // SAFETY: `name` is a valid C string.
    check(unsafe { libc::mkdirat(dir, c(name).as_ptr(), mode) })
}

fn remove_dir_at(dir: RawFd, name: &str) {
    // SAFETY: `name` is a valid C string.
    unsafe { libc::unlinkat(dir, c(name).as_ptr(), libc::AT_REMOVEDIR) };
}

#[derive(Clone, Copy)]
struct Stat {
    mode: u32,
    dev: u64,
    ino: u64,
    nlink: u64,
}

fn lstat_at(dir: RawFd, name: &str) -> io::Result<Stat> {
    // SAFETY: `info` is plain data that `fstatat` fills on success.
    let mut info: libc::stat = unsafe { std::mem::zeroed() };
    check(unsafe { libc::fstatat(dir, c(name).as_ptr(), &mut info, libc::AT_SYMLINK_NOFOLLOW) })?;
    #[allow(clippy::unnecessary_cast)]
    Ok(Stat {
        mode: info.st_mode as u32,
        dev: info.st_dev as u64,
        ino: info.st_ino as u64,
        nlink: info.st_nlink as u64,
    })
}

fn is_kind(mode: u32, kind: libc::mode_t) -> bool {
    #[allow(clippy::unnecessary_cast)]
    let (mask, kind) = (libc::S_IFMT as u32, kind as u32);
    mode & mask == kind
}

/// Renames without replacing an existing entry (`renameatx_np(RENAME_EXCL)`).
fn rename_excl(from_dir: RawFd, from: &str, to_dir: RawFd, to: &str) -> io::Result<()> {
    let (from, to) = (c(from), c(to));
    #[cfg(target_os = "linux")]
    // SAFETY: both names are valid C strings.
    let result = unsafe {
        libc::renameat2(
            from_dir,
            from.as_ptr(),
            to_dir,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(target_vendor = "apple")]
    // SAFETY: both names are valid C strings.
    let result = unsafe {
        libc::renameatx_np(
            from_dir,
            from.as_ptr(),
            to_dir,
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    check(result)
}

fn directory_path(dir: &File) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_link(format!("/proc/self/fd/{}", dir.as_raw_fd()))
            .ok()?
            .into_os_string()
            .into_string()
            .ok()
    }
    #[cfg(target_vendor = "apple")]
    {
        let mut path = [0 as libc::c_char; libc::PATH_MAX as usize];
        // SAFETY: `F_GETPATH` writes at most `MAXPATHLEN` bytes into `path`.
        check(unsafe { libc::fcntl(dir.as_raw_fd(), libc::F_GETPATH, path.as_mut_ptr()) }).ok()?;
        // SAFETY: on success `path` holds a NUL-terminated string.
        unsafe { std::ffi::CStr::from_ptr(path.as_ptr()) }
            .to_str()
            .ok()
            .map(str::to_owned)
    }
}

fn reason_for_missing_directory(parent: RawFd, name: &str) -> &'static str {
    match lstat_at(parent, name) {
        Err(error) => {
            if errno(&error) == libc::ENOENT {
                "folderMissing"
            } else {
                "folderUnreadable"
            }
        }
        Ok(info) if is_kind(info.mode, libc::S_IFLNK) => "symlink",
        Ok(info) if !is_kind(info.mode, libc::S_IFDIR) => "notAFolder",
        Ok(_) => "folderUnreadable",
    }
}

fn sha256_of(file: &mut File) -> Option<String> {
    file.rewind().ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_CHUNK];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => hasher.update(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

fn is_sha256(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    })
}

fn truthy(value: &Value) -> bool {
    value
        .as_bool()
        .unwrap_or_else(|| value.as_f64().is_some_and(|n| n != 0.0))
}

/// Opens `name` in `dir` without blocking and returns it with its metadata, if it is a regular file.
fn open_regular(dir: RawFd, name: &str) -> Option<(File, std::fs::Metadata)> {
    let file = open_at(
        dir,
        name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
    )
    .ok()?;
    let info = file.metadata().ok().filter(|info| info.is_file())?;
    Some((file, info))
}

/// Checks a file that stays in its folder against the identity of the download.
fn verify_in_place(dir: RawFd, name: &str, identity: &Identity, path: &str) -> Value {
    let Some((mut file, info)) = open_regular(dir, name) else {
        return failure("changed");
    };
    if info.nlink() != 1 {
        return failure("linked");
    }
    if info.size() as i64 != identity.size
        || sha256_of(&mut file).as_deref() != Some(identity.sha256.as_str())
    {
        return failure("changed");
    }
    json!({"ok": true, "path": path})
}

fn make_staging(source: RawFd) -> Option<String> {
    for _ in 0..MAX_STAGING_TRIES {
        let random = std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish();
        let name = format!("{STAGING_PREFIX}{random:016x}");
        match mkdir_at(source, &name, 0o700) {
            Ok(()) => return Some(name),
            Err(error) if errno(&error) == libc::EEXIST => {}
            Err(_) => return None,
        }
    }
    None
}

fn open_own_staging(source: RawFd, name: &str) -> Option<File> {
    let dir = open_at(source, name, DIR_FLAGS).ok()?;
    let info = dir.metadata().ok()?;
    // SAFETY: `getuid` has no preconditions.
    let uid = unsafe { libc::getuid() };
    (info.is_dir() && info.uid() == uid && info.mode() & 0o077 == 0).then_some(dir)
}

impl DownloadMover {
    pub fn new(home: &str, defaults: Arc<dyn Defaults>) -> Self {
        let mut state = State::default();
        if let Some(Value::Array(entries)) = defaults.get(MOVED_DEFAULT) {
            for entry in entries {
                if let (Some(id), Some(from), Some(to)) = (
                    entry["id"].as_str(),
                    entry["from"].as_str(),
                    entry["to"].as_str(),
                ) {
                    state
                        .moved
                        .push((id.to_owned(), from.to_owned(), to.to_owned()));
                }
            }
        }
        let mover = Self {
            home: home.to_owned(),
            defaults,
            state: Mutex::new(state),
            persistence: Mutex::new(()),
            #[cfg(test)]
            step_hook: Mutex::new(None),
        };
        mover.reconcile_journal();
        mover
    }

    fn remember(state: &mut State, id: &str, from: &str, to: &str) -> Value {
        let from = state
            .moved
            .iter()
            .rev()
            .find(|entry| entry.0 == id)
            .map_or(from, |entry| entry.1.as_str())
            .to_owned();
        state.moved.push((id.to_owned(), from, to.to_owned()));
        if state.moved.len() > MAX_MOVED {
            let extra = state.moved.len() - MAX_MOVED;
            state.moved.drain(..extra);
        }
        let saved = state
            .moved
            .iter()
            .map(|(id, from, to)| json!({"id": id, "from": from, "to": to}))
            .collect();
        Value::Array(saved)
    }

    fn journal(&self, id: &str, entry: Option<&Map<String, Value>>) {
        let _guard = self.persistence.lock().unwrap();
        let mut journal = match self.defaults.get(JOURNAL_DEFAULT) {
            Some(Value::Object(journal)) => journal,
            _ => Map::new(),
        };
        match entry {
            Some(entry) => journal.insert(id.to_owned(), Value::Object(entry.clone())),
            None => journal.remove(id),
        };
        if journal.is_empty() {
            self.defaults.remove(JOURNAL_DEFAULT);
        } else {
            self.defaults.set(JOURNAL_DEFAULT, Value::Object(journal));
        }
    }

    fn reconcile_journal(&self) {
        let Some(Value::Object(journal)) = self.defaults.get(JOURNAL_DEFAULT) else {
            return;
        };
        let state = &mut *self.state.lock().unwrap();
        for (id, entry) in &journal {
            let (Some(source), Some(staging), Some(name)) = (
                entry["source"].as_str(),
                entry["staging"].as_str(),
                entry["name"].as_str(),
            ) else {
                continue;
            };
            let staged = join(staging, name);
            let identity = match (entry["sha256"].as_str(), entry["size"].as_i64()) {
                (Some(sha256), Some(size)) => Some(Identity {
                    sha256: sha256.to_owned(),
                    size,
                }),
                _ => None,
            };
            let matches = |path: &str| {
                identity.as_ref().is_some_and(|identity| {
                    verify_in_place(libc::AT_FDCWD, path, identity, path)["ok"] == true
                })
            };
            let regular = |path: &str| {
                lstat_at(libc::AT_FDCWD, path).is_ok_and(|info| is_kind(info.mode, libc::S_IFREG))
            };
            // A rename leaves the file at one path only, so a staged file means the rename to `placed` did not happen.
            if regular(&staged) {
                let source_free = matches!(lstat_at(libc::AT_FDCWD, source), Err(error) if errno(&error) == libc::ENOENT);
                if matches(&staged)
                    && (!source_free
                        || rename_excl(libc::AT_FDCWD, &staged, libc::AT_FDCWD, source).is_err())
                {
                    self.defaults
                        .set(MOVED_DEFAULT, Self::remember(state, id, source, &staged));
                }
            } else if let Some(placed) = entry["placed"].as_str().filter(|placed| matches(placed)) {
                self.defaults
                    .set(MOVED_DEFAULT, Self::remember(state, id, source, placed));
            }
            remove_dir_at(libc::AT_FDCWD, staging);
        }
        self.defaults.remove(JOURNAL_DEFAULT);
    }

    /// Records a download event and returns it, with the path rewritten when the download was moved.
    pub fn note_download_event(&self, download: &Value) -> Value {
        let Some(id) = download["id"].as_str().filter(|id| !id.is_empty()) else {
            return download.clone();
        };
        let state = &mut *self.state.lock().unwrap();
        let mut event = download.clone();
        if let Some((_, from, to)) = state.moved.iter().rev().find(|entry| entry.0 == id)
            && download["path"].as_str() == Some(from.as_str())
        {
            event["path"] = json!(to);
            event["filename"] = json!(last_component(to));
        }
        if !state.records.contains_key(id) {
            state.record_order.push(id.to_owned());
            if state.record_order.len() > MAX_RECORDS {
                let oldest = state.record_order.remove(0);
                state.records.remove(&oldest);
                state.identities.remove(&oldest);
            }
        }
        if !state.identities.contains_key(id)
            && event["state"] == "finished"
            && !truthy(&event["dangerous"])
            && !truthy(&event["offTheRecord"])
            && is_sha256(&event["sha256"])
            && event["received"].is_number()
        {
            let size = event["received"]
                .as_i64()
                .unwrap_or_else(|| event["received"].as_f64().unwrap_or(0.0) as i64);
            let sha256 = event["sha256"].as_str().unwrap_or_default().to_owned();
            state
                .identities
                .insert(id.to_owned(), Identity { sha256, size });
        }
        state.records.insert(id.to_owned(), event.clone());
        event
    }

    /// Sets the folders, besides `~/Downloads`, that moves may target; entries that are not `~/` paths are dropped.
    pub fn set_approved_folders(&self, folders: &[String]) {
        self.state.lock().unwrap().approved = folders
            .iter()
            .filter_map(|folder| folder_components(folder))
            .collect();
    }

    pub fn start_move(
        self: &Arc<Self>,
        path: String,
        folder: String,
    ) -> io::Result<JoinHandle<Value>> {
        let mover = Arc::clone(self);
        thread::Builder::new()
            .name("ria-download-move".into())
            .spawn(move || mover.move_download(&path, &folder))
    }

    pub(super) fn move_download(&self, path: &str, folder: &str) -> Value {
        if path.is_empty() {
            return failure("notADownload");
        }
        let (id, record, identity) = {
            let state = &mut *self.state.lock().unwrap();
            let id = state
                .record_order
                .iter()
                .rev()
                .find(|id| state.records[*id]["path"].as_str() == Some(path))
                .cloned();
            let Some(id) = id else {
                return failure("notADownload");
            };
            let record = state.records[&id].clone();
            let refusal = if truthy(&record["offTheRecord"]) {
                Some("private")
            } else if truthy(&record["dangerous"]) {
                Some("dangerous")
            } else if record["state"] != "finished" {
                Some("notFinished")
            } else if !state.identities.contains_key(&id) {
                Some("noIdentity")
            } else if state.in_flight.contains(&id) {
                Some("busy")
            } else {
                None
            };
            if let Some(refusal) = refusal {
                return failure(refusal);
            }
            state.in_flight.insert(id.clone());
            let identity = &state.identities[&id];
            (
                id,
                record,
                Identity {
                    sha256: identity.sha256.clone(),
                    size: identity.size,
                },
            )
        };
        let result = self.run_move(&id, record, &identity, path, folder);
        self.state.lock().unwrap().in_flight.remove(&id);
        result
    }

    /// The length of the shortest approved root that holds `components`, or 0; reads the approvals now.
    fn root_length(&self, components: &[String]) -> usize {
        let state = self.state.lock().unwrap();
        let downloads = vec!["Downloads".to_owned()];
        state
            .approved
            .iter()
            .chain([&downloads])
            .filter(|root| components.starts_with(root))
            .map(Vec::len)
            .min()
            .unwrap_or(0)
    }

    #[cfg(test)]
    fn step(&self, name: &str, staging: &File) {
        let hook = self.step_hook.lock().unwrap().clone();
        if let Some(hook) = hook {
            hook(name, &directory_path(staging).unwrap_or_default());
        }
    }

    fn run_move(
        &self,
        id: &str,
        mut record: Value,
        identity: &Identity,
        path: &str,
        folder: &str,
    ) -> Value {
        let Some(components) = folder_components(folder) else {
            return failure("folderNotAllowed");
        };
        let root_length = self.root_length(&components);
        if root_length == 0 {
            return failure("folderNotAllowed");
        }

        let Ok(mut destination) = open_at(
            libc::AT_FDCWD,
            &self.home,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        ) else {
            return failure("folderUnreadable");
        };
        let expected = directory_path(&destination).map(|home| join(&home, &components.join("/")));
        for (i, name) in components.iter().enumerate() {
            let parent = destination.as_raw_fd();
            let mut next = open_at(parent, name, DIR_FLAGS);
            if matches!(&next, Err(error) if errno(error) == libc::ENOENT) && i + 1 >= root_length {
                if self.root_length(&components) == 0 {
                    return failure("folderNotAllowed");
                }
                if let Err(error) = mkdir_at(parent, name, 0o755)
                    && errno(&error) != libc::EEXIST
                {
                    return failure("folderNotCreated");
                }
                next = open_at(parent, name, DIR_FLAGS);
            }
            match next {
                Ok(dir) => destination = dir,
                Err(_) => return failure(reason_for_missing_directory(parent, name)),
            }
        }

        let source_name = last_component(path);
        let source_dir = match path.trim_end_matches('/').rfind('/') {
            Some(0) => "/",
            Some(slash) => &path[..slash],
            None => ".",
        };
        let Ok(source) = open_at(
            libc::AT_FDCWD,
            source_dir,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        ) else {
            return failure("missing");
        };
        let Ok(entry) = lstat_at(source.as_raw_fd(), source_name) else {
            return failure("missing");
        };
        if !is_kind(entry.mode, libc::S_IFREG) {
            return failure("notAFile");
        }
        if entry.nlink != 1 {
            return failure("linked");
        }
        let (Ok(source_info), Ok(destination_info)) = (source.metadata(), destination.metadata())
        else {
            return failure("failed");
        };
        if source_info.dev() == destination_info.dev()
            && source_info.ino() == destination_info.ino()
        {
            return verify_in_place(source.as_raw_fd(), source_name, identity, path);
        }
        if source_info.dev() != destination_info.dev() {
            return failure("crossesVolumes");
        }

        let (src, dst) = (source.as_raw_fd(), destination.as_raw_fd());
        let Some(staging_name) = make_staging(src) else {
            return failure("failed");
        };
        let Some(staging) = open_own_staging(src, &staging_name) else {
            return failure("failed");
        };
        let stg = staging.as_raw_fd();
        let Some(staging_path) = directory_path(&staging) else {
            remove_dir_at(src, &staging_name);
            return failure("failed");
        };
        let mut journal = Map::new();
        journal.insert("source".into(), json!(path));
        journal.insert("staging".into(), json!(staging_path));
        journal.insert("name".into(), json!(source_name));
        journal.insert("sha256".into(), json!(identity.sha256));
        journal.insert("size".into(), json!(identity.size));
        journal.insert("folder".into(), json!(folder));
        self.journal(id, Some(&journal));
        let finish = || {
            remove_dir_at(src, &staging_name);
            self.journal(id, None);
        };
        let keep_staged = |reason: &str, name: &str| {
            let staged_path = directory_path(&staging).map(|dir| join(&dir, name));
            if let Some(staged_path) = staged_path.as_deref().filter(|_| name == source_name) {
                let _guard = self.persistence.lock().unwrap();
                let saved = {
                    let state = &mut *self.state.lock().unwrap();
                    if let Some(event) = state.records.get_mut(id) {
                        event["path"] = json!(staged_path);
                        event["filename"] = json!(last_component(staged_path));
                    }
                    Self::remember(state, id, path, staged_path)
                };
                self.defaults.set(MOVED_DEFAULT, saved);
            }
            self.journal(id, None);
            staged_failure(reason, staged_path)
        };
        let give_back = |reason: &str| {
            if rename_excl(stg, source_name, src, source_name).is_err() {
                return keep_staged(reason, source_name);
            }
            finish();
            failure(reason)
        };

        if let Err(error) = rename_excl(src, source_name, stg, source_name) {
            finish();
            return failure(if errno(&error) == libc::ENOENT {
                "missing"
            } else {
                "failed"
            });
        }
        #[cfg(test)]
        self.step("claimed", &staging);

        let Some((mut claimed, verified)) = open_regular(stg, source_name) else {
            return give_back("changed");
        };
        if verified.nlink() != 1 {
            return give_back("linked");
        }
        #[cfg(test)]
        self.step("hashing", &staging);
        if verified.size() as i64 != identity.size
            || sha256_of(&mut claimed).as_deref() != Some(identity.sha256.as_str())
        {
            return give_back("changed");
        }
        #[cfg(test)]
        self.step("verified", &staging);
        // The folder is held by descriptor, so a rename of it keeps the descriptor but leaves the approved root.
        // A removed folder is not a moved one: the rename then fails with ENOENT and the file goes back.
        let outside_root = || {
            expected.as_ref().is_some_and(|path| {
                destination.metadata().is_ok_and(|info| info.nlink() > 0)
                    && directory_path(&destination).is_some_and(|now| now != *path)
            })
        };
        if self.root_length(&components) == 0 || outside_root() {
            return give_back("folderNotAllowed");
        }

        let safe = safe_file_name(source_name);
        let directory =
            directory_path(&destination).unwrap_or_else(|| join(&self.home, &components.join("/")));
        let mut placed = None;
        for n in 0..=MAX_COLLISIONS {
            let candidate = if n == 0 {
                safe.clone()
            } else {
                name_with_suffix(&safe, &format!(" ({n})"))
            };
            journal.insert("placed".into(), json!(join(&directory, &candidate)));
            self.journal(id, Some(&journal));
            match rename_excl(stg, source_name, dst, &candidate) {
                Ok(()) => {
                    placed = Some(candidate);
                    break;
                }
                Err(error) => match errno(&error) {
                    libc::EEXIST => continue,
                    libc::ENOENT => {
                        if lstat_at(stg, source_name).is_ok_and(|file| {
                            file.dev == verified.dev() && file.ino == verified.ino()
                        }) {
                            return give_back("failed");
                        }
                        return keep_staged("changed", "");
                    }
                    libc::EXDEV => return give_back("crossesVolumes"),
                    _ => return give_back("failed"),
                },
            }
        }
        let Some(placed) = placed else {
            return give_back("tooManyCollisions");
        };
        #[cfg(test)]
        self.step("renamed", &staging);
        let final_path = join(&directory, &placed);
        #[cfg(test)]
        self.step("placed", &staging);

        let landed = lstat_at(dst, &placed).ok();
        let Some(landed) =
            landed.filter(|landed| landed.dev == verified.dev() && landed.ino == verified.ino())
        else {
            let staged = keep_staged("changed", "");
            let stray = format!("not-{source_name}");
            if landed.is_none() || rename_excl(dst, &placed, stg, &stray).is_ok() {
                return staged;
            }
            let mut with_stray = staged;
            with_stray["strayPath"] = json!(final_path);
            return with_stray;
        };
        if landed.nlink != 1 {
            if rename_excl(dst, &placed, stg, source_name).is_ok() {
                return give_back("linked");
            }
            self.journal(id, None);
            return staged_failure("linked", Some(final_path));
        }
        // A writer that kept the file open can still edit it after the hash; a final hash decides.
        let edited = claimed
            .metadata()
            .map_or(true, |info| info.size() as i64 != identity.size)
            || sha256_of(&mut claimed).as_deref() != Some(identity.sha256.as_str());
        if edited || outside_root() {
            let reason = if edited {
                "changed"
            } else {
                "folderNotAllowed"
            };
            if rename_excl(dst, &placed, stg, source_name).is_ok() {
                return give_back(reason);
            }
            self.journal(id, None);
            return staged_failure(reason, Some(final_path));
        }

        let event = {
            let _guard = self.persistence.lock().unwrap();
            let (saved, event) = {
                let state = &mut *self.state.lock().unwrap();
                let saved = Self::remember(state, id, path, &final_path);
                let event = state.records.get_mut(id).unwrap_or(&mut record);
                event["path"] = json!(final_path);
                event["filename"] = json!(placed);
                (saved, event.clone())
            };
            self.defaults.set(MOVED_DEFAULT, saved);
            event
        };
        finish();
        json!({"ok": true, "path": final_path, "event": event})
    }
}
