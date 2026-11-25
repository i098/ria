# Slice 2.5 pending replay sequences

Check S was not run on recorded sequences: the recorder for `state/browser` is not written. Every action that
reaches a sibling stub in `rust/crates/ria-state/src/browser/pending_siblings.rs` waits for the slice that merges
that sibling (gate G2, slice 2.49). The `closeTab`, `closeGroup`, `deleteProfile`, profile-move and `openProfileMenu`
actions stop at `todo!()` after their first guard (dialog second action not written).
