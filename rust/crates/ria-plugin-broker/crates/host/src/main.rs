// Source: packages/plugin-broker/crates/host/src/main.rs in Ardia at commit 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// Rust port with changes: header added and moved into the Ria workspace.
fn main() {
    ardia_plugin_host::main_with(&mut ardia_plugin_host::Unconfined)
}
