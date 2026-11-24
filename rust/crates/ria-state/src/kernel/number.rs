// Ria original, no Ardia source.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A TypeScript `number`. It serializes as an integer when it has no fraction, so that the state JSON of a module
/// equals the `JSON.stringify` text of the TypeScript store (plan section 6.5).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Number(pub f64);

impl Serialize for Number {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let n = self.0;
        if n.is_finite() && n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
            serializer.serialize_i64(n as i64)
        } else if n.is_finite() {
            serializer.serialize_f64(n)
        } else {
            // `JSON.stringify` writes NaN and the infinities as `null`.
            serializer.serialize_none()
        }
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Any;
        impl Visitor<'_> for Any {
            type Value = Number;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a number")
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Number, E> {
                Ok(Number(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Number, E> {
                Ok(Number(v as f64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Number, E> {
                Ok(Number(v as f64))
            }
        }
        deserializer.deserialize_any(Any)
    }
}

impl From<f64> for Number {
    fn from(v: f64) -> Self {
        Self(v)
    }
}

impl From<i32> for Number {
    fn from(v: i32) -> Self {
        Self(f64::from(v))
    }
}

#[cfg(test)]
mod tests {
    use super::Number;

    #[test]
    fn whole_numbers_serialize_without_a_fraction() {
        assert_eq!(serde_json::to_string(&Number(5.0)).unwrap(), "5");
        assert_eq!(
            serde_json::to_string(&Number(1_700_000_000_000.0)).unwrap(),
            "1700000000000"
        );
        assert_eq!(serde_json::to_string(&Number(1.5)).unwrap(), "1.5");
        assert_eq!(serde_json::to_string(&Number(-0.0)).unwrap(), "0");
        assert_eq!(serde_json::to_string(&Number(f64::NAN)).unwrap(), "null");
    }

    #[test]
    fn integers_and_floats_deserialize() {
        assert_eq!(serde_json::from_str::<Number>("7").unwrap(), Number(7.0));
        assert_eq!(serde_json::from_str::<Number>("-3").unwrap(), Number(-3.0));
        assert_eq!(
            serde_json::from_str::<Number>("0.25").unwrap(),
            Number(0.25)
        );
        assert!(serde_json::from_str::<Number>("\"7\"").is_err());
    }
}
