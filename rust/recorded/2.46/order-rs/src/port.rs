// Ria original, no Ardia source.

//! Calls `ria_sync::logic::order` for the recorded cases of `packages/sync/src/order.ts`.

#![forbid(unsafe_code)]

use crate::js::{string_units, utf16_string};
use serde_json::Value;

/// Port of `assignPositions` (`packages/sync/src/order.ts:19`). A value that is not a string
/// reads as `undefined`.
pub fn assign_positions(existing: Vec<Value>) -> Result<Vec<Value>, String> {
    let units: Vec<Option<Vec<u16>>> = existing.iter().map(string_units).collect();
    let out = ria_sync::logic::order::assign_positions_utf16(&units)?;
    Ok(out.iter().map(|p| utf16_string(p)).collect())
}
