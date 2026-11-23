// Ported from ardia packages/ardiacore/ios/ArdiaLaunchPicture.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// The picture cache checks also use ArdiaEarlyWindowLayouts.mm.

use std::ffi::c_void;
use std::path::Path;

use objc2_foundation::{NSJSONSerialization, NSJSONWritingOptions};
use serde_json::{Map, Value, json};

use crate::core::convert::from_json;

use super::{launch_config::LaunchConfig, saved_session::SavedSession};

unsafe extern "C" {
    fn CC_SHA256(data: *const c_void, len: u32, digest: *mut u8) -> *mut u8;
}

pub(super) fn read(
    config: &LaunchConfig,
    session: &SavedSession,
    window: &Map<String, Value>,
    id: &str,
    layout: &Value,
) -> Option<Vec<u8>> {
    if layout.get("glass").is_some()
        || !matches!(layout.get("pictureContent"), Some(Value::String(_)))
    {
        return None;
    }
    let expected = layout.get("pictureContent")?.as_str()?;
    if expected.len() != 64 || content_hash(session, window, id)? != expected {
        return None;
    }
    config.read_data_file(Path::new(&file_name(id)))
}

fn file_name(id: &str) -> String {
    let mut name = String::from("launch-picture-");
    for unit in id.encode_utf16().take(80) {
        let ch = u8::try_from(unit).map_or('_', char::from);
        name.push(if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            ch
        } else {
            '_'
        });
    }
    name.push_str(".png");
    name
}

fn content_hash(session: &SavedSession, window: &Map<String, Value>, id: &str) -> Option<String> {
    let tabs = session
        .tabs
        .iter()
        .filter(|tab| tab.get("windowId").and_then(Value::as_str) == Some(id))
        .filter_map(|tab| {
            Some((
                tab.get("id")?.as_str()?.to_owned(),
                json!([
                    tab.get("title"),
                    tab.get("customTitle"),
                    tab.get("url"),
                    tab.get("favicon"),
                    tab.get("customIcon"),
                    session.pin_of_tab(tab),
                    tab.get("muted")
                ]),
            ))
        })
        .collect::<Map<String, Value>>();
    let groups: Vec<_> = session
        .groups
        .iter()
        .filter(|group| group.get("windowId").and_then(Value::as_str) == Some(id))
        .collect();
    let content = from_json(
        &json!({"window":window,"tabs":tabs,"groups":groups,"profiles":session.profiles,"settings":session.settings}),
    );
    // Serialize with Foundation because the original stores its exact digest, not a digest of equivalent JSON.
    let sorted = unsafe {
        NSJSONSerialization::dataWithJSONObject_options_error(
            &content,
            NSJSONWritingOptions::SortedKeys,
        )
    }
    .ok()?;
    let data = sorted.to_vec();
    let mut out = [0u8; 32];
    // SAFETY: the buffer lengths match the CommonCrypto SHA-256 contract.
    unsafe {
        CC_SHA256(
            data.as_ptr().cast(),
            data.len().try_into().ok()?,
            out.as_mut_ptr(),
        )
    };
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(64);
    for byte in out {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 15)]));
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::file_name;

    #[test]
    fn picture_name_stays_in_the_profile_root() {
        assert_eq!(file_name("../a-b_2🌙"), "launch-picture-___a-b_2__.png");
        assert_eq!(
            file_name(&"a".repeat(82)),
            format!("launch-picture-{}.png", "a".repeat(80))
        );
    }
}
