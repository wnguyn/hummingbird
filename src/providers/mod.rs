//! Playable/library source abstraction.
//!
//! Hummingbird's playback pipeline was originally built around local files and
//! identified every playable item by a `PathBuf`. Remote sources (OpenSubsonic
//! servers, and in the future Jellyfin or WebDAV) do not have a local path, so
//! queue items and "now playing" state now carry a [`PlaybackSource`] instead.
//!
//! A remote item is identified by the stable `(server_id, object_id)` pair
//! rather than by artist/album/title strings, so it can never collide with a
//! local track or with another server's item that happens to share a name.

use std::{fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(feature = "libre-services")]
pub mod opensubsonic;

/// Identifies where a playable item's audio lives.
///
/// This is the single abstraction that lets the rest of the player treat local
/// files and remote OpenSubsonic tracks uniformly. Serialization is stable, so
/// queue and session state survive restarts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaybackSource {
    /// A track backed by a file on disk (the original local library).
    Local(PathBuf),
    /// A track streamed from an OpenSubsonic-compatible server.
    Subsonic(Box<SubsonicTrackRef>),
}

impl PlaybackSource {
    /// Whether this source is a local file.
    #[allow(dead_code)]
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    /// The local path, if this is a local source.
    pub fn local_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Local(path) => Some(path),
            Self::Subsonic(_) => None,
        }
    }

    /// The remote track reference, if this is a Subsonic source.
    pub fn as_subsonic(&self) -> Option<&SubsonicTrackRef> {
        match self {
            Self::Subsonic(track) => Some(track.as_ref()),
            Self::Local(_) => None,
        }
    }

    /// Whether the item can be played. Local items must exist on disk; remote
    /// items are always considered playable (the server is consulted at
    /// playback time and errors are surfaced then).
    pub fn is_playable(&self) -> bool {
        match self {
            Self::Local(path) => path.exists(),
            Self::Subsonic(_) => true,
        }
    }
}

impl fmt::Display for PlaybackSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(path) => write!(f, "{}", path.display()),
            Self::Subsonic(track) => write!(f, "{}", track.title),
        }
    }
}

/// Self-contained reference to a track on an OpenSubsonic server.
///
/// This carries everything needed to play and display the track without a
/// database round-trip or a further API call, while deliberately *excluding*
/// credentials and authenticated stream URLs (those are resolved from the
/// server configuration by `server_id` at playback time). Keeping this type in
/// the feature-agnostic `providers` module means the playback pipeline
/// compiles even when online services are disabled.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubsonicTrackRef {
    /// Stable identifier of the server this track belongs to.
    pub server_id: String,
    /// The server's ID for the track.
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub artist_id: Option<String>,
    /// Track duration in seconds, when known.
    pub duration: Option<u64>,
    /// Cover-art identifier for `getCoverArt`, when known.
    pub cover_art: Option<String>,
    /// File suffix (e.g. `mp3`) used as a decoder hint.
    pub suffix: Option<String>,
    /// MIME type reported by the server, when known.
    pub content_type: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
}

#[allow(dead_code)]
impl SubsonicTrackRef {
    /// Display name of the track, falling back to the server track ID.
    pub fn display_title(&self) -> &str {
        &self.title
    }
}

/// Opens a buffered, seekable stream for a remote OpenSubsonic track.
///
/// This is the single bridge between the feature-agnostic playback pipeline and
/// the `libre-services`-gated OpenSubsonic client. When online services are
/// disabled, remote playback reports an error instead of failing to compile.
#[cfg(feature = "libre-services")]
pub fn open_subsonic_stream(
    track: &SubsonicTrackRef,
) -> Result<Box<dyn symphonia::core::io::MediaSource>, String> {
    opensubsonic::open_subsonic_stream(track).map_err(|e| e.to_string())
}

#[cfg(not(feature = "libre-services"))]
pub fn open_subsonic_stream(
    _track: &SubsonicTrackRef,
) -> Result<Box<dyn symphonia::core::io::MediaSource>, String> {
    Err("remote playback requires an online-enabled build".to_string())
}

/// Downloads cover art from a connected OpenSubsonic server.
#[cfg(feature = "libre-services")]
pub async fn fetch_cover_art(
    server_id: &str,
    cover_art: &str,
    size: Option<u32>,
) -> Result<Vec<u8>, String> {
    let client =
        opensubsonic::client_for(server_id).ok_or_else(|| "server not connected".to_string())?;
    client
        .fetch_cover_art(cover_art, size)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(feature = "libre-services"))]
pub async fn fetch_cover_art(
    _server_id: &str,
    _cover_art: &str,
    _size: Option<u32>,
) -> Result<Vec<u8>, String> {
    Err("remote artwork requires an online-enabled build".to_string())
}
