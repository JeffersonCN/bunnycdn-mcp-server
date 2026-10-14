//! Wrapper that keeps the API key out of logs, errors and `Debug` output (C3).

use std::fmt;

/// A secret string. `Debug`/`Display` never print the value; use [`Secret::expose`].
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Raw value. Only the HTTP client should call this.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_hide_value() {
        let s = Secret::new("super-secret-key");
        assert!(!format!("{s:?}").contains("super-secret-key"));
        assert!(!format!("{s}").contains("super-secret-key"));
        assert_eq!(s.expose(), "super-secret-key");
    }
}
