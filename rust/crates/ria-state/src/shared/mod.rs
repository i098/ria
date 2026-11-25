// Ria original, no Ardia source.

//! Shared helpers of Ardia (`apps/browser/src/lib`), one module for each source file.
//! `js` holds the JavaScript rules that Rust lacks; it has no Ardia source.

pub mod absolute_folder;
pub mod after_first_window;
pub mod badge_fit;
pub mod color;
pub mod custom_icon;
pub mod deadline;
pub mod decode;
pub mod dev_eval;
pub mod emoji;
pub mod emoji_data;
pub mod errors;
pub mod fallback;
pub mod hermes_heap;
pub mod hover_marks;
pub mod interface_scale;
pub mod js;
pub mod json_schema;
pub mod keys;
pub mod legacy;
pub mod link_source;
pub mod link_wrappers;
pub mod match_pattern;
pub mod press_motion;
pub mod saved_document;
pub mod sha256;
pub mod single_flight;
pub mod single_line;
pub mod tile_letter;
pub mod time;
pub mod toolbar_badge;
pub mod toolbar_row;
pub mod ui_locale;
pub mod unread;
pub mod url;
pub mod url_parts;
pub mod versions;
pub mod webloc;
