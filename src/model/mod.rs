//! Shared serde helpers for tolerant API models (design §8).
//!
//! Domain models live in `model/<domain>.rs`. Deserialize them from `serde_json::Value` into
//! structs whose fields are all `Option<_>`/`#[serde(default)]`, never `deny_unknown_fields`.

pub mod edgerule;
pub mod pullzone;
pub mod redact;
pub mod reference;
pub mod stats;

use serde::{Deserialize, Deserializer, Serialize};

/// A Bunny enum value: normally an integer, sometimes its name as a string.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IntOrString {
    Int(i64),
    Str(String),
}

impl IntOrString {
    /// Integer value, also for numeric strings (`"1"`).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            Self::Str(s) => s.trim().parse().ok(),
        }
    }

    /// Human name using a generated `(name, value)` table; strings that are already names are
    /// returned as-is, unknown integers as their decimal form.
    pub fn name(&self, table: &[(&'static str, u8)]) -> String {
        match self.as_i64() {
            Some(n) => table
                .iter()
                .find(|(_, v)| i64::from(*v) == n)
                .map(|(name, _)| (*name).to_string())
                .unwrap_or_else(|| n.to_string()),
            None => match self {
                Self::Str(s) => s.clone(),
                Self::Int(n) => n.to_string(),
            },
        }
    }
}

/// `deserialize_with` helper: accepts `1`, `"1"` or `null` as `Option<i64>`. Non-numeric strings
/// become `None` rather than failing the whole object (models must stay tolerant).
///
/// ```ignore
/// #[serde(default, deserialize_with = "crate::model::int_or_string")]
/// pub r#type: Option<i64>,
/// ```
pub fn int_or_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    Ok(Option::<IntOrString>::deserialize(d)?.and_then(|v| v.as_i64()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Deserialize)]
    struct M {
        #[serde(default, deserialize_with = "int_or_string")]
        t: Option<i64>,
    }

    #[test]
    fn int_or_string_accepts_number_string_and_null() {
        let m: M = serde_json::from_value(json!({"t": 2})).unwrap();
        assert_eq!(m.t, Some(2));
        let m: M = serde_json::from_value(json!({"t": "3"})).unwrap();
        assert_eq!(m.t, Some(3));
        let m: M = serde_json::from_value(json!({"t": null})).unwrap();
        assert_eq!(m.t, None);
        let m: M = serde_json::from_value(json!({})).unwrap();
        assert_eq!(m.t, None);
        let m: M = serde_json::from_value(json!({"t": "Volume"})).unwrap();
        assert_eq!(m.t, None);
    }

    #[test]
    fn int_or_string_names() {
        const T: &[(&str, u8)] = &[("Premium", 0), ("Volume", 1)];
        assert_eq!(IntOrString::Int(1).name(T), "Volume");
        assert_eq!(IntOrString::Str("0".into()).name(T), "Premium");
        assert_eq!(IntOrString::Str("Volume".into()).name(T), "Volume");
        assert_eq!(IntOrString::Int(9).name(T), "9");
    }
}
