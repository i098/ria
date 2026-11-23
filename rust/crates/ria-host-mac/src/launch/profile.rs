// Ria original, no Ardia source.

use std::ffi::{CString, c_char, c_int};
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

const MARKER: &str = ".ria-profile";

pub(super) fn claim(directory: &Path) -> io::Result<File> {
    let marker = directory.join(MARKER);
    fs::create_dir_all(directory)?;
    let admitted = open_directory(directory)?;
    if has_marker(&admitted) {
        return Ok(admitted);
    }
    if fs::read_dir(directory)?.next().transpose()?.is_some() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "non-empty data directory has no Ria profile marker",
        ));
    }
    File::create_new(marker)?;
    ensure_identity(directory, &admitted)?;
    Ok(admitted)
}

pub(super) fn open_directory(path: &Path) -> io::Result<File> {
    let directory = File::open(path)?;
    ensure_identity(path, &directory)?;
    Ok(directory)
}

fn ensure_identity(path: &Path, directory: &File) -> io::Result<()> {
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

pub(super) fn check_tree(root: &Path, root_links: &[&str]) -> io::Result<()> {
    if directory_exists(root)? {
        check_entries(root, root, root_links)?;
    }
    Ok(())
}

fn check_entries(root: &Path, directory: &Path, allowed_links: &[&str]) -> io::Result<()> {
    let Some(entries) = if_present(fs::read_dir(directory))? else {
        return Ok(());
    };
    for entry in entries {
        let Some(entry) = if_present(entry)? else {
            continue;
        };
        let Some(kind) = if_present(entry.file_type())? else {
            continue;
        };
        if kind.is_dir() {
            check_entries(root, &entry.path(), &[])?;
        } else if kind.is_symlink() && !allowed_links.iter().any(|&link| entry.file_name() == link)
        {
            check_link(root, &entry.path())?;
        }
    }
    Ok(())
}

fn check_link(root: &Path, link: &Path) -> io::Result<()> {
    match link.canonicalize() {
        Ok(target) if target.starts_with(root) => Ok(()),
        Ok(_) => Err(io::Error::new(
            ErrorKind::InvalidInput,
            "a link points outside the Chromium directory",
        )),
        Err(error)
            if error.kind() == ErrorKind::NotFound
                && if_present(link.symlink_metadata())?.is_none() =>
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
    use super::{check_entries, check_link};
    use std::{fs, os::unix::fs::symlink};

    #[test]
    fn ignores_a_chromium_directory_or_link_removed_after_listing() {
        let root = std::env::temp_dir().join(format!("ria-vanished-cache-{}", std::process::id()));
        let directory = root.join("cache");
        let link = root.join("link");
        fs::create_dir_all(&directory).unwrap();
        symlink(&directory, &link).unwrap();
        let entries: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert!(entries.contains(&directory));
        assert!(entries.contains(&link));
        fs::remove_dir(&directory).unwrap();
        assert!(check_link(&root, &link).is_err());
        fs::remove_file(&link).unwrap();
        check_entries(&root, &directory, &[]).unwrap();
        check_link(&root, &link).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
