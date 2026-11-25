# Pending replays: `splits`

Check S (`record-splits.mjs`) records 629 cases: 614 replay and match Ardia, 15 reach a sibling `todo!()` and wait for gate G2 (slice 2.49).
Stubs live in `rust/crates/ria-state/src/splits/pending_siblings.rs`; each names its Ardia source and is replaced when the sibling slice merges.

| Case | Actions | Blocked by |
|---|---|---|
| 71 | removeTabFromSplit | groups: syncGroupOrder(w1) |
| 74 | focusPane, focusPane | slice 2.8 |
| 83 | createSplit | slice 2.5 browser: newId(split) |
| 84 | createSplit | slice 2.5 browser: newId(split) |
| 85 | openSplitPane | groups: leaveGroups({"t2", "t3"}) |
| 86 | openSplitPane | slice 2.10 |
| 87 | openSplitPane | tabs: withNewTab(w1) |
| 88 | openSplitPane | groups: leaveGroups({"t3"}) |
| 89 | removeTabFromSplit | groups: syncGroupOrder(w1) |
| 90 | replaceSplitPane | groups: leaveGroups({"t3"}) |
| 91 | focusPane | slice 2.8 |
| 92 | openInSplit | groups: leaveGroups({"t3", "t2"}) |
| 93 | openLinkInSplit | tabs: withNewTab(w1) |
| 94 | dropTabIntoSplit | groups: leaveGroups({"t3", "t2"}) |
| 95 | toggleTabLayout | slice 2.5 browser: tabLayoutOf(w1) |
