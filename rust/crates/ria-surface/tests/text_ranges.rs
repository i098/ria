// Ria original, no Ardia source.

#![cfg(target_os = "macos")]

use objc2_foundation::NSRange;
use ria_surface::TextInputSnapshot;

#[test]
fn reconversion_selects_complete_unicode_scalars() {
    let text = TextInputSnapshot::new("a🌏日本".to_owned(), 5..8).unwrap();
    assert_eq!(text.selected_range(), NSRange::new(3, 1));
    assert_eq!(text.substring(NSRange::new(1, 2)), Some("🌏"));
    assert_eq!(text.substring(NSRange::new(2, 1)), None);
    assert_eq!(text.substring(NSRange::new(usize::MAX, 6)), None);
}

#[test]
fn a_selection_cannot_split_a_utf8_scalar() {
    assert!(TextInputSnapshot::new("日本".to_owned(), 1..3).is_none());
}
