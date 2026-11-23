// Ported from ardia packages/ardiacore/tests/tab-lifecycle.test.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use super::{Phase, TabLifecycle};

#[test]
fn close_during_create_invalidates_reply() {
    let mut life = TabLifecycle::default();
    assert!(life.can_create());
    let epoch = life.begin_creating();
    assert!(!life.can_create());
    life.close();
    assert!(!life.is_current(epoch));
    assert!(life.closed() && !life.can_create() && !life.can_attach(false));
    life.leave_creating();
    life.resume_creating();
    assert!(life.closed());
}

#[test]
fn detach_after_close_does_not_reopen() {
    let mut life = TabLifecycle::default();
    life.begin_creating();
    life.attached();
    life.close();
    life.detached();
    assert!(life.closed() && !life.can_create());
}

#[test]
fn close_while_waiting_prevents_create_and_attach() {
    let mut life = TabLifecycle::default();
    life.close();
    assert!(life.closed() && !life.can_create() && !life.can_attach(false));
}

#[test]
fn handoff_detach_allows_create_with_new_epoch() {
    let mut life = TabLifecycle::default();
    let before = life.begin_creating();
    life.attached();
    life.hand_off();
    life.detached();
    assert_eq!(life.phase, Phase::Waiting);
    assert!(life.can_create() && life.create_epoch > before);
}

#[test]
fn retry_invalidates_prior_create_reply() {
    let mut life = TabLifecycle::default();
    let epoch = life.begin_creating();
    assert!(life.is_current(epoch));
    life.leave_creating();
    assert_eq!(life.phase, Phase::Waiting);
    assert!(life.can_create());
    let next = life.begin_creating();
    assert!(next > epoch && !life.is_current(epoch) && life.is_current(next));
}

#[test]
fn attach_requires_no_existing_tab() {
    let mut life = TabLifecycle::default();
    life.begin_creating();
    life.attached();
    assert!(!life.can_attach(true));
    assert!(life.can_attach(false));
}
