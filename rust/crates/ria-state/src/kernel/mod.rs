// Ria original, no Ardia source.

//! The store kernel (plan section 6.5): `AppState` with `get` and `get_mut`, `Effects`, `AppAction`, and the change counters.
//!
//! The process has one `AppState` on the main thread, with one private field for each state module. An action reads a module
//! with `app.get::<M>()` and changes it with `app.get_mut::<M>()`, where `M` is the `State` type of the module
//! (for example `tabs::State`). `get_mut` adds 1 to the change counter of the module, so no code adds to a counter by hand.
//! A `get_mut` with no change gives one frame more than necessary and no wrong frame.

mod modules;
mod number;

pub use modules::{AppAction, AppState};
pub use number::Number;

use crate::platform::PlatformCall;

/// The `State` type of one state module. `ria-state` implements it for each of the 41 modules, in `modules.rs`.
pub trait Module: Default + Sized {
    /// The module state of `app`.
    fn slot(app: &AppState) -> &Self;
    /// The module state of `app`, for change. Use `AppState::get_mut`, which also counts the change.
    fn slot_mut(app: &mut AppState) -> &mut Self;
    /// The change counter of the module.
    fn changes(app: &AppState) -> u64;
    /// Adds 1 to the change counter of the module.
    fn count_change(app: &mut AppState);
}

impl AppState {
    /// Reads the state of module `M`.
    pub fn get<M: Module>(&self) -> &M {
        M::slot(self)
    }

    /// Changes the state of module `M` and adds 1 to its change counter.
    pub fn get_mut<M: Module>(&mut self) -> &mut M {
        M::count_change(self);
        M::slot_mut(self)
    }

    /// How many times `get_mut` gave out module `M`.
    pub fn changes<M: Module>(&self) -> u64 {
        M::changes(self)
    }
}

/// The platform calls that one action asks for, in order. `apply` only adds to it; it never calls the platform itself.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Effects {
    calls: Vec<PlatformCall>,
}

impl Effects {
    /// Asks for a platform call.
    pub fn push(&mut self, call: PlatformCall) {
        self.calls.push(call);
    }

    /// The calls in the order of the requests.
    pub fn calls(&self) -> &[PlatformCall] {
        &self.calls
    }

    /// Takes the calls out, for `ria-app` to run after the `AppState` borrow is dropped.
    pub fn take(&mut self) -> Vec<PlatformCall> {
        std::mem::take(&mut self.calls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{find, focus};

    #[test]
    fn get_mut_adds_one_to_the_counter_of_its_module_only() {
        let mut app = AppState::default();
        assert_eq!(app.changes::<find::State>(), 0);
        app.get_mut::<find::State>();
        app.get_mut::<find::State>();
        assert_eq!(app.changes::<find::State>(), 2);
        assert_eq!(app.changes::<focus::State>(), 0);
    }

    #[test]
    fn get_does_not_count() {
        let app = AppState::default();
        let _ = app.get::<find::State>();
        assert_eq!(app.changes::<find::State>(), 0);
    }

    #[test]
    fn effects_start_empty_and_take_leaves_them_empty() {
        let mut fx = Effects::default();
        assert!(fx.calls().is_empty());
        assert!(fx.take().is_empty());
    }
}
