# Resource numbers

Plan v5 section 6.7. Load case: 3 windows, 20 tabs in each on the fixture page (`rust/proofs/abc-host/src/page.html`),
the sidebar open, 1 tab that plays the fixture video (`page-video.html`, `fixture.mp4`).
Limit: no number of the port is worse than the number of Ardia at `551d3c13c` by more than 10 %.
Ardia numbers: not measured in slices 0.7 and 0.8 (open for the owner of the RES check).

## Proof crate (slices 0.7 and 0.8)

Mac arm64, macOS 26.2, 24 GB, Chromium 158.0.8127.36 from the prebuilt engine, debug info on, built with `cargo build --release` (before the blaze patch; not repeated with blaze).
Command: `ENGINE_DIR=... OUT_DIR=... ./run.sh b --load 1` (1 Rune layer in each window, proof B) and `./run.sh c --load 2` (2 layers, proof C).
Each case ran 5 times. The Rune views are the proof views, not empty views: a button, a text field, a list of 60 rows and a bar (layer 1); the layer in 2-layer runs is the transparent view with a title, a button and a counter. [INFERENCE: this is a heavier view than "empty"; the numbers are an upper bound for the views.]

| Number | 1 layer per window (B), 5 runs | 2 layers per window (C), 5 runs |
| --- | --- | --- |
| Launch to first Rune frame of window 1 (ms) | 482.2, 482.6, 494.3, 502.5, 513.3; median 494.3 | 484.0, 511.8, 520.7, 524.2, 529.0; median 520.7 |
| Memory after 60 s with no input, physical footprint of all app processes (MB) | 1344.5, 1347.6, 1351.6, 1363.6, 1367.6; median 1351.6 | 1323.6, 1345.6, 1346.5, 1364.6, 1388.5; median 1346.5 |
| Memory, resident size sum of the same processes (MB, counts shared pages more than once) | 5151 to 5894; 66 processes | 5859 to 5919; 66 processes |
| p99 frame time, 10 s sidebar open and close loop (ms) | 16.667 in all 5 runs | 16.667 in all 5 runs |
| Longest frame in that loop (ms) | 29.1, 20.1, 28.0, 25.1, 18.7; median 25.1 | 27.2, 21.7, 18.1, 16.7, 16.7; median 18.1 |

Notes:

- Memory is read with `footprint` for all descendants of the app process (the "physical footprint" of macOS); the resident sum is given because it is the number `ps` shows.
- Frame times come from the Rune view's `CADisplayLink`. On this 120 Hz display the link ticked at 60 Hz (interval 16.667 ms, 600 ticks in 10 s), so p99 is the link interval and the longest frame is the useful number. Rune CPU per frame: p99 under 0.5 ms; callback: p95 under 1.5 ms.
- The memory difference between 1 and 2 layers is inside the spread of the runs (about 40 MB).
- Steady state with the page and Rune animating together, proof B (1 window, 1 tab): 620 frames in 10.3 s, 0 frames longer than 20.7 ms.
- Live window resize, proof B: 300 frames, 0 frames with a layer rect or drawable different from the view bounds, 0 longer than 20.7 ms.
- Proof B step: `b-rune-beside-page.png` in the firstmate data folder.
