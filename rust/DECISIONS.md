# Halyard decisions

Each decision is an ADR. Status of all: accepted on the captain's approval of the port plan.
Source: the port plan, section 9 (choices C1 to C10), and the captain's hard rule on animations.
Ardia means the original at `ardia/`, frozen at commit `551d3c13c`.

## ADR-C1: Rune is `anomalyco/rune`

- Context: two projects use the name "Rune". `anomalyco/rune` is a UI framework. `rune-rs/rune` is a script language.
- Decision: use `anomalyco/rune`, at one pinned revision.
- Consequences: only this one has an embed host (`embed::LayerHost`) for a native layer. Every UI crate depends on it through the pin. A change of revision is a reviewed change.

## ADR-C2: The port lives in the private repository `i098/halyard`

- Context: rust-anything puts the port beside the original. A new repository keeps the original untouched.
- Decision: Ardia is a git submodule at `ardia/`, pinned at `551d3c13c`. The port's code is in `rust/`.
- Consequences: `record` and `diff` run the original from `ardia/`. We never push, open an issue or comment on `ardia-browser/ardia`. Each change to Halyard starts as an issue in `i098/halyard`, and its PR body has `Closes #N`.

## ADR-C3: First scope is the Mac desktop app only

- Context: Rune's embed host is Apple only. The Linux and Windows hosts are different designs.
- Decision: port the Mac desktop app. Linux, Windows, phone, site, dashboard and videos are out of scope.
- Consequences: `rust/MAP.md` lists only the Mac scope.

## ADR-C4: One process, Chromium owns the loop, Rune draws into a layer

- Context: the other option is two processes, with page surfaces that cross a process boundary.
- Decision: the Rust binary is the app's main executable. It runs the engine, as `main.swift` does today. Rune draws into a `CAMetalLayer` on an `NSView`. Pages are sibling engine views.
- Consequences: no new focus, input or latency problem from a process boundary. The host layer owns all unsafe code. Engine callbacks must never unwind into C++.

## ADR-C5: Rust calls the existing C and Objective-C engine interfaces with `objc2`

- Context: a new C interface to ArdiaCore needs an engine patch and a full Chromium build.
- Decision: call `ac_engine_call` and `ac_engine_set_event_sink` (C) and the `ArdiaCore.h` classes (Objective-C) with `objc2`. Use the prebuilt engine that `prebuilt-engines.tsv` pins.
- Consequences: no engine fork and no Chromium build. The engine stays non-Rust and unchanged.

## ADR-C6: Logic first with recorded proof, then UI

- Context: the other option is a new Rune app from zero with a parity check at the end. That gives no proof until the end.
- Decision: follow the rust-anything method. Record the original, port one unit at a time, and call a unit done only when `gate` passes. Port the UI after the logic.
- Consequences: each unit has recorded cases as proof. `rust/MAP.md` tracks each unit.

## ADR-C7: The original is frozen at `551d3c13c`

- Context: the Mac scope grew by about 170,000 lines in 7 days. Recordings go stale at each change.
- Decision: freeze Ardia at `551d3c13c`. Sync again at the end of each phase.
- Consequences: the submodule pin changes only in a sync change. A sync re-records the affected units.

## ADR-C8: Our own version, named Halyard

- Context: Apache-2.0 section 6 gives no right to the Ardia name or logo.
- Decision: the working name is Halyard, with its own icon. `NOTICE` credits Ardia. The Ardia name and icon are never the brand.
- Consequences: user-visible strings, bundle identifiers and icons use the Halyard brand. Internal crate names may keep the `ardia-` prefix from the plan while the port is in progress. [INFERENCE: a rename of the crates is a later decision.]

## ADR-C9: The port's own files are Apache-2.0

- Context: the other option is GPL-3.0-only for all files. Ardia's own source files are Apache-2.0.
- Decision: Halyard's own files are Apache-2.0 (`LICENSE`).
- Consequences: the shipped app is GPL-3.0 in both cases, because the engine uses Helium's GPL-3.0 patches (`ardia/NOTICE`).

## ADR-C10: Swift native modules are ported to Rust with `objc2`

- Context: the other option keeps the Swift modules behind a C interface. That leaves two languages and a second build.
- Decision: port the Swift modules to Rust with `objc2`. Delete what Rune already gives; do not port it.
- Consequences: `rust/MAP.md` marks each Swift and Objective-C++ unit as port or delete.

## ADR-A1: Every animation matches the original

- Context: the captain's hard rule: "get all the animations right".
- Decision: each animation is recorded from Ardia and from Halyard. Parity is proven with numbers: duration, curve, frame times and end state. A look-alike is not proof.
- Consequences: no UI surface is done until its animations have a recorded pair and a passing numeric comparison. The parity rig (`rust/parity/`) and the animation list (`rust/ANIMATIONS.md`) hold the method and the cases.
