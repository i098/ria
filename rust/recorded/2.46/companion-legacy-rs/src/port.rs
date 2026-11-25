// Ria original, no Ardia source.

//! Calls `ria_sync::logic::companion::legacy` for the cases of `companion/legacy.ts`.

#![forbid(unsafe_code)]

/// Port of `fromLegacyUrl` (`packages/sync/src/companion/legacy.ts:7`).
pub fn from_legacy_url(url: String) -> Result<String, String> {
    Ok(ria_sync::logic::companion::legacy::from_legacy_url(&url))
}
