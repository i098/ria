// Ria original, no Ardia source.

use std::fs::{self, File};
use std::io::{self, ErrorKind};
use std::path::{Component, Path};

const MARKER: &str = ".ria-profile";

pub(super) fn claim(directory: &Path) -> io::Result<()> {
    let marker = directory.join(MARKER);
    if is_regular_file(&marker) {
        return Ok(());
    }
    fs::create_dir_all(directory)?;
    if fs::read_dir(directory)?.next().transpose()?.is_some() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "non-empty data directory has no Ria profile marker",
        ));
    }
    File::create_new(marker)?;
    Ok(())
}

pub(super) fn read(directory: &Path, name: &Path) -> Option<Vec<u8>> {
    let mut components = name.components();
    let Some(Component::Normal(name)) = components.next() else {
        return None;
    };
    if components.next().is_some() || directory.as_os_str().is_empty() {
        return None;
    }
    let directory = directory.canonicalize().ok()?;
    let file = directory.join(name);
    if !is_regular_file(&directory.join(MARKER)) || !is_regular_file(&file) {
        return None;
    }
    fs::read(file).ok()
}

pub(super) fn read_config_file(root: &Path, name: &Path) -> io::Result<Vec<u8>> {
    if root.canonicalize()? != root {
        return Err(ErrorKind::InvalidInput.into());
    }
    let mut path = root.to_owned();
    let mut parts = name.components().peekable();
    while let Some(Component::Normal(part)) = parts.next() {
        path.push(part);
        if parts.peek().is_none() {
            if !path.symlink_metadata()?.is_file() {
                return Err(ErrorKind::InvalidInput.into());
            }
            return fs::read(path);
        }
        directory_exists(&path)?;
    }
    Err(ErrorKind::InvalidInput.into())
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
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            check_entries(root, &entry.path(), &[])?;
        } else if kind.is_symlink() {
            let name = entry.file_name();
            if allowed_links.iter().any(|&link| name == link)
                || entry.path().canonicalize()?.starts_with(root)
            {
                continue;
            }
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "a link points outside the Chromium directory",
            ));
        }
    }
    Ok(())
}

fn is_regular_file(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|metadata| metadata.is_file())
}
