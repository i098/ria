// Ria original, no Ardia source.

//! Mac host: everything that talks to AppKit and to the `ArdiaCore.h` classes of the engine.
//! The crate is empty on other systems, so the workspace builds everywhere.

#![cfg(target_os = "macos")]
#![allow(unsafe_code)]

pub mod core;
pub mod diagnostics;
pub mod launch;
pub mod pip;
pub mod swipe;
pub mod tabs;
pub mod windows;
