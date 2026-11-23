// Ported from ardia apps/browser/macos/Ardia-macOS/main.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

#[cfg(target_os = "macos")]
mod launch;

fn main() -> std::process::ExitCode {
    #[cfg(target_os = "macos")]
    {
        use ria_host_mac::launch::host_launch::LaunchEvent;
        launch::start(std::rc::Rc::new(|event| {
            if let LaunchEvent::Error(message) = event {
                eprintln!("[ria] {message}");
            }
        }))
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Ria requires macOS");
        std::process::ExitCode::FAILURE
    }
}
