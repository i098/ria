// Ria original, no Ardia source.

//! Calls `ria_sync::logic::canon` for the recorded cases of `packages/sync/src/canon.ts`.

#![forbid(unsafe_code)]

/// What `JSON.stringify` sees: `undefined` members vanish (`null` in arrays), NaN and the
/// infinities become `null`, and -0 becomes 0.
fn decode(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    if crate::js::is_undefined(&value) {
        return Value::Null;
    }
    if let Some(n) = crate::js::as_number(&value).filter(|_| value.is_object()) {
        return if n == 0.0 { Value::from(0) } else { Value::Null };
    }
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(decode).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(_, v)| !crate::js::is_undefined(v))
                .map(|(k, v)| (k, decode(v)))
                .collect(),
        ),
        other => other,
    }
}

/// Port of `canonical` (`packages/sync/src/canon.ts:1`).
pub fn canonical(value: serde_json::Value) -> Result<String, String> {
    Ok(ria_sync::logic::canon::canonical(&decode(value)))
}
