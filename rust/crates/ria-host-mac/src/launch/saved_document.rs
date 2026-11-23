// Ported from ardia packages/ardiacore/ios/ArdiaSavedDocument.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::ops::RangeInclusive;
use std::path::Path;

use serde_json::{Map, Value};

use super::profile;

/// A JSON object with a version in the range accepted by its reader.
pub struct SavedDocument {
    pub version: i32,
    pub fields: Map<String, Value>,
}

pub fn read(directory: &Path, name: &Path, versions: RangeInclusive<i32>) -> Option<SavedDocument> {
    parse(&profile::read(directory, name)?, versions)
}

pub fn parse(data: &[u8], versions: RangeInclusive<i32>) -> Option<SavedDocument> {
    let fields: Map<String, Value> = serde_json::from_slice(data).ok()?;
    let version = match fields.get("version") {
        None if *versions.start() == 0 => 0,
        Some(value) => {
            let value = value.as_f64()?;
            if value.fract() != 0.0
                || value < f64::from(*versions.start())
                || value > f64::from(*versions.end())
            {
                return None;
            }
            value as i32
        }
        None => return None,
    };
    Some(SavedDocument { version, fields })
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn accepts_only_supported_integral_document_versions() {
        assert!(parse(br#"{"version": 5.0}"#, 5..=5).is_some());
        assert!(parse(br#"{}"#, 0..=11).is_some());
        assert!(parse(br#"{}"#, 5..=5).is_none());
        for data in [
            "[]",
            "{",
            r#"{"version":11.5}"#,
            r#"{"version":12}"#,
            r#"{"version":-1}"#,
            r#"{"version":true}"#,
            r#"{"version":null}"#,
            r#"{"version":"5"}"#,
        ] {
            assert!(parse(data.as_bytes(), 0..=11).is_none(), "{data}");
        }
    }
}
