// Ported from ardia packages/ardiacore/ios/ArdiaLaunchSequence.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};

pub const ENGINE_STARTED: u8 = 1;
pub const APP_KIT_FINISHED: u8 = 1 << 2;
pub const APP_READY: u8 = 1 << 3;

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum LaunchReason {
    #[default]
    Normal,
    Hidden,
    Documents,
}

type Waiter = (u8, Box<dyn FnOnce(&LaunchSequence)>);

#[derive(Default)]
pub struct LaunchSequence {
    reached: Cell<u8>,
    reason: Cell<LaunchReason>,
    shut_down: Cell<bool>,
    waiters: RefCell<Vec<Waiter>>,
}

impl LaunchSequence {
    pub fn reached(&self, milestones: u8) -> bool {
        self.reached.get() & milestones == milestones
    }

    pub fn reason(&self) -> LaunchReason {
        self.reason.get()
    }

    pub fn engine_running(&self) -> bool {
        self.reached(ENGINE_STARTED) && !self.shut_down.get()
    }

    pub fn app_kit_finished(&self, reason: LaunchReason) {
        self.reason.set(reason);
        self.reach(APP_KIT_FINISHED);
    }

    pub fn reach(&self, milestones: u8) {
        if self.shut_down.get() {
            return;
        }
        self.reached.set(self.reached.get() | milestones);
        loop {
            let ready = {
                let mut waiters = self.waiters.borrow_mut();
                waiters
                    .iter()
                    .position(|(bits, _)| self.reached(*bits))
                    .map(|index| waiters.remove(index).1)
            };
            let Some(run) = ready else { return };
            run(self);
        }
    }

    pub fn after(&self, milestones: u8, run: impl FnOnce(&Self) + 'static) {
        if !self.shut_down.get() {
            self.waiters.borrow_mut().push((milestones, Box::new(run)));
            self.reach(0);
        }
    }

    pub fn engine_shutdown(&self) {
        self.shut_down.set(true);
        drop(self.waiters.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[derive(Default)]
    struct Host {
        launch: Rc<LaunchSequence>,
        log: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Host {
        fn run(&self) {
            let log = self.log.clone();
            self.launch
                .after(ENGINE_STARTED | APP_KIT_FINISHED, move |launch| {
                    log.borrow_mut().push(match launch.reason() {
                        LaunchReason::Hidden => "earlyWindow hidden",
                        LaunchReason::Documents => "earlyWindow documents",
                        LaunchReason::Normal => "earlyWindow",
                    });
                });
        }

        fn engine_started(&self) {
            self.log.borrow_mut().push("setup");
            self.launch.reach(ENGINE_STARTED);
        }

        fn launch_urls(&self) {
            let log = self.log.clone();
            self.launch
                .after(APP_READY, move |_| log.borrow_mut().push("parkLinks"));
            let log = self.log.clone();
            self.launch.after(APP_KIT_FINISHED, move |_| {
                log.borrow_mut().push("deliverURLs")
            });
        }

        fn app_ready(&self) {
            self.launch.reach(APP_READY);
            self.log.borrow_mut().push("launchTab");
        }

        fn expect(&self, expected: &[&str]) {
            assert_eq!(&*self.log.borrow(), expected);
        }
    }

    #[test]
    fn engine_starts_before_app_kit() {
        let host = Host::default();
        host.run();
        host.engine_started();
        host.expect(&["setup"]);
        host.launch.app_kit_finished(LaunchReason::Normal);
        host.app_ready();
        host.expect(&["setup", "earlyWindow", "launchTab"]);
    }

    #[test]
    fn app_kit_finishes_before_engine() {
        let host = Host::default();
        host.run();
        host.launch.app_kit_finished(LaunchReason::Normal);
        host.expect(&[]);
        host.engine_started();
        host.expect(&["setup", "earlyWindow"]);
        host.launch_urls();
        host.app_ready();
        host.expect(&[
            "setup",
            "earlyWindow",
            "deliverURLs",
            "parkLinks",
            "launchTab",
        ]);
    }

    #[test]
    fn urls_before_app_kit_finish() {
        let host = Host::default();
        host.run();
        host.engine_started();
        host.launch_urls();
        host.expect(&["setup"]);
        host.launch.app_kit_finished(LaunchReason::Normal);
        host.expect(&["setup", "earlyWindow", "deliverURLs"]);
        host.app_ready();
        host.expect(&[
            "setup",
            "earlyWindow",
            "deliverURLs",
            "parkLinks",
            "launchTab",
        ]);
    }

    #[test]
    fn urls_after_app_ready() {
        let host = Host::default();
        host.run();
        host.engine_started();
        host.launch.app_kit_finished(LaunchReason::Normal);
        host.app_ready();
        host.launch_urls();
        host.expect(&[
            "setup",
            "earlyWindow",
            "launchTab",
            "parkLinks",
            "deliverURLs",
        ]);
    }

    #[test]
    fn hidden_launch() {
        let host = Host::default();
        host.run();
        host.engine_started();
        host.launch.app_kit_finished(LaunchReason::Hidden);
        host.app_ready();
        host.expect(&["setup", "earlyWindow hidden", "launchTab"]);
    }

    #[test]
    fn document_launch() {
        let host = Host::default();
        host.run();
        host.launch.app_kit_finished(LaunchReason::Documents);
        host.engine_started();
        host.expect(&["setup", "earlyWindow documents"]);
    }

    #[test]
    fn reentrant_waiters_run_once() {
        let launch = LaunchSequence::default();
        let count = Rc::new(Cell::new(0));
        let inner = count.clone();
        launch.after(ENGINE_STARTED, move |launch| {
            inner.set(inner.get() + 1);
            launch.after(ENGINE_STARTED, move |_| inner.set(inner.get() + 1));
            launch.reach(ENGINE_STARTED);
        });
        launch.reach(ENGINE_STARTED);
        launch.reach(ENGINE_STARTED);
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn shutdown_discards_waiters() {
        let launch = LaunchSequence::default();
        launch.reach(ENGINE_STARTED);
        launch.after(APP_READY, |_| panic!("waiter fired after shutdown"));
        launch.engine_shutdown();
        launch.reach(APP_READY);
        launch.after(0, |_| panic!("waiter fired after shutdown"));
        assert!(!launch.engine_running());
        assert!(!launch.reached(APP_READY));
    }
}
