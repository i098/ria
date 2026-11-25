// Ria original, no Ardia source.

use std::ffi::{CString, c_char, c_int};
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

const MARKER: &str = ".ria-profile";

pub(super) fn claim(directory: &Path) -> io::Result<File> {
    fs::create_dir_all(directory)?;
    let admitted = open_directory(directory)?;
    if has_marker(&admitted) {
        return Ok(admitted);
    }
    if !entries(&admitted)?.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "non-empty data directory has no Ria profile marker",
        ));
    }
    create_child(&admitted, MARKER.as_ref())?;
    ensure_identity(directory, &admitted)?;
    Ok(admitted)
}

fn create_child(directory: &File, name: &std::ffi::OsStr) -> io::Result<()> {
    let name = CString::new(name.as_encoded_bytes())?;
    let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags, 0o644) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    drop(unsafe { File::from_raw_fd(fd) });
    Ok(())
}

/// Names and directory flags of the entries, listed through the open directory and not through its path.
fn entries(directory: &File) -> io::Result<Vec<(std::ffi::OsString, bool)>> {
    let fd = unsafe { libc::dup(directory.as_raw_fd()) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        let error = io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(error);
    }
    unsafe { libc::rewinddir(stream) };
    let mut found = Vec::new();
    let result = loop {
        unsafe { *libc::__error() = 0 };
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            let errno = io::Error::last_os_error();
            break if errno.raw_os_error() == Some(0) {
                Ok(())
            } else {
                Err(errno)
            };
        }
        let entry = unsafe { &*entry };
        let len = entry.d_namlen as usize;
        let name: Vec<u8> = entry.d_name[..len].iter().map(|&c| c as u8).collect();
        if name == b"." || name == b".." {
            continue;
        }
        let is_dir = match entry.d_type {
            libc::DT_DIR => true,
            libc::DT_UNKNOWN => {
                let name = std::ffi::OsString::from_vec(name.clone());
                match child_kind(directory, &name) {
                    Ok(kind) => kind == libc::S_IFDIR,
                    Err(error) if error.kind() == ErrorKind::NotFound => continue,
                    Err(error) => break Err(error),
                }
            }
            _ => false,
        };
        found.push((std::ffi::OsString::from_vec(name), is_dir));
    };
    unsafe { libc::closedir(stream) };
    result.map(|()| found)
}

/// File type bits of a child, without following a link.
fn child_kind(directory: &File, name: &std::ffi::OsStr) -> io::Result<libc::mode_t> {
    let name = CString::new(name.as_encoded_bytes())?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let status = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(stat.st_mode & libc::S_IFMT)
}

/// The real path of an open directory, as the system reports it now.
fn directory_path(directory: &File) -> io::Result<PathBuf> {
    let mut buffer = [0u8; libc::PATH_MAX as usize];
    if unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let len = buffer
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(buffer.len());
    Ok(PathBuf::from(std::ffi::OsString::from_vec(
        buffer[..len].to_vec(),
    )))
}

/// A path that names the directory by device and inode, so a rename or swap of any parent cannot redirect it.
pub(super) fn identity_path(directory: &File) -> io::Result<PathBuf> {
    let metadata = directory.metadata()?;
    Ok(PathBuf::from(format!(
        "/.vol/{}/{}",
        metadata.dev(),
        metadata.ino()
    )))
}

pub(super) fn open_directory(path: &Path) -> io::Result<File> {
    let directory = File::open(path)?;
    ensure_identity(path, &directory)?;
    Ok(directory)
}

pub(super) fn ensure_identity(path: &Path, directory: &File) -> io::Result<()> {
    let named = path.symlink_metadata()?;
    let opened = directory.metadata()?;
    if !named.is_dir() || (named.dev(), named.ino()) != (opened.dev(), opened.ino()) {
        return Err(ErrorKind::InvalidInput.into());
    }
    Ok(())
}

pub(super) fn has_marker(directory: &File) -> bool {
    open_child(directory, MARKER.as_ref())
        .and_then(|file| file.metadata())
        .is_ok_and(|metadata| metadata.is_file())
}

pub(super) fn read(root: &Path, directory: &File, name: &Path) -> io::Result<Vec<u8>> {
    ensure_identity(root, directory)?;
    let mut parent = None;
    let mut parts = name.components().peekable();
    while let Some(Component::Normal(part)) = parts.next() {
        let mut file = open_child(parent.as_ref().unwrap_or(directory), part)?;
        let metadata = file.metadata()?;
        if parts.peek().is_none() && metadata.is_file() {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            ensure_identity(root, directory)?;
            return Ok(bytes);
        }
        if !metadata.is_dir() {
            break;
        }
        parent = Some(file);
    }
    Err(ErrorKind::InvalidInput.into())
}

unsafe extern "C" {
    fn openat(directory: c_int, path: *const c_char, flags: c_int, ...) -> c_int;
}

pub(super) fn open_child(directory: &File, name: &std::ffi::OsStr) -> io::Result<File> {
    #[cfg(target_os = "macos")]
    const FLAGS: c_int = 0x0100_0000 | 0x0100 | 0x0004;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    const FLAGS: c_int = 0x0008_0000 | 0x0002_0000 | 0x0800;

    let name = CString::new(name.as_encoded_bytes())?;
    let fd = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), FLAGS) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(super) fn directory_exists(path: &Path) -> io::Result<bool> {
    match path.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
        Ok(_) => Err(ErrorKind::NotADirectory.into()),
    }
}

/// Requires the engine's singleton lock to sit in the admitted `Chromium` directory and name this process.
/// Use the admitted handle because Chromium writes the lock in the directory it opened.
pub(super) fn check_singleton(data: &File) -> io::Result<()> {
    let root = open_child(data, "Chromium".as_ref())?;
    let name = CString::new("SingletonLock")?;
    let mut buffer = vec![0u8; libc::PATH_MAX as usize];
    let len = unsafe {
        libc::readlinkat(
            root.as_raw_fd(),
            name.as_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
        )
    };
    if len < 0 {
        let error = io::Error::last_os_error();
        return Err(if error.kind() == ErrorKind::NotFound {
            ErrorKind::InvalidInput.into()
        } else {
            error
        });
    }
    buffer.truncate(len as usize);
    let target = std::ffi::OsString::from_vec(buffer);
    let pid = unsafe { libc::getpid() };
    if !target.to_string_lossy().ends_with(&format!("-{pid}")) {
        return Err(ErrorKind::InvalidInput.into());
    }
    Ok(())
}

/// Checks the links under the `Chromium` directory of the admitted data directory, reached through its handle.
pub(super) fn check_tree(data: &File, root_links: &[&str]) -> io::Result<()> {
    let root = match open_child(data, "Chromium".as_ref()) {
        Ok(root) => root,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !root.metadata()?.is_dir() {
        return Err(ErrorKind::NotADirectory.into());
    }
    let root_path = directory_path(&root)?;
    check_entries(&root_path, &root, root_links)
}

fn check_entries(root: &Path, directory: &File, allowed_links: &[&str]) -> io::Result<()> {
    for (name, is_dir) in entries(directory)? {
        if is_dir {
            let Some(child) = if_present(open_child(directory, &name))? else {
                continue;
            };
            check_entries(root, &child, &[])?;
        } else if !allowed_links.iter().any(|&link| name == link) {
            match child_kind(directory, &name) {
                Ok(libc::S_IFLNK) => check_link(root, directory, &name)?,
                Ok(libc::S_IFDIR) => {
                    if let Some(child) = if_present(open_child(directory, &name))? {
                        check_entries(root, &child, &[])?;
                    }
                }
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(())
}

fn check_link(root: &Path, directory: &File, name: &std::ffi::OsStr) -> io::Result<()> {
    let c_name = CString::new(name.as_encoded_bytes())?;
    let mut buffer = vec![0u8; libc::PATH_MAX as usize];
    let len = unsafe {
        libc::readlinkat(
            directory.as_raw_fd(),
            c_name.as_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
        )
    };
    if len < 0 {
        let error = io::Error::last_os_error();
        return if error.kind() == ErrorKind::NotFound {
            Ok(())
        } else {
            Err(error)
        };
    }
    buffer.truncate(len as usize);
    let target = PathBuf::from(std::ffi::OsString::from_vec(buffer));
    match directory_path(directory)?.join(target).canonicalize() {
        Ok(target) if target.starts_with(root) => Ok(()),
        Ok(_) => Err(io::Error::new(
            ErrorKind::InvalidInput,
            "a link points outside the Chromium directory",
        )),
        Err(error)
            if error.kind() == ErrorKind::NotFound
                && if_present(child_kind(directory, name))?.is_none() =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn if_present<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        check_entries, check_link, check_singleton, directory_path, entries, identity_path,
        open_child,
    };
    use std::{fs, fs::File, os::unix::fs::symlink};

    #[test]
    fn singleton_lock_proves_the_opened_directory_after_a_swap() {
        let base = std::env::temp_dir().join(format!("ria-singleton-{}", std::process::id()));
        let admitted = base.join("admitted");
        let moved = base.join("moved");
        fs::create_dir_all(admitted.join("Chromium")).unwrap();
        let handle = File::open(&admitted).unwrap();
        let pid = unsafe { libc::getpid() };
        symlink(
            format!("host-{pid}"),
            admitted.join("Chromium/SingletonLock"),
        )
        .unwrap();
        check_singleton(&handle).unwrap();
        fs::rename(&admitted, &moved).unwrap();
        fs::create_dir_all(admitted.join("Chromium")).unwrap();
        symlink(
            format!("host-{pid}"),
            admitted.join("Chromium/SingletonLock"),
        )
        .unwrap();
        fs::remove_file(moved.join("Chromium/SingletonLock")).unwrap();
        symlink(
            format!("host-{}", pid + 1),
            moved.join("Chromium/SingletonLock"),
        )
        .unwrap();
        // The engine opened the swapped-in path under this process's pid; the admitted inode does not hold it.
        assert!(check_singleton(&handle).is_err());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn ignores_a_chromium_directory_or_link_removed_after_listing() {
        let root = std::env::temp_dir().join(format!("ria-vanished-cache-{}", std::process::id()));
        let directory = root.join("cache");
        let link = root.join("link");
        fs::create_dir_all(&directory).unwrap();
        symlink(&directory, &link).unwrap();
        let handle = File::open(&root).unwrap();
        let root = directory_path(&handle).unwrap();
        let listed = entries(&handle).unwrap();
        assert!(listed.contains(&("cache".into(), true)));
        assert!(listed.contains(&("link".into(), false)));
        fs::remove_dir(&directory).unwrap();
        assert!(check_link(&root, &handle, "link".as_ref()).is_err());
        fs::remove_file(&link).unwrap();
        check_entries(&root, &handle, &[]).unwrap();
        check_link(&root, &handle, "link".as_ref()).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn scan_and_handoff_follow_the_opened_directory_after_a_swap() {
        let base = std::env::temp_dir().join(format!("ria-swap-scan-{}", std::process::id()));
        let admitted = base.join("admitted");
        let moved = base.join("moved");
        fs::create_dir_all(admitted.join("Chromium")).unwrap();
        let handle = File::open(&admitted).unwrap();
        let chromium = open_child(&handle, "Chromium".as_ref()).unwrap();
        let handoff = identity_path(&handle).unwrap();
        fs::rename(&admitted, &moved).unwrap();
        fs::create_dir_all(admitted.join("Chromium")).unwrap();
        symlink("/etc", admitted.join("Chromium/escape")).unwrap();
        // The swapped-in path has an escaping link; the admitted directory has none.
        super::check_tree(&handle, &[]).unwrap();
        drop(chromium);
        assert!(
            entries(&handle)
                .unwrap()
                .contains(&("Chromium".into(), true))
        );
        assert!(handoff.join("Chromium").is_dir());
        assert!(!handoff.join("Chromium/escape").exists());
        fs::remove_dir_all(base).unwrap();
    }
}
