// Ria original, no Ardia source.

//! Rune layers and native page views in a macOS window.

#![cfg(target_os = "macos")]

mod input;
mod view;
mod window;

pub use input::TextInputSnapshot;
pub use window::{WindowHost, WindowSpec};
