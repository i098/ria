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

pub(super) fn check_subdirectory(directory: &Path) -> io::Result<()> {
    match directory.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(ErrorKind::NotADirectory.into()),
    }
}

fn is_regular_file(path: &Path) -> bool {
    path.symlink_metadata().is_ok_and(|metadata| metadata.is_file())
}
