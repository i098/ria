// Ported from ardia packages/core/src/webStore.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `url::web_store`. The logic is in `ria_core::url::web_store`.

#![forbid(unsafe_code)]

use ria_core::url::web_store::{self, ExtensionPermissions};
use serde_json::Value;

/// The strings of a JS array; other items cannot equal a permission or match a host pattern.
fn strings(value: &Value) -> Vec<String> {
    value.as_array().map_or_else(Vec::new, |items| {
        items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    })
}

/// Port of `isExtensionId` (`packages/core/src/webStore.ts:3`).
pub fn is_extension_id(s: String) -> Result<bool, String> {
    Ok(web_store::is_extension_id(&s))
}

/// Port of `webStoreExtensionId` (`packages/core/src/webStore.ts:5`).
pub fn web_store_extension_id(url_or_id: String) -> Result<Option<String>, String> {
    Ok(web_store::web_store_extension_id(&url_or_id))
}

/// Port of `permissionWarnings` (`packages/core/src/webStore.ts:21`).
pub fn permission_warnings(pkg: Value) -> Result<Vec<String>, String> {
    if pkg.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'permissions')".into());
    }
    // `new Set(pkg.permissions)`: a missing value is an empty set; a non-array adds no permission name.
    let permissions = strings(pkg.get("permissions").unwrap_or(&Value::Null));
    let Some(hosts) = pkg.get("hostPermissions").filter(|h| h.is_array()) else {
        return Err("TypeError: Cannot read properties of undefined (reading 'filter')".into());
    };
    let pkg = ExtensionPermissions {
        permissions,
        host_permissions: strings(hosts),
    };
    Ok(web_store::permission_warnings(&pkg))
}
