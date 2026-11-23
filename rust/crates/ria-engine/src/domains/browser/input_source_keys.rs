// Ported from ardia packages/ardiacore/ios/InputSourceKeys.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The Japanese input source keys (Eisu and Kana), and the count of those that a web page tab consumed.
//! The call into `NSTextInputContext` that consumes the key event is AppKit code of `ria-host-mac`.

use std::sync::atomic::{AtomicUsize, Ordering};

/// `kVK_JIS_Eisu`.
pub const KEY_CODE_EISU: u16 = 0x66;
/// `kVK_JIS_Kana`.
pub const KEY_CODE_KANA: u16 = 0x68;

static WEB_KIT_TAB_KEYS: AtomicUsize = AtomicUsize::new(0);

/// Whether the key code is an input source key.
pub fn is_input_source_key(key_code: u16) -> bool {
    matches!(key_code, KEY_CODE_EISU | KEY_CODE_KANA)
}

/// Call when the host consumed an input source key in a web page tab.
pub fn note_web_kit_tab_key() {
    WEB_KIT_TAB_KEYS.fetch_add(1, Ordering::Relaxed);
}

/// The count since the last call, and resets it.
pub fn take_web_kit_tab_keys() -> usize {
    WEB_KIT_TAB_KEYS.swap(0, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_consumed_keys_and_resets_on_take() {
        take_web_kit_tab_keys();
        assert!(is_input_source_key(0x66));
        assert!(is_input_source_key(0x68));
        assert!(!is_input_source_key(0x67));
        note_web_kit_tab_key();
        note_web_kit_tab_key();
        assert_eq!(take_web_kit_tab_keys(), 2);
        assert_eq!(take_web_kit_tab_keys(), 0);
    }
}
