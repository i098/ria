// Ria original, no Ardia source.

//! Omnibox logic of `packages/core`: address bar input, suggestions, the calculator and quick-create shortcuts.

pub mod calculator;
pub mod candidate_pool;
#[expect(clippy::module_inception, reason = "mirrors omnibox.ts")]
pub mod omnibox;
pub mod open_tabs;
pub mod quick_create;
pub mod suggest;
pub mod suggest_fetcher;
#[cfg(test)]
mod suggest_reference;
pub mod suggestion;
pub mod suggestion_rows;
