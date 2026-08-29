//! Reports playback to an OpenSubsonic server via the `scrobble` endpoint.
//!
//! Reuses the existing media-metadata broadcast (MMBS) infrastructure: it
//! receives the current [`PlaybackSource`] and, for remote Subsonic tracks,
//! submits a "now playing" (`submission=false`) on track change and a completed
//! scrobble (`submission=true`) once the normal playback threshold is met. Local
//! tracks are ignored, so this never interferes with existing behaviour.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, warn};

use crate::{
    media::metadata::Metadata,
    playback::thread::PlaybackState,
    providers::{PlaybackSource, opensubsonic},
};

use super::MediaMetadataBroadcastService;

pub const MMBS_KEY: &str = "subsonic";

#[derive(Default)]
pub struct SubsonicScrobbler {
    server_id: Option<String>,
    track_id: Option<String>,
    duration: u64,
    accumulated_time: u64,
    last_position: u64,
    should_scrobble: bool,
}

impl SubsonicScrobbler {
    async fn scrobble(&mut self) {
        let (Some(server_id), Some(track_id)) =
            (self.server_id.as_deref(), self.track_id.as_deref())
        else {
            return;
        };
        let Some(client) = opensubsonic::client_for(server_id) else {
            return;
        };
        if let Err(e) = client.scrobble(track_id, true).await {
            warn!("could not scrobble to server: {e}");
        }
        self.should_scrobble = false;
    }
}

#[async_trait]
impl MediaMetadataBroadcastService for SubsonicScrobbler {
    async fn new_track(&mut self, source: PlaybackSource) {
        if self.should_scrobble {
            self.scrobble().await;
        }

        match source {
            PlaybackSource::Subsonic(track) => {
                self.server_id = Some(track.server_id.clone());
                self.track_id = Some(track.id.clone());
                self.duration = track.duration.unwrap_or(0);
            }
            PlaybackSource::Local(_) => {
                self.server_id = None;
                self.track_id = None;
                self.duration = 0;
            }
        }

        self.accumulated_time = 0;
        self.last_position = 0;
        self.should_scrobble = false;

        // Submit "now playing" to the server.
        let (Some(server_id), Some(track_id)) =
            (self.server_id.as_deref(), self.track_id.as_deref())
        else {
            return;
        };
        if let Some(client) = opensubsonic::client_for(server_id)
            && let Err(e) = client.scrobble(track_id, false).await
        {
            debug!("could not submit now playing: {e}");
        }
    }

    async fn metadata_recieved(&mut self, _info: Arc<Metadata>) {}

    async fn state_changed(&mut self, state: PlaybackState) {
        if self.should_scrobble && state != PlaybackState::Playing {
            self.scrobble().await;
        }
    }

    async fn position_changed(&mut self, position: u64) {
        if position < self.last_position + 2 && position > self.last_position {
            self.accumulated_time += position - self.last_position;
        }
        self.last_position = position;

        if self.duration >= 30
            && (self.accumulated_time > self.duration / 2 || self.accumulated_time > 240)
            && !self.should_scrobble
            && self.track_id.is_some()
        {
            self.should_scrobble = true;
        }
    }

    async fn duration_changed(&mut self, duration: u64) {
        self.duration = duration;
    }
}
