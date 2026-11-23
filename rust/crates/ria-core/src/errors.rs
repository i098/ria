// Ported from ardia packages/core/src/errors.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `ArdiaError` and `ardia_error`.

use std::error::Error;
use std::fmt;

/// An error with a stable machine code, as `ArdiaError` in the TypeScript source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArdiaError {
    pub code: String,
    pub message: String,
}

impl ArdiaError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ArdiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ArdiaError {}

/// Port of `ardiaError` (`errors.ts:10`): an `ArdiaError` passes through; any other error
/// becomes `ERR_UNKNOWN` with its message.
pub fn ardia_error(error: &(dyn Error + 'static)) -> ArdiaError {
    match error.downcast_ref::<ArdiaError>() {
        Some(e) => e.clone(),
        None => ArdiaError::new("ERR_UNKNOWN", error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ardia_error_passes_through_and_wraps() {
        let own = ArdiaError::new("ERR_X", "boom");
        assert_eq!(ardia_error(&own), own);
        let other = fmt::Error;
        assert_eq!(
            ardia_error(&other),
            ArdiaError::new("ERR_UNKNOWN", other.to_string())
        );
    }
}
