//! Ria Phase 0, proofs A, B and C: a Rust host for the prebuilt Ardia engine.
//!
//! The binary lives in `Ria Proof.app/Contents/MacOS` beside `Contents/Frameworks/Chromium
//! Framework.framework` (see `run.sh`). It loads that framework, hands it a delegate, and the
//! engine runs the main loop. When the engine starts, `proof.rs` opens one window and one tab and
//! runs the chosen scenario.
//!
//! `abc-host --proof a|b|c [--page <url>] [--data-dir <dir>]`

#![allow(unsafe_code)]

mod layer;
mod load;
mod proof;

use std::ffi::{CString, c_char, c_int};
use std::path::PathBuf;
use std::time::Instant;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{ClassType, define_class, msg_send};
use objc2_foundation::NSString;

use proof::{Config, Kind};

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "RiaEngineDelegate"]
    struct EngineDelegate;

    impl EngineDelegate {
        #[unsafe(method(engineDidStart))]
        fn engine_did_start(&self) {
            proof::engine_started();
        }

        #[unsafe(method(engineWillShutDown))]
        fn engine_will_shut_down(&self) {
            proof::engine_will_shut_down();
        }

        #[unsafe(method(engineQuitCancelled))]
        fn engine_quit_cancelled(&self) {}

        // No extra browser windows in a proof.
        #[unsafe(method_id(engineWindowForNewBrowserOfProfile:type:))]
        fn window_for_new_browser(&self, _profile: &AnyObject, _kind: &NSString) -> Option<Retained<AnyObject>> {
            None
        }
    }
);

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn framework_binary() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // Contents/MacOS/abc-host -> Contents/Frameworks/Chromium Framework.framework
    let contents = exe.parent().and_then(|p| p.parent()).expect("exe is inside a bundle");
    contents.join("Frameworks/Chromium Framework.framework/Versions/Current/Chromium Framework")
}

fn main() {
    let started = Instant::now();
    let argv: Vec<String> = std::env::args().collect();
    // Helper processes re-enter through their own executables; this one is always the browser.
    let kind = match arg("--proof").as_deref() {
        Some("a") => Kind::A,
        Some("b") => Kind::B,
        Some("c") => Kind::C,
        other => {
            eprintln!("usage: abc-host --proof a|b|c [--page <url>] [--data-dir <dir>] (got {other:?})");
            std::process::exit(64);
        }
    };
    let page_url = arg("--page").expect("--page <url> is required");
    let data_dir = arg("--data-dir").expect("--data-dir <dir> is required");
    proof::set_config(Config { kind, page_url, started, load: arg("--load").and_then(|n| n.parse().ok()) });

    let framework = CString::new(framework_binary().to_str().expect("utf-8 path")).expect("no NUL");
    // SAFETY: loads the engine framework once, before any Objective-C class of it is used.
    let handle = unsafe { libc::dlopen(framework.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
    if handle.is_null() {
        let err = unsafe { std::ffi::CStr::from_ptr(libc::dlerror()) }.to_string_lossy().into_owned();
        eprintln!("cannot load the engine framework {}: {err}", framework.to_string_lossy());
        std::process::exit(65);
    }

    // The same switches Ardia's own host passes (ArdiaHost+Launch.mm), minus its UI-specific ones.
    let mut args: Vec<String> = vec![
        argv[0].clone(),
        format!("--user-data-dir={data_dir}/Chromium"),
        "--use-mock-keychain".into(),
        "--profile-directory=Default".into(),
        "--no-first-run".into(),
        "--enable-smooth-scrolling".into(),
        "--disable-stack-profiler".into(),
        "--disable-features=MacAppCodeSignClone,DevToolsAcceptDebuggingConnections,AutofillAiWalletPrivatePasses,SettingsGlowupJourneys,AsyncLiveResize".into(),
    ];
    // Engine switches given after `--engine` pass straight through.
    if let Some(i) = argv.iter().position(|a| a == "--engine") {
        args.extend(argv[i + 1..].iter().cloned());
    }
    let c_args: Vec<CString> = args.iter().map(|a| CString::new(a.as_str()).expect("no NUL")).collect();
    let mut ptrs: Vec<*const c_char> = c_args.iter().map(|a| a.as_ptr()).collect();
    ptrs.push(std::ptr::null());

    let delegate: Retained<AnyObject> = unsafe { msg_send![EngineDelegate::class(), new] };
    let engine_class = objc2::runtime::AnyClass::get(c"ArdiaCoreEngine").expect("ArdiaCoreEngine is in the framework");
    let code: c_int = unsafe { msg_send![engine_class, runWithArgc: (ptrs.len() - 1) as c_int, argv: ptrs.as_ptr(), delegate: &*delegate] };
    proof::emit(&format!(
        "{{\"t_ms\":{:.1},\"proof\":\"{kind:?}\",\"event\":\"engine_returned\",\"exit_code\":{code}}}",
        started.elapsed().as_secs_f64() * 1000.0
    ));
    std::process::exit(code);
}
