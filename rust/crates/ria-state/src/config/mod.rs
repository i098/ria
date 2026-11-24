// Ported from ardia apps/browser/src/state/config/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `config` (`apps/browser/src/state/config`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.25 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LookBlocks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThemeLook {
    pub family: Option<String>,
    pub mono: String,
    pub sizes: ThemeLookSizes,
    pub weights: ThemeLookWeights,
    pub metrics: ThemeLookMetrics,
    #[serde(rename = "sidebarWidth")]
    pub sidebar_width: Option<Number>,
    #[serde(rename = "sidebarScale")]
    pub sidebar_scale: Number,
    pub radii: ThemeLookRadii,
    pub hidden: ThemeLookHidden,
    #[serde(rename = "addressField")]
    pub address_field: ThemeLookAddressField,
    #[serde(rename = "newTab")]
    pub new_tab: Option<ThemeLookNewTab>,
    pub icons: ThemeLookIcons,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThemeLookSizes {
    pub large: Number,
    pub small: Number,
    pub title: Number,
    pub badge: Number,
    pub body: Number,
    pub tiny: Number,
    pub caption: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThemeLookWeights {
    pub medium: FontWeight,
    pub bold: FontWeight,
    pub regular: FontWeight,
    pub semibold: FontWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FontWeight {
    #[default]
    #[serde(rename = "400")]
    V400,
    #[serde(rename = "500")]
    V500,
    #[serde(rename = "600")]
    V600,
    #[serde(rename = "700")]
    V700,
    #[serde(rename = "100")]
    V100,
    #[serde(rename = "200")]
    V200,
    #[serde(rename = "300")]
    V300,
    #[serde(rename = "800")]
    V800,
    #[serde(rename = "900")]
    V900,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThemeLookMetrics {
    #[serde(rename = "rowHeight")]
    pub row_height: Number,
    #[serde(rename = "rowGap")]
    pub row_gap: Number,
    #[serde(rename = "pinnedHeight")]
    pub pinned_height: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThemeLookRadii {
    pub panel: Number,
    pub toolbar: Number,
    pub control: Number,
    pub none: Number,
    pub pinned: Number,
    pub tile: Number,
    pub chrome: Number,
    pub hair: Number,
    pub sheet: Number,
    pub hero: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThemeLookHidden {
    pub size: Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ThemeLookAddressField {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ThemeLookNewTab {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThemeLookIcons {
    pub size: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BasePalette {
    pub dark: std::collections::HashMap<String, RoleValue>,
    pub light: std::collections::HashMap<String, RoleValue>,
    pub revision: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RoleValue {
    String(String),
    Number(Number),
    Bool(bool),
    Other4((String, String)),
}

impl Default for RoleValue {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RolePalettes {
    pub dark: std::collections::HashMap<String, RoleValue>,
    pub light: std::collections::HashMap<String, RoleValue>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub blocks: Option<LookBlocks>,
    pub look: ThemeLook,
    pub palette: Option<BasePalette>,
    pub theme: Option<String>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `reconcileConfig` (an exported function that writes a store).
    #[serde(rename = "reconcileConfig")]
    ReconcileConfig {
        #[serde(default)]
        launch: Option<bool>,
    },
    /// `resetConfigFilesForTests` (an exported function that writes a store).
    #[serde(rename = "resetConfigFilesForTests")]
    ResetConfigFilesForTests,
    /// `startConfigFiles` (an exported function that writes a store).
    #[serde(rename = "startConfigFiles")]
    StartConfigFiles {
        roles: RolePalettes,
        #[serde(default)]
        launch: Option<bool>,
    },
}

/// Applies `action` to `app`. Slice 2.25 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.25")
}
