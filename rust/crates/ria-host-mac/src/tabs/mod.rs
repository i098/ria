// Ria original, no Ardia source.

pub mod ardia_page_cover;
pub mod ardia_page_cursor;
pub mod ardia_page_draw_wait;
pub mod ardia_renderer_scroll_observer;
pub mod ardia_tab_lifecycle;
pub mod ardia_tab_parking;
pub mod ardia_tab_presses;
pub mod ardia_tab_registry;
pub mod ardia_tab_strip;
pub mod ardia_transfer_ledger;
pub mod ardia_web_view_context_menu;
pub mod ardia_web_view_field_providers;
pub mod ardia_web_view_first_paint;
pub mod ardia_web_view_focus;
pub mod ardia_web_view_input_monitors;
pub mod ardia_web_view_navigation;
pub mod ardia_web_view_page_commands;
pub mod ardia_web_view_page_messages;
pub mod ardia_web_view_painting;

fn superview(view: &objc2_app_kit::NSView) -> Option<objc2::rc::Retained<objc2_app_kit::NSView>> {
    // SAFETY: The live AppKit hierarchy is confined to the main thread.
    unsafe { view.superview() }
}
