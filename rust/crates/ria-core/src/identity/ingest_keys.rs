// Ported from ardia packages/core/src/ingestKeys.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Ingest keys of the three senders and the lookup of a key's source.

pub const APP_INGEST_KEY: &str = "ak_app_41b5d0762fd8ade6949ac2ac";
pub const CLI_INGEST_KEY: &str = "ak_cli_67adb8ebc49841125eda860e";
pub const SITE_INGEST_KEY: &str = "ak_site_6b26588b3764ae7d0f6d63d7";

pub const INGEST_KEY_HEADER: &str = "X-Ardia-Key";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestSource {
    App,
    Cli,
    Site,
}

impl IngestSource {
    /// The name of the source, as the TypeScript string union.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Cli => "cli",
            Self::Site => "site",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::App => APP_INGEST_KEY,
            Self::Cli => CLI_INGEST_KEY,
            Self::Site => SITE_INGEST_KEY,
        }
    }
}

/// Port of `ingestSourceOf` (`ingestKeys.ts:15`).
pub fn ingest_source_of(key: Option<&str>) -> Option<IngestSource> {
    [IngestSource::App, IngestSource::Cli, IngestSource::Site]
        .into_iter()
        .find(|source| Some(source.key()) == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_of_known_and_unknown_keys() {
        assert_eq!(
            ingest_source_of(Some(CLI_INGEST_KEY)),
            Some(IngestSource::Cli)
        );
        assert_eq!(ingest_source_of(Some("ak_app_x")), None);
        assert_eq!(ingest_source_of(None), None);
    }
}
