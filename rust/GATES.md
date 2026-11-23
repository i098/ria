# Gates

One row for each gate. A slice starts only when each gate in its `Needs` column has a result here.
Each gate G1 to G6 also writes its drift report here: the count of changed units and changed lines of Ardia `origin/main` against `551d3c13c`.
Gate G0 also has the memory and frame-time numbers of proofs B and C.
Gate G1 also has the planned and the real hours of phase 1 and the corrected hour rates for the later phases.

| Gate | Needs | Result | Date | Evidence | Drift (units, lines) |
| --- | --- | --- | --- | --- | --- |
| Plan | owner approval on the board | approved | 2026-11-23 | Owner, on the board: "I approve this implementation plan. Implement it as written." Plan version 5. | |
| G0 | slices 0.5 to 0.9 are done; proofs A, B, C and D pass; owner go on the gate issue | open | | | |
| G1 | all Phase 1 checks pass in one run | open | | | |
| G2 | all Phase 2 checks pass in one run, with all store diffs | open | | | |
| G3 | all Phase 3 checks pass in one run | open | | | |
| G4 | all Phase 4 checks pass in one run; resource numbers in the limit | open | | | |
| G5 | all Phase 5 checks pass in one run | open | | | |
| G6 | full parity run and resource check pass; owner go | open | | | |
