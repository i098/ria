// Ported from ardia packages/core/src/providerNetwork.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `search::provider_network`: it maps the JSON arguments of the
//! recording to typed values and gives the original's JS errors. The logic is in
//! `ria_core::search::provider_network`.

#![forbid(unsafe_code)]

use ria_core::search::provider_network;
use ria_core::search::provider_rows::JsValue;
use serde_json::Value;

/// A JSON argument as the `unknown` value the TS code sees. Lone surrogates become U+FFFD.
fn js_value(value: &Value) -> JsValue {
    if crate::js::is_undefined(value) {
        return JsValue::Undefined;
    }
    if let Some(n) = crate::js::as_number(value) {
        return JsValue::Number(n);
    }
    if let Some(units) = crate::js::string_units(value) {
        return JsValue::String(String::from_utf16_lossy(&units));
    }
    match value {
        Value::Null => JsValue::Null,
        Value::Bool(b) => JsValue::Bool(*b),
        Value::Array(items) => JsValue::Array(items.iter().map(js_value).collect()),
        Value::Object(fields) => JsValue::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), js_value(v)))
                .collect(),
        ),
        // Numbers and strings are handled above.
        Value::Number(_) | Value::String(_) => JsValue::Null,
    }
}

/// Port of `isPublicHost` (`packages/core/src/providerNetwork.ts:8`).
pub fn is_public_host(host: String) -> Result<bool, String> {
    Ok(provider_network::is_public_host(&host))
}

/// Port of `remoteRequestAllowed` (`packages/core/src/providerNetwork.ts:31`). The option is
/// read as JS reads `{ allowLoopback = false } = {}`.
pub fn remote_request_allowed(url: String, arg1: Value) -> Result<bool, String> {
    let allow = match js_value(&arg1) {
        JsValue::Undefined => JsValue::Undefined,
        JsValue::Null => {
            return Err(
                "TypeError: Cannot read properties of null (reading 'allowLoopback')".into(),
            );
        }
        options => options.get("allowLoopback").clone(),
    };
    Ok(provider_network::remote_request_allowed(
        &url,
        allow.is_truthy(),
    ))
}

/// Port of `sha256Hex` (`packages/core/src/providerNetwork.ts:72`). The text crosses as JSON so
/// that a lone surrogate reaches Rust as U+FFFD, as the original's `utf8` maps it.
pub fn sha256_hex(text: Value) -> Result<String, String> {
    let units = crate::js::string_units(&text).ok_or("TypeError: text is not iterable")?;
    Ok(provider_network::sha256_hex(&String::from_utf16_lossy(
        &units,
    )))
}

/// Port of `providerApprovalKey` (`packages/core/src/providerNetwork.ts:99`).
pub fn provider_approval_key(definition: Value) -> Result<String, String> {
    Ok(provider_network::provider_approval_key(&js_value(
        &definition,
    )))
}
