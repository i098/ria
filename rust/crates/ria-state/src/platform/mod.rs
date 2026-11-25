// Ria original, no Ardia source.

//! The one boundary between the state modules and the native code (plan section 6.5).
//!
//! `apply` never calls the platform. It pushes a [`PlatformCall`] on `Effects`, and `ria-app` runs each call
//! after `apply` returns and the `AppState` borrow is dropped. [`PlatformCall`] has one variant for each native call
//! of Ardia's `src/platform`, and [`FakePlatform`] is the fake that the replay drivers use.

mod calls;
mod fake;
pub mod kill_switches;

pub use calls::PlatformCall;
pub use fake::{Answer, FakePlatform};

use crate::kernel::AppAction;

/// Runs native calls.
pub trait Platform {
    /// Runs `call`. A call that returns a value ends with a result action, which `ria-app` puts in the action queue.
    fn run(&mut self, call: PlatformCall) -> Option<AppAction>;
}
