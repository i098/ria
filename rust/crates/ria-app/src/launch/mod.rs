// Ported from ardia apps/browser/macos/Ardia-macOS/AppDelegate.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::process::ExitCode;

use ria_host_mac::launch::host_launch::{self, Observer};

pub fn start(observer: Observer) -> ExitCode {
    match host_launch::run(observer) {
        Ok(code) => u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from),
        Err(error) => {
            eprintln!("[ria] {error}");
            ExitCode::FAILURE
        }
    }
}
