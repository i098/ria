// Ported from ardia apps/browser/src/lib/toolbarRow.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Toolbar row sizes (regular and compact) and the trailing insets of the URL field.
//!
//! Changes from Ardia:
//! - `TOOLBAR_ROWS` is a struct with `regular` and `compact` fields instead of a record.
//! - The anonymous results of `urlFieldOf` and `toolbarTrailing` are the structs [`UrlField`] and [`ToolbarTrailing`].
//! - `Math.max` keeps JavaScript's `NaN` and `-0` rules through `js_max`; truthiness of `pinnedWidth` treats 0, -0 and NaN as false.

use super::js::{js_max, js_round};

/// `ToolbarRow` (`toolbarRow.ts:1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarRow {
    pub height: f64,
    pub center: f64,
    pub button: f64,
    pub button_radius: f64,
    pub field: f64,
    pub field_radius: f64,
    pub field_button: f64,
    pub field_button_radius: f64,
    pub extension_button: f64,
}

/// The type of `TOOLBAR_ROWS`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarRows {
    pub regular: ToolbarRow,
    pub compact: ToolbarRow,
}

/// `TOOLBAR_ROWS` (`toolbarRow.ts:13`).
pub const TOOLBAR_ROWS: ToolbarRows = ToolbarRows {
    regular: ToolbarRow {
        height: 41.0,
        center: 21.2,
        button: 30.0,
        button_radius: 7.0,
        field: 30.0,
        field_radius: 8.0,
        field_button: 24.0,
        field_button_radius: 5.0,
        extension_button: 28.0,
    },
    compact: ToolbarRow {
        height: 28.0,
        center: 14.0,
        button: 22.0,
        button_radius: 4.0,
        field: 22.0,
        field_radius: 4.0,
        field_button: 18.0,
        field_button_radius: 2.0,
        extension_button: 22.0,
    },
};

/// `toolbarRowOf` (`toolbarRow.ts:18`).
pub fn toolbar_row_of(compact: bool) -> ToolbarRow {
    if compact {
        TOOLBAR_ROWS.compact
    } else {
        TOOLBAR_ROWS.regular
    }
}

/// The result of `urlFieldOf`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UrlField {
    pub top: f64,
    pub height: f64,
}

/// `urlFieldOf` (`toolbarRow.ts:20`): the URL field centred on the row.
pub fn url_field_of(row: &ToolbarRow) -> UrlField {
    UrlField {
        top: row.center - row.field / 2.0,
        height: row.field,
    }
}

/// `compactLift` (`toolbarRow.ts:22`): how far the row's centre sits above the regular one, to the nearest half.
pub fn compact_lift(row: &ToolbarRow) -> f64 {
    js_round((TOOLBAR_ROWS.regular.center - row.center) * 2.0) / 2.0
}

/// The type of `TOOLBAR_TRAILING`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarTrailingSizes {
    pub item_gap: f64,
    pub extension_icon: f64,
    pub split_glyph: f64,
}

/// `TOOLBAR_TRAILING` (`toolbarRow.ts:24`).
pub const TOOLBAR_TRAILING: ToolbarTrailingSizes = ToolbarTrailingSizes {
    item_gap: 2.0,
    extension_icon: 16.0,
    split_glyph: 15.0,
};

/// `fieldInsetOf` (`toolbarRow.ts:26`).
pub fn field_inset_of(row: &ToolbarRow) -> f64 {
    url_field_of(row).top
}

/// `fieldAloneInsetOf` (`toolbarRow.ts:28`): the field inset, at least enough to clear the card's corner.
pub fn field_alone_inset_of(row: &ToolbarRow, card_radius: f64) -> f64 {
    let top = field_inset_of(row);
    if top + row.field_radius >= card_radius {
        top
    } else {
        card_radius - row.field_radius
    }
}

/// The options of `toolbarTrailing`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarTrailingOptions {
    pub pinned_width: f64,
    pub split: bool,
    pub card_radius: Option<f64>,
}

/// The result of `toolbarTrailing`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarTrailing {
    pub field_right: f64,
    pub extensions_right: f64,
    pub split_right: f64,
    pub ink_gap: f64,
}

/// `toolbarTrailing` (`toolbarRow.ts:33`): the right insets of the field, the extensions and the split button.
pub fn toolbar_trailing(row: &ToolbarRow, o: &ToolbarTrailingOptions) -> ToolbarTrailing {
    let icon_inset = (row.extension_button - TOOLBAR_TRAILING.extension_icon) / 2.0;
    let ink_gap = TOOLBAR_TRAILING.item_gap + 2.0 * icon_inset;
    let split_inset = (row.button - TOOLBAR_TRAILING.split_glyph) / 2.0;
    let split_right = ink_gap - split_inset;
    let split_width = 2.0 * row.button;
    let split_ink = if o.split {
        split_right + split_width - split_inset
    } else {
        0.0
    };
    let next_ink = split_ink + ink_gap;
    let split_left = if o.split {
        split_right + split_width
    } else {
        0.0
    };
    let extensions_right = js_max(split_left, next_ink - icon_inset);
    let pinned = o.pinned_width != 0.0 && !o.pinned_width.is_nan();
    let field_right = if pinned {
        extensions_right + o.pinned_width + ink_gap - icon_inset
    } else if o.split {
        next_ink
    } else {
        field_alone_inset_of(row, o.card_radius.unwrap_or(0.0))
    };
    ToolbarTrailing {
        field_right,
        extensions_right,
        split_right,
        ink_gap,
    }
}
