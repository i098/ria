// Ported from ardia apps/browser/src/lib/pluginProtocol.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The shapes that pass between the browser and a plugin: UI nodes, surfaces, pickers, toasts.
//! The TS file exports only types and constants, so it has no bridge crate.
//!
//! Changes from Ardia:
//! - String unions (`Tone`, `SurfaceKind`, `PluginCallKind`, the `op` of a message) are `String`:
//!   the checks compare them with `TONES` and `SURFACE_KINDS`, as the TS does.
//! - `unknown` is `Json`, `Record<string, unknown>` is `Fields`, `string | number` keys and badges are `Json`.
//! - An optional field `x?: T` is `Option<T>`; `x?: T | null` is `Option<Option<T>>` (`None` is
//!   absent, `Some(None)` is `null`).

use ria_core::identity::telemetry_wire::Json;

use super::decode::Fields;

/// `Tone` (`pluginProtocol.ts:1`): one of [`TONES`].
pub type Tone = String;
/// `TONES` (`pluginProtocol.ts:2`).
pub const TONES: &[&str] = &[
    "default",
    "secondary",
    "tertiary",
    "accent",
    "success",
    "warning",
    "destructive",
];

/// `PluginNodeType` (`pluginProtocol.ts:4`).
pub type PluginNodeType = String;

/// `PluginNode` (`pluginProtocol.ts:5`).
#[derive(Debug, Clone, PartialEq)]
pub struct PluginNode {
    pub r#type: PluginNodeType,
    /// `string | number`.
    pub key: Option<Json>,
    pub props: Fields,
    pub children: Vec<PluginChild>,
}

/// A child of a [`PluginNode`]: `PluginNode | string`.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginChild {
    Node(PluginNode),
    Text(String),
}

/// `SurfaceKind` (`pluginProtocol.ts:7`): one of [`SURFACE_KINDS`].
pub type SurfaceKind = String;
/// `SURFACE_KINDS` (`pluginProtocol.ts:8`).
pub const SURFACE_KINDS: &[&str] = &["sidebar", "popover", "panel", "command"];

/// `QuickPickItem` (`pluginProtocol.ts:10`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuickPickItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub badge: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub picked: Option<bool>,
}

/// `QuickPickOptions` (`pluginProtocol.ts:11`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuickPickOptions {
    pub title: Option<String>,
    pub placeholder: Option<String>,
    pub can_pick_many: Option<bool>,
}

/// `InputBoxOptions` (`pluginProtocol.ts:12`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InputBoxOptions {
    pub title: String,
    pub message: Option<String>,
    pub placeholder: Option<String>,
    pub value: Option<String>,
    pub password: Option<bool>,
    pub confirm_title: Option<String>,
}

/// `ConfirmOptions` (`pluginProtocol.ts:13`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfirmOptions {
    pub title: String,
    pub message: Option<String>,
    pub confirm_title: Option<String>,
    pub destructive: Option<bool>,
}

/// `ToastOptions` (`pluginProtocol.ts:14`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToastOptions {
    pub icon: Option<String>,
    pub tone: Option<Tone>,
}

/// `ToolbarUpdate` (`pluginProtocol.ts:15`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolbarUpdate {
    pub icon: Option<String>,
    pub label: Option<Option<String>>,
    /// `string | number | boolean | null`.
    pub badge: Option<Json>,
    pub tooltip: Option<Option<String>>,
    pub tone: Option<Tone>,
}

/// `OmniboxSuggestion` (`pluginProtocol.ts:16`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OmniboxSuggestion {
    pub title: String,
    pub url: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub badge: Option<String>,
}

/// `SurfaceMessage` (`pluginProtocol.ts:21`), with `SurfaceMountMessage`, `SurfaceUnmountMessage`
/// and `SurfaceEventMessage` (`pluginProtocol.ts:18-20`) as its variants, tagged by `surface`.
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceMessage {
    Mount {
        surface_id: String,
        view_id: String,
        kind: SurfaceKind,
        context: Option<Fields>,
    },
    Unmount {
        surface_id: String,
    },
    Event {
        surface_id: String,
        handler_id: String,
        args: Vec<Json>,
    },
}

/// `PluginCallKind` (`pluginProtocol.ts:23`): `"tool" | "command" | "omnibox" | "decide"`.
pub type PluginCallKind = String;
