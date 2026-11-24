// Ported from ardia packages/ardiacore/ios/ArdiaFreezeArming.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia: the timer is a closure instead of a function pointer with a context, and a `Mutex` takes the
//! place of the `os_unfair_lock`.

use std::marker::PhantomData;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering::SeqCst};

pub trait ArmingHooks {
    fn inside_disarm() {}
}

pub struct NoArmingHooks;

impl ArmingHooks for NoArmingHooks {}

type SetTimer = Box<dyn Fn(u64) + Send + Sync>;

pub struct FreezeArming<Hooks: ArmingHooks = NoArmingHooks> {
    set_timer: SetTimer,
    lock: Mutex<()>,
    deadline: AtomicU64,
    busy: AtomicI32,
    capturing: AtomicI32,
    hooks: PhantomData<Hooks>,
}

impl<Hooks: ArmingHooks> FreezeArming<Hooks> {
    pub fn new(set_timer: impl Fn(u64) + Send + Sync + 'static) -> Self {
        Self {
            set_timer: Box::new(set_timer),
            lock: Mutex::new(()),
            deadline: AtomicU64::new(0),
            busy: AtomicI32::new(0),
            capturing: AtomicI32::new(0),
            hooks: PhantomData,
        }
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, ()> {
        self.lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn wake(&self, now: u64, freeze_ticks: u64, active: bool) {
        self.busy.fetch_add(1, SeqCst);
        if active {
            self.arm_at(now + freeze_ticks);
        }
    }

    pub fn sleep(&self) {
        if self.busy.fetch_sub(1, SeqCst) == 1 {
            self.disarm_if_idle();
        }
    }

    pub fn arm_at(&self, due: u64) {
        let _lock = self.locked();
        let current = self.deadline.load(SeqCst);
        if current == 0 || due < current {
            self.deadline.store(due, SeqCst);
            (self.set_timer)(due);
        }
    }

    pub fn fired(&self) {
        let _lock = self.locked();
        self.deadline.store(0, SeqCst);
    }

    pub fn disarm_if_idle(&self) {
        let _lock = self.locked();
        if self.deadline.load(SeqCst) != 0
            && self.busy.load(SeqCst) == 0
            && self.capturing.load(SeqCst) == 0
        {
            Hooks::inside_disarm();
            self.deadline.store(0, SeqCst);
            (self.set_timer)(0);
        }
    }

    pub fn capture_began(&self) {
        self.capturing.fetch_add(1, SeqCst);
    }

    pub fn capture_ended(&self) {
        self.capturing.fetch_sub(1, SeqCst);
    }

    pub fn deadline(&self) -> u64 {
        self.deadline.load(SeqCst)
    }

    pub fn busy(&self) -> i32 {
        self.busy.load(SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, LazyLock};
    use std::thread;
    use std::time::Duration;

    #[derive(Default)]
    struct Timer {
        due: AtomicU64,
        sets: AtomicI32,
    }

    fn arming<Hooks: ArmingHooks>() -> (Arc<Timer>, FreezeArming<Hooks>) {
        let timer = Arc::new(Timer::default());
        let set = timer.clone();
        let arming = FreezeArming::new(move |due| {
            set.due.store(due, SeqCst);
            set.sets.fetch_add(1, SeqCst);
        });
        (timer, arming)
    }

    static RACED: LazyLock<(Arc<Timer>, FreezeArming<WakeDuringDisarm>)> = LazyLock::new(arming);
    static WAKE_DONE: AtomicBool = AtomicBool::new(false);
    static INJECTED: AtomicBool = AtomicBool::new(false);

    struct WakeDuringDisarm;

    impl ArmingHooks for WakeDuringDisarm {
        fn inside_disarm() {
            if INJECTED.swap(true, SeqCst) {
                return;
            }
            // The waker blocks on the lock that the disarm holds, as in Ardia; it is detached after 200 ms.
            thread::spawn(|| {
                RACED.1.wake(2000, 400, true);
                WAKE_DONE.store(true, SeqCst);
            });
            for _ in 0..200 {
                if WAKE_DONE.load(SeqCst) {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
        }
    }

    #[test]
    fn wake_during_idle_cancellation_keeps_the_watchdog_armed() {
        let (timer, arming) = &*RACED;
        arming.wake(1000, 400, true);
        arming.sleep();
        for _ in 0..2000 {
            if WAKE_DONE.load(SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(WAKE_DONE.load(SeqCst), "the waking loop finished");
        assert_eq!(arming.busy(), 1, "one loop is busy after the interleaving");
        assert!(
            arming.deadline() != 0 && timer.due.load(SeqCst) == arming.deadline(),
            "a loop that woke while idle cancellation ran still has its watchdog armed"
        );
    }

    #[test]
    fn short_stretches_cost_no_wakeup() {
        let (timer, arming) = arming::<NoArmingHooks>();
        arming.wake(1000, 400, true);
        assert_eq!(timer.due.load(SeqCst), 1400, "a wake arms 400 ms ahead");
        arming.sleep();
        assert!(
            arming.deadline() == 0 && timer.due.load(SeqCst) == 0,
            "sleeping with no capture disarms"
        );
        arming.wake(5000, 400, false);
        assert_eq!(arming.deadline(), 0, "an inactive app never arms");
        arming.sleep();
    }

    #[test]
    fn a_capture_keeps_its_deadline() {
        let (_timer, arming) = arming::<NoArmingHooks>();
        arming.wake(1000, 400, true);
        arming.capture_began();
        arming.sleep();
        assert_eq!(
            arming.deadline(),
            1400,
            "a capture in progress keeps the deadline when the loops sleep"
        );
        arming.capture_ended();
        arming.disarm_if_idle();
        assert_eq!(
            arming.deadline(),
            0,
            "once the capture ends an idle disarm clears it"
        );
    }

    #[test]
    fn the_other_loop_busy_keeps_the_deadline() {
        let (_timer, arming) = arming::<NoArmingHooks>();
        arming.wake(1000, 400, true);
        arming.wake(1100, 400, true);
        arming.sleep();
        assert_eq!(
            arming.deadline(),
            1400,
            "one loop sleeping leaves the other's watchdog armed"
        );
        arming.sleep();
        assert_eq!(arming.deadline(), 0, "both asleep: disarmed");
    }

    #[test]
    fn stress_wakes_and_sleeps() {
        let (_timer, arming) = arming::<NoArmingHooks>();
        let lost = AtomicI32::new(0);
        let run = || {
            for i in 0..200_000 {
                arming.wake(1000 + i, 400, true);
                if arming.deadline() == 0 {
                    lost.fetch_add(1, SeqCst);
                }
                arming.sleep();
            }
        };
        thread::scope(|scope| {
            scope.spawn(run);
            scope.spawn(run);
        });
        assert_eq!(
            lost.load(SeqCst),
            0,
            "200,000 wakes on each of two threads: no busy loop ever saw its watchdog disarmed"
        );
        assert!(
            arming.busy() == 0 && arming.deadline() == 0,
            "both asleep at the end: disarmed"
        );
    }
}
