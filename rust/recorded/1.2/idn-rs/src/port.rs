// Ported from ardia packages/core/src/idn.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `url::idn`. The logic is in `ria_core::url::idn`.

#![forbid(unsafe_code)]

use ria_core::url::idn;

/// Port of `punycodeEncode` (`packages/core/src/idn.ts:54`).
pub fn punycode_encode(input: String) -> Result<String, String> {
    Ok(idn::punycode_encode(&input))
}

/// Port of `displayHost` (`packages/core/src/idn.ts:319`).
pub fn display_host(host: String) -> Result<String, String> {
    Ok(idn::display_host(&host))
}

/// Port of `hostSpoofReason` (`packages/core/src/idn.ts:339`).
pub fn host_spoof_reason(host: String) -> Result<Option<String>, String> {
    Ok(idn::host_spoof_reason(&host).map(|r| r.as_str().to_string()))
}
