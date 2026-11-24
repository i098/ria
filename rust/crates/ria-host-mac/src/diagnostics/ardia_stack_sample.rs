// Ported from ardia packages/ardiacore/ios/ArdiaStackSample.c at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Also ports ArdiaStackSample.h. Changes from Ardia: the functions return `Option` instead of filling out
//! parameters, and the declarations that `libc` lacks are here.

#![allow(
    deprecated,
    reason = "libc marks its mach bindings deprecated in favour of the mach2 crate, not a dependency"
)]

use libc::{c_int, mach_msg_type_number_t, mach_port_t, pthread_t, thread_basic_info};

pub const STACK_FRAMES: usize = 24;

const ADDRESS_MASK: u64 = 0x0000_7fff_ffff_ffff;
const P_TRACED: c_int = 0x0000_0800;
const KINFO_PROC_SIZE: usize = 648;
/// The offset of `kp_proc.p_flag` in `struct kinfo_proc` on 64-bit macOS.
const P_FLAG_OFFSET: usize = 32;
#[cfg(target_arch = "aarch64")]
const ARM_THREAD_STATE64: c_int = 6;

#[derive(Clone, Copy, Default)]
pub struct SampledThread {
    pub port: mach_port_t,
    pub stack_low: usize,
    pub stack_high: usize,
}

#[derive(Clone, Copy)]
pub struct StackSample {
    pub count: u32,
    pub run_state: i32,
    pub frames: [u64; STACK_FRAMES],
}

impl Default for StackSample {
    fn default() -> Self {
        Self {
            count: 0,
            run_state: 0,
            frames: [0; STACK_FRAMES],
        }
    }
}

/// `arm_thread_state64_t` without pointer authentication, as on `aarch64-apple-darwin`.
#[cfg(target_arch = "aarch64")]
#[repr(C)]
#[derive(Default)]
struct ArmThreadState64 {
    x: [u64; 29],
    fp: u64,
    lr: u64,
    sp: u64,
    pc: u64,
    cpsr: u32,
    pad: u32,
}

unsafe extern "C" {
    fn thread_suspend(thread: mach_port_t) -> c_int;
    fn thread_resume(thread: mach_port_t) -> c_int;
    fn thread_get_state(
        thread: mach_port_t,
        flavor: c_int,
        state: *mut u32,
        count: *mut mach_msg_type_number_t,
    ) -> c_int;
    fn mach_vm_read_overwrite(
        task: mach_port_t,
        address: u64,
        size: u64,
        data: u64,
        out_size: *mut u64,
    ) -> c_int;
}

pub fn sampled_thread(pthread: pthread_t) -> Option<SampledThread> {
    // SAFETY: `pthread` is a live thread of this process.
    let (high, size) = unsafe {
        (
            libc::pthread_get_stackaddr_np(pthread) as usize,
            libc::pthread_get_stacksize_np(pthread),
        )
    };
    if high == 0 || size == 0 || size > high {
        return None;
    }
    // SAFETY: as above.
    let port = unsafe { libc::pthread_mach_thread_np(pthread) };
    (port != 0).then_some(SampledThread {
        port,
        stack_high: high,
        stack_low: high - size,
    })
}

pub fn debugger_attached() -> bool {
    let mut info = [0u64; KINFO_PROC_SIZE / 8];
    let mut size = KINFO_PROC_SIZE;
    // SAFETY: `getpid` has no preconditions.
    let mut name = [
        libc::CTL_KERN,
        libc::KERN_PROC,
        libc::KERN_PROC_PID,
        unsafe { libc::getpid() },
    ];
    // SAFETY: `info` has room for `size` bytes, and `name` has 4 entries.
    let failed = unsafe {
        libc::sysctl(
            name.as_mut_ptr(),
            4,
            info.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    } != 0;
    if failed {
        return true;
    }
    let flag = (info[P_FLAG_OFFSET / 8] & 0xffff_ffff) as c_int;
    flag & P_TRACED != 0
}

fn read_frame_record(fp: usize) -> Option<[u64; 2]> {
    let mut record = [0u64; 2];
    let mut got = 0u64;
    // SAFETY: `record` has room for 16 bytes; the kernel checks that `fp` is readable.
    let read = unsafe {
        mach_vm_read_overwrite(
            libc::mach_task_self(),
            fp as u64,
            16,
            record.as_mut_ptr() as u64,
            &mut got,
        )
    };
    (read == libc::KERN_SUCCESS && got == 16).then_some(record)
}

fn held_elsewhere(port: mach_port_t, run_state: &mut i32) -> bool {
    // SAFETY: `thread_basic_info` is plain data.
    let mut info: thread_basic_info = unsafe { std::mem::zeroed() };
    let mut count = libc::THREAD_BASIC_INFO_COUNT;
    // SAFETY: `info` has room for `count` integers.
    let read = unsafe {
        libc::thread_info(
            port,
            libc::THREAD_BASIC_INFO as u32,
            (&raw mut info).cast(),
            &mut count,
        )
    };
    if read != libc::KERN_SUCCESS {
        return true;
    }
    *run_state = info.run_state;
    info.suspend_count > 0
}

#[cfg(target_arch = "aarch64")]
fn walk(thread: &SampledThread, state: &ArmThreadState64, sample: &mut StackSample) -> u32 {
    let pc = state.pc & ADDRESS_MASK;
    let lr = state.lr & ADDRESS_MASK;
    let mut fp = state.fp as usize;
    let mut n = 0;
    let mut push = |frame: u64, n: &mut usize| {
        sample.frames[*n] = frame;
        *n += 1;
    };
    if pc != 0 {
        push(pc, &mut n);
    }
    if lr != 0 && n < STACK_FRAMES {
        push(lr, &mut n);
    }
    while n < STACK_FRAMES {
        if fp & 0xf != 0 || fp < thread.stack_low || fp > thread.stack_high - 16 {
            break;
        }
        let Some(record) = read_frame_record(fp) else {
            break;
        };
        let next = record[0] as usize;
        let ret = record[1] & ADDRESS_MASK;
        if ret == 0 {
            break;
        }
        if !(n == 2 && ret == lr) {
            push(ret, &mut n);
        }
        if next <= fp {
            break;
        }
        fp = next;
    }
    n as u32
}

/// Samples the stack of `thread`, which must not be the calling thread. Returns `None` when no frame was read.
#[cfg(target_arch = "aarch64")]
pub fn take(thread: &SampledThread) -> Option<StackSample> {
    let mut sample = StackSample::default();
    // SAFETY: `pthread_self` is the calling thread.
    let me = unsafe { libc::pthread_mach_thread_np(libc::pthread_self()) };
    if thread.port == 0 || thread.port == me || held_elsewhere(thread.port, &mut sample.run_state) {
        return None;
    }
    // SAFETY: `port` names a thread of this process, and the thread is resumed below.
    if unsafe { thread_suspend(thread.port) } != libc::KERN_SUCCESS {
        return None;
    }
    let mut state = ArmThreadState64::default();
    let mut count = (size_of::<ArmThreadState64>() / 4) as mach_msg_type_number_t;
    // SAFETY: `state` has room for `count` 32-bit words.
    let got = unsafe {
        thread_get_state(
            thread.port,
            ARM_THREAD_STATE64,
            (&raw mut state).cast(),
            &mut count,
        )
    } == libc::KERN_SUCCESS;
    let n = if got {
        walk(thread, &state, &mut sample)
    } else {
        0
    };
    // SAFETY: the thread was suspended above.
    unsafe { thread_resume(thread.port) };
    sample.count = n;
    (n > 0).then_some(sample)
}

#[cfg(not(target_arch = "aarch64"))]
pub fn take(_thread: &SampledThread) -> Option<StackSample> {
    None
}
