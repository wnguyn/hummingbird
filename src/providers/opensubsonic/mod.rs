//! OpenSubsonic/Subsonic client support.
//!
//! A libre, HTTP-based protocol for talking to music servers such as
//! Navidrome, Airsonic, and Gonic. Gated behind `libre-services`.
use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use symphonia::core::io::MediaSource;

use crate::providers::SubsonicTrackRef;
pub mod auth;
pub mod client;
pub mod credentials;
pub mod errors;
pub mod library;
pub mod models;
pub mod playback;
pub mod settings;

#[cfg(test)]
pub mod fixtures;

#[cfg(test)]
mod tests;

pub use client::{DEFAULT_API_VERSION, DEFAULT_CLIENT_NAME, OpenSubsonicClient, ServerConfig};
pub use credentials::Secret;
pub use errors::SubsonicError;
pub use settings::{OpenSubsonicSettings, register_servers};

/// Registry of connected servers, keyed by stable server ID.
///
/// The playback thread reads from this to resolve a [`SubsonicTrackRef`] into
/// a live HTTP client without touching the UI thread or settings models.
static CLIENTS: LazyLock<RwLock<HashMap<String, OpenSubsonicClient>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Registers (or replaces) a connected server's client.
pub fn register_client(server_id: String, client: OpenSubsonicClient) {
    CLIENTS
        .write()
        .expect("poisoned opensubsonic client registry")
        .insert(server_id, client);
}

/// Removes a server's client (on disconnect/removal).
pub fn remove_client(server_id: &str) {
    CLIENTS
        .write()
        .expect("poisoned opensubsonic client registry")
        .remove(server_id);
}

/// Clears all registered clients.
#[allow(dead_code)]
pub fn clear_clients() {
    CLIENTS
        .write()
        .expect("poisoned opensubsonic client registry")
        .clear();
}

/// Looks up a server's client by ID.
pub fn client_for(server_id: &str) -> Option<OpenSubsonicClient> {
    CLIENTS
        .read()
        .expect("poisoned opensubsonic client registry")
        .get(server_id)
        .cloned()
}

/// Opens a buffered, seekable stream for a remote track.
///
/// Resolves the server's client from the registry and builds an HTTP range
/// source backed by the `stream` endpoint. `max_bit_rate` is currently `None`
/// (original quality) — transcoding is left for a future explicit setting.
pub fn open_subsonic_stream(
    track: &SubsonicTrackRef,
) -> Result<Box<dyn MediaSource>, SubsonicError> {
    let client = client_for(&track.server_id).ok_or(SubsonicError::NotConnected)?;
    let source = playback::SubsonicStreamSource::open(client, track.id.clone(), None)?;
    Ok(Box::new(source))
}
