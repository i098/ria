// Ria original, no Ardia source.

//! Calls `ria_sync::logic::canon` for the recorded cases of `packages/sync/src/canon.ts`.

#![forbid(unsafe_code)]

/// Port of `canonical` (`packages/sync/src/canon.ts:1`).
pub fn canonical(value: serde_json::Value) -> Result<String, String> {
    Ok(ria_sync::logic::canon::canonical(&value))
}
