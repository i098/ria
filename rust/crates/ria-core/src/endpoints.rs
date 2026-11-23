// Ria original, no Ardia source.

//! The 5 `ardia.dev` addresses of Ardia, all turned off. Each sender reads its
//! address here and makes no request when the address is empty (plan section 6.10).

// Ardia: apps/browser/src/state/telemetry/config.ts:1
pub const TELEMETRY_EXPORT: &str = "";
// Ardia: apps/browser/macos/Ardia-macOS/Info.plist:182
pub const UPDATE_FEED: &str = "";
// Ardia: apps/browser/src/state/report/actions.ts:33
pub const FEEDBACK_API: &str = "";
// Ardia: apps/browser/src/platform/killSwitches.ts:103
pub const SWITCHES: &str = "";
// Ardia: apps/browser/macos/Ardia-macOS/Info.plist:10
pub const RELEASE_NOTES: &str = "";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_endpoint_is_off() {
        for address in [
            TELEMETRY_EXPORT,
            UPDATE_FEED,
            FEEDBACK_API,
            SWITCHES,
            RELEASE_NOTES,
        ] {
            assert!(address.is_empty());
        }
    }
}
