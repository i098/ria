// Ported from ardia packages/plugin-broker/crates/host/src/sandbox.rs at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
use std::os::fd::RawFd;

pub struct ConfinePoint<'a> {
    pub channel_fd: RawFd,
    pub kept_fds: &'a [RawFd],
    pub threads: usize,
}

pub trait Sandbox {
    fn confine(&mut self, point: &ConfinePoint<'_>) -> Result<(), String>;
}

pub struct Unconfined;

impl Sandbox for Unconfined {
    fn confine(&mut self, _point: &ConfinePoint<'_>) -> Result<(), String> {
        Ok(())
    }
}

pub const SYSCALLS_AFTER_CONFINE: &[&str] = &[
    "read",
    "write",
    "poll",
    "ppoll",
    "clock_gettime",
    "clock_nanosleep",
    "nanosleep",
    "getrandom",
    "brk",
    "mmap",
    "munmap",
    "mremap",
    "madvise",
    "futex",
    "rt_sigreturn",
    "exit_group",
];
