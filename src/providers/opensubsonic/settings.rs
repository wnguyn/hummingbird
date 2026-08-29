//! Persisted OpenSubsonic server configuration.

use serde::{Deserialize, Serialize};

use super::{client::ServerConfig, register_client};

/// The OpenSubsonic section of the user's settings file.
///
/// A `Vec` (rather than a single server) keeps the door open for multiple
/// servers without a schema rewrite.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenSubsonicSettings {
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
}

impl OpenSubsonicSettings {
    /// Whether a server with the given ID is configured.
    #[allow(dead_code)]
    pub fn contains(&self, server_id: &str) -> bool {
        self.servers.iter().any(|s| s.id == server_id)
    }
}

/// Registers clients for every configured server with the playback-time client
/// registry. Servers that fail to construct a client (bad URL, etc.) are
/// skipped so a single misconfigured entry doesn't take the rest down.
pub fn register_servers(settings: &OpenSubsonicSettings) {
    for config in &settings.servers {
        match config.client() {
            Ok(client) => register_client(config.id.clone(), client),
            Err(e) => tracing::warn!(
                server = %config.name,
                error = %e,
                "failed to connect to OpenSubsonic server",
            ),
        }
    }
}
