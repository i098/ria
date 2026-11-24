// Ported from ardia packages/ardiacore/ios/ArdiaFreezeCapture.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Also ports ArdiaFreezeCapture.h. Changes from Ardia:
//! - `configure` gives the data folder, the `freezeWatchActive` test flag and the consent epoch, which Ardia reads
//!   from `ArdiaLaunchConfig` and `ArdiaEvents`. Without it no records file is opened, where Ardia falls back to
//!   Application Support.
//! - A watched stretch is an `Arc<Stretch>`, compared by pointer as Ardia compares `const Stretch *`.
//! - The emitter takes the record as JSON; whole-number values are integers.
//! - Ardia's unread `Capture.start` field is not kept.

#![allow(
    deprecated,
    reason = "libc marks its mach bindings deprecated in favour of the mach2 crate, not a dependency"
)]

use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock};

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSApplication, NSApplicationDidBecomeActiveNotification,
    NSApplicationDidResignActiveNotification,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSProcessInfo,
    NSProcessInfoThermalStateDidChangeNotification,
};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use super::ardia_freeze_arming::FreezeArming;
use super::ardia_stack_sample::{self as stack, SampledThread, StackSample};

pub const THREAD_COUNT: usize = 2;

const FREEZE_MS: f64 = 400.0;
const SAMPLE_GAP_MS: f64 = 100.0;
const SAMPLES: usize = 3;
const WRITE_AT_MS: f64 = 2000.0;
const SLOTS: usize = 20;
const SLOT_BYTES: usize = 4096;
const SLOT_TAIL: usize = 640;
const MAX_IMAGES: usize = 6;
const KEPT: usize = 4;
const OWN_IMAGES: [&str; 2] = ["ria-app", "Chromium Framework"];
const THREAD_NAMES: [&str; THREAD_COUNT] = ["main", "js"];
const LC_UUID: u32 = 0x1b;
const QOS_CLASS_UTILITY: u32 = 0x11;
const MACH_PORT_RIGHT_SEND: u32 = 0;
const DISPATCH_TIME_FOREVER: u64 = !0;
const NSEC_PER_MSEC: u64 = 1_000_000;

#[derive(Default)]
pub struct Stretch {
    pub start: AtomicU64,
    pub id: AtomicU64,
}

pub struct Ended {
    pub thread: usize,
    pub source: Arc<Stretch>,
    pub id: u64,
    pub ms: f64,
    pub input_wait_ms: f64,
    pub gc_ms: f64,
    pub gc_count: f64,
    pub epoch: u64,
}

pub struct Config {
    pub data_directory: PathBuf,
    /// The `freezeWatchActive` test flag: watch as if the app were active.
    pub acted_out: bool,
    /// `ArdiaEvents.consentEpoch`; called on a worker queue.
    pub consent_epoch: Box<dyn Fn() -> String + Send + Sync>,
}

type Emitter = Arc<dyn Fn(Value) + Send + Sync>;

#[derive(Clone)]
struct Image {
    name: String,
    low: u64,
    high: u64,
    uuid: String,
}

#[derive(Default)]
struct Images {
    own: Vec<Image>,
    cache_low: u64,
    cache_high: u64,
}

#[derive(Clone, Default)]
struct Capture {
    active: bool,
    source: Option<Arc<Stretch>>,
    stretch: u64,
    started_ms: f64,
    thread: usize,
    attempts: usize,
    count: usize,
    written: bool,
    slot: usize,
    seq: u64,
    thermal: i32,
    duration_ms: f64,
    input_wait_ms: f64,
    gc_ms: f64,
    gc_count: f64,
    activity: String,
    consent: String,
    samples: [StackSample; SAMPLES],
}

impl Capture {
    fn holds(&self, id: u64, source: &Arc<Stretch>) -> bool {
        self.active
            && self.stretch == id
            && self
                .source
                .as_ref()
                .is_some_and(|own| Arc::ptr_eq(own, source))
    }
}

struct Watched {
    stretch: Arc<Stretch>,
    sampled: SampledThread,
}

#[derive(Default)]
struct Records {
    captures: [Capture; THREAD_COUNT],
    kept: [Capture; KEPT],
    kept_next: usize,
    file: Option<File>,
    next_slot: usize,
    seq: u64,
}

struct Queues {
    work: usize,
    timer: usize,
}

static CONFIG: OnceLock<Config> = OnceLock::new();
static QUEUES: OnceLock<Queues> = OnceLock::new();
static IMAGES: OnceLock<Images> = OnceLock::new();
static WATCHED: Mutex<[Option<Watched>; THREAD_COUNT]> = Mutex::new([None, None]);
static RECORDS: LazyLock<Mutex<Records>> = LazyLock::new(Mutex::default);
static EMIT: Mutex<Option<Emitter>> = Mutex::new(None);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(false);
static THERMAL: AtomicI32 = AtomicI32::new(0);
static ARMING: LazyLock<FreezeArming> = LazyLock::new(|| FreezeArming::new(set_timer));

thread_local! {
    static ACTIVE_OBSERVERS: std::cell::RefCell<Vec<Retained<AnyObject>>> = const { std::cell::RefCell::new(Vec::new()) };
}

unsafe extern "C" {
    static _dispatch_source_type_timer: c_void;
    fn dispatch_queue_attr_make_with_qos_class(
        attr: *const c_void,
        qos: u32,
        priority: i32,
    ) -> *const c_void;
    fn dispatch_queue_create(label: *const libc::c_char, attr: *const c_void) -> *mut c_void;
    fn dispatch_source_create(
        kind: *const c_void,
        handle: usize,
        mask: usize,
        queue: *mut c_void,
    ) -> *mut c_void;
    fn dispatch_source_set_event_handler_f(
        source: *mut c_void,
        handler: extern "C" fn(*mut c_void),
    );
    fn dispatch_source_set_timer(source: *mut c_void, start: u64, interval: u64, leeway: u64);
    fn dispatch_resume(object: *mut c_void);
    fn dispatch_time(when: u64, delta: i64) -> u64;
    fn dispatch_async_f(queue: *mut c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
    fn mach_port_mod_refs(task: u32, name: u32, right: u32, delta: i32) -> i32;
    fn mach_port_deallocate(task: u32, name: u32) -> i32;
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn ticks_per_ms() -> f64 {
    static PER_MS: LazyLock<f64> = LazyLock::new(|| {
        let mut base = libc::mach_timebase_info { numer: 0, denom: 0 };
        // SAFETY: `base` is a valid out parameter.
        unsafe { libc::mach_timebase_info(&mut base) };
        1e6 * f64::from(base.denom) / f64::from(base.numer)
    });
    *PER_MS
}

fn ms_to_ticks(ms: f64) -> u64 {
    (ms * ticks_per_ms()) as u64
}

pub(super) fn ticks_to_ms(ticks: u64) -> f64 {
    ticks as f64 / ms_to_ticks(1.0) as f64
}

pub(super) fn now_ticks() -> u64 {
    // SAFETY: No preconditions.
    unsafe { libc::mach_absolute_time() }
}

fn wall_ms() -> f64 {
    let mut now = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    // SAFETY: `now` is a valid out parameter.
    unsafe { libc::gettimeofday(&mut now, std::ptr::null_mut()) };
    now.tv_sec as f64 * 1000.0 + f64::from(now.tv_usec / 1000)
}

/// The text that fits a C buffer of `size` bytes, as `strlcpy` keeps it.
fn clipped(text: &str, size: usize) -> String {
    let mut end = text.len().min(size - 1);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn format_uuid(bytes: &[u8; 16]) -> String {
    let mut out = String::with_capacity(36);
    for (i, byte) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// The `__TEXT` range and the UUID of the image whose header is at `header`.
///
/// # Safety
/// `header` is null or points at a loaded Mach-O image.
unsafe fn text_range(header: *const libc::mach_header_64) -> Option<(u64, u64, String)> {
    // SAFETY: by the contract.
    let header = unsafe { header.as_ref() }?;
    if header.magic != libc::MH_MAGIC_64 {
        return None;
    }
    let (mut low, mut high, mut uuid) = (0, 0, String::new());
    // SAFETY: the load commands follow the header.
    let mut command = unsafe { (header as *const libc::mach_header_64).add(1).cast::<u8>() };
    for _ in 0..header.ncmds {
        // SAFETY: every load command starts with a `load_command`, and `cmdsize` leads to the next one.
        unsafe {
            let load = &*command.cast::<libc::load_command>();
            if load.cmd == libc::LC_SEGMENT_64 {
                let segment = &*command.cast::<libc::segment_command_64>();
                if std::ffi::CStr::from_ptr(segment.segname.as_ptr()) == c"__TEXT" {
                    low = header as *const _ as u64;
                    high = low + segment.vmsize;
                }
            } else if load.cmd == LC_UUID {
                uuid = format_uuid(&*command.add(8).cast::<[u8; 16]>());
            }
            command = command.add(load.cmdsize as usize);
        }
    }
    (high > low).then_some((low, high, uuid))
}

fn load_images() -> Images {
    let mut images = Images::default();
    // SAFETY: dyld calls are safe to make at any time; the strings and headers stay loaded.
    for i in 0..unsafe { libc::_dyld_image_count() } {
        let name = unsafe { libc::_dyld_get_image_name(i) };
        if name.is_null() {
            continue;
        }
        let path = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
        let header = unsafe { libc::_dyld_get_image_header(i) }.cast::<libc::mach_header_64>();
        if path.starts_with("/System/") || path.starts_with("/usr/lib/") {
            // SAFETY: dyld gave the header.
            if let Some((low, high, _)) = unsafe { text_range(header) } {
                images.cache_low = if images.cache_low == 0 {
                    low
                } else {
                    images.cache_low.min(low)
                };
                images.cache_high = images.cache_high.max(high);
            }
            continue;
        }
        let base = path.rsplit('/').next().unwrap_or(&path);
        if images.own.len() >= MAX_IMAGES || !OWN_IMAGES.contains(&base) {
            continue;
        }
        // SAFETY: as above.
        if let Some((low, high, uuid)) = unsafe { text_range(header) }
            && !uuid.is_empty()
        {
            images.own.push(Image {
                name: clipped(base, 32),
                low,
                high,
                uuid,
            });
        }
    }
    images
}

/// The image index of `pc` (`-1` for the system cache, `-2` for none) and the offset in it.
fn image_of(images: &Images, pc: u64) -> (isize, u64) {
    if let Some(i) = images
        .own
        .iter()
        .position(|image| pc >= image.low && pc < image.high)
    {
        return (i as isize, pc - images.own[i].low);
    }
    if pc >= images.cache_low && pc < images.cache_high {
        return (-1, pc - images.cache_low);
    }
    (-2, 0)
}

fn image_name(images: &Images, image: isize) -> &str {
    match image {
        -1 => "system",
        i if i >= 0 => &images.own[i as usize].name,
        _ => "other",
    }
}

struct Text {
    out: [u8; SLOT_BYTES],
    used: usize,
}

impl Text {
    fn put(&mut self, s: &str) {
        for &byte in s.as_bytes() {
            if self.used >= SLOT_BYTES - 1 {
                return;
            }
            self.out[self.used] = byte;
            self.used += 1;
        }
    }

    fn num(&mut self, n: u64) {
        self.put(&n.to_string());
    }

    fn hex(&mut self, n: u64) {
        self.put(&format!("0x{n:x}"));
    }
}

fn write_samples(text: &mut Text, c: &Capture, images: &Images, used: &mut [bool; MAX_IMAGES]) {
    for (s, sample) in c.samples[..c.count].iter().enumerate() {
        if text.used >= SLOT_BYTES - SLOT_TAIL {
            break;
        }
        text.put("sample ");
        text.num(s as u64 + 1);
        text.put(" state ");
        text.num(sample.run_state.max(0) as u64);
        text.put("\n");
        for &frame in &sample.frames[..sample.count as usize] {
            if text.used >= SLOT_BYTES - SLOT_TAIL {
                break;
            }
            let (image, offset) = image_of(images, frame);
            if image >= 0 {
                used[image as usize] = true;
            }
            text.put("  ");
            text.put(image_name(images, image));
            text.put(" +");
            text.hex(offset);
            text.put("\n");
        }
    }
}

fn slot_text(c: &Capture, images: &Images) -> Text {
    let mut text = Text {
        out: [0; SLOT_BYTES],
        used: 0,
    };
    text.put("freeze ");
    text.num(c.seq);
    text.put(" ");
    text.put(THREAD_NAMES[c.thread]);
    text.put(if c.duration_ms > 0.0 {
        " recovered\n"
    } else {
        " frozen\n"
    });
    text.put("started_ms ");
    text.num(c.started_ms as u64);
    text.put("\nduration_ms ");
    text.num(if c.duration_ms > 0.0 {
        c.duration_ms
    } else {
        WRITE_AT_MS
    } as u64);
    text.put("\ninput_wait_ms ");
    text.num(c.input_wait_ms as u64);
    text.put("\ngc_ms ");
    if c.gc_ms >= 0.0 {
        text.num(c.gc_ms as u64);
    } else {
        text.put("unknown");
    }
    text.put("\nactivity ");
    text.put(if c.activity.is_empty() {
        "none"
    } else {
        &c.activity
    });
    text.put("\nthermal ");
    text.num(c.thermal as u64);
    text.put("\n");
    let mut used = [false; MAX_IMAGES];
    write_samples(&mut text, c, images, &mut used);
    for (image, _) in images.own.iter().zip(used).filter(|(_, used)| *used) {
        text.put("image ");
        text.put(&image.name);
        text.put(" ");
        text.put(&image.uuid);
        text.put("\n");
    }
    text.put("end\n");
    text
}

fn write_slot(records: &Records, c: &Capture) {
    let (Some(file), Some(images)) = (&records.file, IMAGES.get()) else {
        return;
    };
    let mut text = slot_text(c, images);
    text.out[text.used..SLOT_BYTES - 1].fill(b' ');
    text.out[SLOT_BYTES - 1] = b'\n';
    let _ = file.write_at(&text.out, (c.slot * SLOT_BYTES) as u64);
}

fn leading_number(text: &[u8]) -> u64 {
    text.iter()
        .take_while(|byte| byte.is_ascii_digit())
        .fold(0u64, |n, byte| {
            n.saturating_mul(10).saturating_add(u64::from(byte - b'0'))
        })
}

fn open_records(records: &mut Records) {
    let Some(config) = CONFIG.get() else {
        return;
    };
    let path = config.data_directory.join("freezes.txt");
    let existing = std::fs::read(&path).unwrap_or_default();
    let Ok(file) = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
    else {
        return;
    };
    records.file = Some(file);
    let mut newest = 0;
    for (slot, text) in existing
        .as_chunks::<SLOT_BYTES>()
        .0
        .iter()
        .take(SLOTS)
        .enumerate()
    {
        let Some(rest) = text.strip_prefix(b"freeze ") else {
            continue;
        };
        let seq = leading_number(rest);
        if seq > newest {
            newest = seq;
            records.next_slot = (slot + 1) % SLOTS;
        }
    }
    records.seq = newest;
}

fn set_timer(due: u64) {
    let Some(queues) = QUEUES.get() else {
        return;
    };
    let timer = queues.timer as *mut c_void;
    // SAFETY: `timer` is the live timer source that `setup` made.
    unsafe {
        if due == 0 {
            dispatch_source_set_timer(timer, DISPATCH_TIME_FOREVER, DISPATCH_TIME_FOREVER, 0);
            return;
        }
        let now = now_ticks();
        let ns = if due > now {
            (ticks_to_ms(due - now) * 1e6) as i64
        } else {
            0
        };
        dispatch_source_set_timer(
            timer,
            dispatch_time(0, ns),
            DISPATCH_TIME_FOREVER,
            20 * NSEC_PER_MSEC,
        );
    }
}

fn begin(records: &mut Records, thread: usize, source: &Arc<Stretch>, stretch: u64, start: u64) {
    let consent = CONFIG
        .get()
        .map(|config| (config.consent_epoch)())
        .unwrap_or_default();
    let seq = records.seq + 1;
    let slot = records.next_slot;
    records.seq = seq;
    records.next_slot = (slot + 1) % SLOTS;
    let c = &mut records.captures[thread];
    if c.active {
        ARMING.capture_ended();
    }
    *c = Capture {
        active: true,
        source: Some(source.clone()),
        thread,
        stretch,
        started_ms: wall_ms() - ticks_to_ms(now_ticks() - start),
        gc_ms: -1.0,
        gc_count: -1.0,
        thermal: THERMAL.load(Ordering::Relaxed),
        consent: clipped(&consent, 48),
        seq,
        slot,
        ..Capture::default()
    };
    ARMING.capture_began();
}

fn clear(c: &mut Capture) {
    if c.active {
        ARMING.capture_ended();
    }
    *c = Capture::default();
}

/// What one firing found for one thread.
struct Due {
    next: u64,
    write: u64,
}

struct Debugger(Option<bool>);

impl Debugger {
    fn attached(&mut self) -> bool {
        *self.0.get_or_insert_with(stack::debugger_attached)
    }
}

fn sample_watched(
    records: &mut Records,
    t: usize,
    watched: &Watched,
    debugger: &mut Debugger,
) -> Due {
    let source = &watched.stretch;
    let id = source.id.load(Ordering::Acquire);
    let start = source.start.load(Ordering::Acquire);
    let mut due = Due {
        next: u64::MAX,
        write: 0,
    };
    if start == 0 || id != source.id.load(Ordering::Acquire) {
        return due;
    }
    let now = now_ticks();
    if now - start < ms_to_ticks(FREEZE_MS) {
        due.next = start + ms_to_ticks(FREEZE_MS);
        return due;
    }
    if !records.captures[t].holds(id, source) && ACTIVE.load(Ordering::SeqCst) {
        begin(records, t, source, id, start);
    }
    let c = &mut records.captures[t];
    if !c.holds(id, source) {
        return due;
    }
    if c.attempts < SAMPLES {
        let attached = debugger.attached();
        c.attempts = if attached { SAMPLES } else { c.attempts + 1 };
        if !attached
            && let Some(sample) = stack::take(&watched.sampled)
            && source.id.load(Ordering::Acquire) == id
            && source.start.load(Ordering::Acquire) != 0
        {
            c.samples[c.count] = sample;
            c.count += 1;
        }
        if c.attempts < SAMPLES {
            due.next = now + ms_to_ticks(SAMPLE_GAP_MS);
        }
    }
    if !c.written {
        if now - start >= ms_to_ticks(WRITE_AT_MS) {
            due.write = c.seq;
        } else {
            due.next = due.next.min(start + ms_to_ticks(WRITE_AT_MS));
        }
    }
    due
}

fn fire() {
    ARMING.fired();
    if !READY.load(Ordering::SeqCst) {
        return;
    }
    let mut next = u64::MAX;
    let mut write = [0u64; THREAD_COUNT];
    let mut debugger = Debugger(None);
    {
        let watched = lock(&WATCHED);
        for (t, watched) in watched.iter().enumerate() {
            let Some(watched) = watched else { continue };
            let due = sample_watched(&mut lock(&RECORDS), t, watched, &mut debugger);
            next = next.min(due.next);
            write[t] = due.write;
        }
    }
    for (t, seq) in write.into_iter().enumerate().filter(|(_, seq)| *seq != 0) {
        let mut records = lock(&RECORDS);
        let c = &records.captures[t];
        if c.active && c.seq == seq && !c.written {
            write_slot(&records, c);
            records.captures[t].written = true;
        }
    }
    if next != u64::MAX {
        ARMING.arm_at(next);
    }
}

fn describe(c: &Capture) -> Value {
    let empty = Images::default();
    let images = IMAGES.get().unwrap_or(&empty);
    let mut used = [false; MAX_IMAGES];
    let samples: Vec<Value> = c.samples[..c.count]
        .iter()
        .map(|sample| {
            let frames: Vec<Value> = sample.frames[..sample.count as usize]
                .iter()
                .map(|&frame| {
                    let (image, offset) = image_of(images, frame);
                    if image >= 0 {
                        used[image as usize] = true;
                    }
                    json!({"image": image_name(images, image), "offset": offset})
                })
                .collect();
            json!({"state": sample.run_state, "frames": frames})
        })
        .collect();
    let images: Vec<Value> = images
        .own
        .iter()
        .zip(used)
        .filter(|(_, used)| *used)
        .map(|(image, _)| json!({"name": image.name, "uuid": image.uuid}))
        .collect();
    json!({
        "id": c.seq,
        "thread": THREAD_NAMES[c.thread],
        "startedAt": c.started_ms.round() as i64,
        "durationMs": c.duration_ms.round() as i64,
        "inputWaitMs": c.input_wait_ms.round() as i64,
        "gcMs": if c.gc_ms >= 0.0 { c.gc_ms.round() as i64 } else { -1 },
        "gcCount": c.gc_count as i64,
        "thermal": c.thermal,
        "samples": samples,
        "images": images,
    })
}

fn finish(ended: &Ended, mut done: Capture) {
    done.duration_ms = ended.ms.max(1.0);
    done.input_wait_ms = ended.input_wait_ms;
    done.gc_ms = ended.gc_ms;
    done.gc_count = ended.gc_count;
    {
        let mut records = lock(&RECORDS);
        write_slot(&records, &done);
        let next = records.kept_next;
        records.kept[next] = done.clone();
        records.kept_next = (next + 1) % KEPT;
    }
    ARMING.disarm_if_idle();
    let mut record = describe(&done);
    record["watchEpoch"] = json!(ended.epoch);
    record["epoch"] = json!(done.consent);
    let emit = lock(&EMIT).clone();
    if let Some(emit) = emit {
        emit(record);
    }
}

fn thermal_state() -> i32 {
    NSProcessInfo::processInfo().thermalState().0 as i32
}

fn observe(
    name: &objc2_foundation::NSNotificationName,
    on: impl Fn() + 'static,
) -> Retained<AnyObject> {
    let block = RcBlock::new(move |_: NonNull<NSNotification>| on());
    // SAFETY: the block only touches atomics, so the posting thread does not matter.
    let token = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(name),
            None,
            None,
            &block,
        )
    };
    token.into()
}

/// `WatchActivation`; `app_active` is `NSApp.isActive`.
fn watch_activation(app_active: bool) {
    let acted_out = CONFIG.get().is_some_and(|config| config.acted_out);
    ACTIVE.store(acted_out || app_active, Ordering::SeqCst);
    THERMAL.store(thermal_state(), Ordering::Relaxed);
    // SAFETY: the notification names are framework constants.
    let observers = unsafe {
        vec![
            observe(NSApplicationDidBecomeActiveNotification, || {
                ACTIVE.store(true, Ordering::SeqCst)
            }),
            observe(NSApplicationDidResignActiveNotification, move || {
                ACTIVE.store(acted_out, Ordering::SeqCst);
            }),
            observe(NSProcessInfoThermalStateDidChangeNotification, || {
                THERMAL.store(thermal_state(), Ordering::Relaxed);
            }),
        ]
    };
    ACTIVE_OBSERVERS.with(|kept| *kept.borrow_mut() = observers);
}

extern "C" fn run_boxed(context: *mut c_void) {
    // SAFETY: `context` is the box that `post_to` made; libdispatch runs it once.
    let work = unsafe { Box::from_raw(context.cast::<Box<dyn FnOnce() + Send>>()) };
    work();
}

fn post_to(queue: usize, work: impl FnOnce() + Send + 'static) {
    let work: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(work));
    // SAFETY: `queue` is a live queue that `setup` made; `run_boxed` frees the context.
    unsafe { dispatch_async_f(queue as *mut c_void, Box::into_raw(work).cast(), run_boxed) };
}

extern "C" fn fire_handler(_: *mut c_void) {
    fire();
}

fn setup() -> &'static Queues {
    QUEUES.get_or_init(|| {
        // SAFETY: libdispatch objects made here live for the rest of the process.
        unsafe {
            let utility =
                dispatch_queue_attr_make_with_qos_class(std::ptr::null(), QOS_CLASS_UTILITY, 0);
            let fire_queue = dispatch_queue_create(c"ardia.freeze-watch.fire".as_ptr(), utility);
            let work = dispatch_queue_create(c"ardia.freeze-watch.work".as_ptr(), utility);
            let timer =
                dispatch_source_create(&raw const _dispatch_source_type_timer, 0, 0, fire_queue);
            dispatch_source_set_event_handler_f(timer, fire_handler);
            dispatch_source_set_timer(timer, DISPATCH_TIME_FOREVER, DISPATCH_TIME_FOREVER, 0);
            dispatch_resume(timer);
            post_to(work as usize, || {
                let _ = IMAGES.set(load_images());
                open_records(&mut lock(&RECORDS));
                READY.store(true, Ordering::SeqCst);
            });
            Queues {
                work: work as usize,
                timer: timer as usize,
            }
        }
    })
}

/// Gives the settings that Ardia reads from `ArdiaLaunchConfig` and `ArdiaEvents`. Call it before `watch`.
pub fn configure(config: Config) {
    let _ = CONFIG.set(config);
}

pub fn watch(thread: usize, pthread: libc::pthread_t, stretch: Option<Arc<Stretch>>) {
    setup();
    let sampled = stretch
        .as_ref()
        .and_then(|_| stack::sampled_thread(pthread));
    if let Some(sampled) = sampled {
        // SAFETY: `port` names a live thread; the extra send right is released in `unwatch` or the next `watch`.
        unsafe {
            mach_port_mod_refs(
                libc::mach_task_self(),
                sampled.port,
                MACH_PORT_RIGHT_SEND,
                1,
            )
        };
        if thread == 0 {
            main_queue::post(Box::new(|| {
                let mtm = MainThreadMarker::new().expect("the main queue runs on the main thread");
                watch_activation(NSApplication::sharedApplication(mtm).isActive());
            }));
        }
    }
    let released = {
        let mut watched = lock(&WATCHED);
        let old = std::mem::replace(
            &mut watched[thread],
            sampled
                .zip(stretch)
                .map(|(sampled, stretch)| Watched { stretch, sampled }),
        );
        clear(&mut lock(&RECORDS).captures[thread]);
        old.map(|old| old.sampled.port)
    };
    release(released);
}

fn release(port: Option<u32>) {
    if let Some(port) = port {
        // SAFETY: the right was added in `watch`.
        unsafe { mach_port_deallocate(libc::mach_task_self(), port) };
    }
}

pub fn unwatch(thread: usize, stretch: &Arc<Stretch>) {
    if QUEUES.get().is_none() {
        return;
    }
    let released = {
        let mut watched = lock(&WATCHED);
        if !watched[thread]
            .as_ref()
            .is_some_and(|watched| Arc::ptr_eq(&watched.stretch, stretch))
        {
            return;
        }
        let old = watched[thread].take();
        if stretch.start.load(Ordering::Acquire) != 0 {
            ARMING.sleep();
        }
        clear(&mut lock(&RECORDS).captures[thread]);
        old.map(|old| old.sampled.port)
    };
    release(released);
}

pub fn arm() {
    if QUEUES.get().is_some() {
        ARMING.wake(
            now_ticks(),
            ms_to_ticks(FREEZE_MS),
            ACTIVE.load(Ordering::Relaxed),
        );
    }
}

pub fn idle() {
    if QUEUES.get().is_some() {
        ARMING.sleep();
    }
}

pub fn stretch_ended(ended: Ended) {
    let Some(queues) = QUEUES.get() else {
        return;
    };
    if ended.ms < FREEZE_MS {
        return;
    }
    let done = {
        let mut records = lock(&RECORDS);
        let c = &mut records.captures[ended.thread];
        if !c.holds(ended.id, &ended.source) {
            return;
        }
        let done = c.clone();
        clear(c);
        done
    };
    post_to(queues.work, move || finish(&ended, done));
}

pub fn set_emitter(emit: impl Fn(Value) + Send + Sync + 'static) {
    let queues = setup();
    let kept: Emitter = Arc::new(emit);
    post_to(queues.work, move || *lock(&EMIT) = Some(kept));
}

pub fn annotate(freeze_id: u64, activity: &str) {
    let Some(queues) = QUEUES.get() else {
        return;
    };
    if activity.is_empty()
        || activity.len() > 24
        || !activity
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return;
    }
    let name = activity.to_owned();
    post_to(queues.work, move || {
        let mut records = lock(&RECORDS);
        for i in 0..KEPT {
            let kept = &mut records.kept[i];
            if kept.seq == freeze_id && kept.duration_ms > 0.0 {
                kept.activity.clone_from(&name);
                let kept = records.kept[i].clone();
                write_slot(&records, &kept);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Condvar;
    use std::sync::atomic::AtomicUsize;
    use std::time::{Duration, Instant};

    static STRETCH: LazyLock<Arc<Stretch>> = LazyLock::new(Arc::default);
    static EMITTED: AtomicUsize = AtomicUsize::new(0);
    static PLUG_ENTERED: AtomicBool = AtomicBool::new(false);
    static PLUG: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());
    static RECORDS_SEEN: Mutex<Vec<Value>> = Mutex::new(Vec::new());

    fn spin(ms: u64) {
        let until = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < until {}
    }

    fn run_stretch(ms: u64) {
        let id = STRETCH.id.fetch_add(1, Ordering::SeqCst) + 1;
        STRETCH.start.store(now_ticks(), Ordering::SeqCst);
        arm();
        spin(ms);
        STRETCH.start.swap(0, Ordering::SeqCst);
        idle();
        stretch_ended(Ended {
            thread: 0,
            source: STRETCH.clone(),
            id,
            ms: ms as f64,
            input_wait_ms: 0.0,
            gc_ms: -1.0,
            gc_count: -1.0,
            epoch: 1,
        });
    }

    fn wait_until(done: impl Fn() -> bool, seconds: u64) {
        let until = Instant::now() + Duration::from_secs(seconds);
        while !done() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn slot(dir: &std::path::Path, index: usize) -> String {
        let file = std::fs::read(dir.join("freezes.txt")).unwrap_or_default();
        file.get(index * SLOT_BYTES..(index + 1) * SLOT_BYTES)
            .map(|text| {
                String::from_utf8_lossy(text)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_owned()
            })
            .unwrap_or_default()
    }

    fn on_record(record: Value) {
        lock(&RECORDS_SEEN).push(record);
        if EMITTED.fetch_add(1, Ordering::SeqCst) == 0 {
            PLUG_ENTERED.store(true, Ordering::SeqCst);
            let (released, wake) = &PLUG;
            let guard = lock(released);
            let _ = wake.wait_timeout_while(guard, Duration::from_secs(20), |released| !*released);
        }
    }

    #[test]
    fn freeze_capture() {
        let data_dir =
            std::env::temp_dir().join(format!("ria-freeze-capture-{}", std::process::id()));
        std::fs::create_dir_all(&data_dir).unwrap();
        configure(Config {
            data_directory: data_dir.clone(),
            acted_out: true,
            consent_epoch: Box::new(|| "period-under-test".into()),
        });
        set_emitter(on_record);
        let worker = std::thread::spawn(|| {
            // SAFETY: `pthread_self` is this live thread.
            watch(0, unsafe { libc::pthread_self() }, Some(STRETCH.clone()));
            // The test has no main run loop, so it runs the activation step that `watch` queued for it.
            watch_activation(false);
            std::thread::sleep(Duration::from_millis(500));
            run_stretch(700);
            for _ in 0..400 {
                if PLUG_ENTERED.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            run_stretch(2300);
            run_stretch(900);
        });
        wait_until(|| worker.is_finished(), 30);
        assert!(
            PLUG_ENTERED.load(Ordering::SeqCst),
            "the first freeze's finish is holding the work queue"
        );
        *lock(&PLUG.0) = true;
        PLUG.1.notify_all();
        wait_until(|| EMITTED.load(Ordering::SeqCst) >= 3, 10);
        worker.join().unwrap();

        let records = lock(&RECORDS_SEEN).clone();
        assert_eq!(
            records.len(),
            3,
            "three freezes, three records, though the second finished only after the third began"
        );
        let second = &records[1];
        assert!(
            second["id"] == 2 && second["durationMs"].as_f64().unwrap() >= 2300.0,
            "the delayed finish reports the second freeze with its own length"
        );
        assert!(
            !second["samples"].as_array().unwrap().is_empty(),
            "the second freeze keeps its stack samples"
        );
        assert!(
            second["epoch"] == "period-under-test" && second["watchEpoch"] == 1,
            "the record carries the consent stamp from when its capture began and the watch epoch"
        );
        assert_eq!(
            slot(&data_dir, 1),
            "freeze 2 main recovered",
            "the second freeze's slot, written frozen at 2 s, now says recovered"
        );
        assert_eq!(
            slot(&data_dir, 2),
            "freeze 3 main recovered",
            "the third freeze is recorded too"
        );
        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
