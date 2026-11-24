# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Ria has no release yet.

## [Unreleased]

### Added

- [Native macOS launch](rust/crates/ria-app/src/launch/README.md).
- Browser domain routing in the Rust engine, including unpacked extension inspection and shared extension management calls for the Mac host.
- Phase 0 proofs: Rust host on the Ardia engine with Rune layers, port loop on `fuzzy.ts`, parity rig, unit and animation maps, decision records.
- [State replay kit](rust/replay/kit/README.md) with a find sample.
- Phase 1 core logic: Rust ports of the Phase 1 units, checked against Ardia. All checks passed for [gate G1](rust/GATES.md#g1-record).
- Open-source community files, issue forms and PR template.
