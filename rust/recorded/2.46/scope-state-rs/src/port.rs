// Ria original, no Ardia source.

//! Calls `ria_sync::logic::scope_state` for the recorded cases of `packages/sync/src/scopeState.ts`.

#![forbid(unsafe_code)]

/// Port of `emptyScope` (`packages/sync/src/scopeState.ts:41`).
pub fn empty_scope() -> Result<serde_json::Value, String> {
    serde_json::to_value(ria_sync::logic::scope_state::empty_scope()).map_err(|e| e.to_string())
}
