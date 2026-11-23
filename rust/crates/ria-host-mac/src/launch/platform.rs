// Ported from ardia packages/ardiacore/ios/ArdiaHost+Launch.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// The signature checks also use ArdiaLaunchConfig.mm.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::io;
use std::path::Path;
use std::ptr::null_mut;

use objc2_foundation::NSString;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecCodeCopySelf(flags: u32, code: *mut *mut c_void) -> i32;
    fn SecRequirementCreateWithString(
        text: *const c_void,
        flags: u32,
        requirement: *mut *mut c_void,
    ) -> i32;
    fn SecCodeCheckValidity(code: *mut c_void, flags: u32, requirement: *mut c_void) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: *const c_void);
}

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

pub(super) fn signed_for_distribution() -> bool {
    fn check(code: *mut c_void, text: &str) -> Option<bool> {
        let text = NSString::from_str(text);
        let mut requirement = null_mut();
        // SAFETY: NSString bridges to CFString. The Create call gives us one owned reference.
        unsafe {
            if SecRequirementCreateWithString(
                (&*text as *const NSString).cast(),
                0,
                &mut requirement,
            ) != 0
            {
                return None;
            }
            let valid = SecCodeCheckValidity(code, 0, requirement) == 0;
            CFRelease(requirement);
            Some(valid)
        }
    }
    let mut code = null_mut();
    // SAFETY: the location is writable. CopySelf gives us one owned reference on success.
    if unsafe { SecCodeCopySelf(0, &mut code) } != 0 {
        return true;
    }
    let distribution = check(code, "anchor apple generic and (certificate leaf[field.1.2.840.113635.100.6.1.9] exists or certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists)").unwrap_or(true);
    // SAFETY: balance the owned CopySelf result.
    unsafe { CFRelease(code) };
    distribution
}

pub(super) fn load_engine(executable: &Path) -> io::Result<()> {
    let contents = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| io::Error::other("Ria must run from its macOS app bundle"))?;
    let path = contents
        .join("Frameworks/Chromium Framework.framework/Versions/Current/Chromium Framework");
    let c_path = CString::new(path.as_os_str().as_encoded_bytes())?;
    const RTLD_NOW: c_int = 2;
    const RTLD_GLOBAL: c_int = 8;
    // SAFETY: the path is NUL-terminated. Keep the framework loaded for the life of AppKit.
    unsafe {
        if dlopen(c_path.as_ptr(), RTLD_NOW | RTLD_GLOBAL).is_null() {
            let error = dlerror();
            let reason = if error.is_null() {
                "unknown loader error".into()
            } else {
                CStr::from_ptr(error).to_string_lossy()
            };
            return Err(io::Error::other(format!(
                "cannot load {}: {reason}",
                path.display()
            )));
        }
    }
    Ok(())
}
