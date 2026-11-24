// Ria original, no Ardia source.

//! The one boundary between the state modules and the native code (plan section 6.5).
//!
//! `apply` never calls the platform. It pushes a [`PlatformCall`] on `Effects`, and `ria-app` runs each call
//! after `apply` returns and the `AppState` borrow is dropped. Slice 2.3 adds the variants of [`PlatformCall`]
//! (one for each native call) and the fake `Platform` that the replay drivers use.

use crate::kernel::AppAction;

/// One native call that an action asks for. Slice 2.3 adds one variant for each native call.
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformCall {}

/// Runs native calls.
pub trait Platform {
    /// Runs `call`. A call that returns a value ends with a result action, which `ria-app` puts in the action queue.
    fn run(&mut self, call: PlatformCall) -> Option<AppAction>;
}
