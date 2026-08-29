//! Credential persistence abstraction.
//!
//! Credentials (server passwords) are stored as plain strings on disk today —
//! matching how Hummingbird persists Last.fm/ListenBrainz tokens — but wrapped in
//! [`Secret`] so they redact in `Debug`/log output and never leak into traces.

use serde::{Deserialize, Serialize};

/// A secret value (password or token) that redacts itself in `Debug` output.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying secret for use in an authentication computation.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_redacts_in_debug() {
        let secret = Secret::new("hunter2");
        let debug = format!("{secret:?}");
        assert!(!debug.contains("hunter2"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn secret_round_trips_serde() {
        let secret = Secret::new("hunter2");
        let json = serde_json::to_string(&secret).unwrap();
        assert_eq!(json, "\"hunter2\"");
        let restored: Secret = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.expose(), "hunter2");
    }
}
