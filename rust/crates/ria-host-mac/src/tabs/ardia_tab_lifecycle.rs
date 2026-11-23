// Ported from ardia packages/ardiacore/ios/ArdiaTabLifecycle.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Phase {
    #[default]
    Waiting,
    Creating,
    Attached,
    Closing,
}

#[derive(Default)]
pub struct TabLifecycle {
    phase: Phase,
    create_epoch: u64,
    pub pending_url: Option<String>,
    pub pending_user_initiated: bool,
    pub creating_url: Option<String>,
    pub creating_at: Option<std::time::Instant>,
    pub transferred_url: Option<String>,
    pub handing_off: bool,
    pub moving: bool,
    pub offer_closed: bool,
    pub launch_close_sent: bool,
    pub keeps_page_on_detach: bool,
    pub waiting_for_size: bool,
}

impl TabLifecycle {
    pub fn closed(&self) -> bool {
        self.phase == Phase::Closing
    }

    pub fn can_create(&self) -> bool {
        self.phase == Phase::Waiting
    }

    pub fn begin_creating(&mut self) -> u64 {
        self.phase = Phase::Creating;
        self.create_epoch = self.create_epoch.wrapping_add(1);
        self.create_epoch
    }

    pub fn is_current(&self, epoch: u64) -> bool {
        self.create_epoch == epoch && !self.closed()
    }

    pub fn resume_creating(&mut self) {
        if self.can_create() {
            self.phase = Phase::Creating;
        }
    }

    pub fn leave_creating(&mut self) {
        if self.phase == Phase::Creating {
            self.phase = Phase::Waiting;
        }
    }

    pub fn can_attach(&self, has_tab: bool) -> bool {
        !self.closed() && !has_tab
    }

    pub fn attached(&mut self) {
        self.phase = Phase::Attached;
    }

    pub fn detached(&mut self) {
        if self.phase == Phase::Attached {
            self.phase = Phase::Waiting;
        }
    }

    pub fn hand_off(&mut self) {
        self.create_epoch = self.create_epoch.wrapping_add(1);
    }

    pub fn close(&mut self) {
        self.hand_off();
        self.phase = Phase::Closing;
    }
}

#[cfg(test)]
#[path = "tab_lifecycle_tests.rs"]
mod tests;
