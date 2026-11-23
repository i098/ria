// Ported from ardia packages/plugin-broker/crates/host/src/main.rs at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
fn main() {
    ardia_plugin_host::main_with(&mut ardia_plugin_host::Unconfined)
}
