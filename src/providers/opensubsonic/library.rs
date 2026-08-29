//! Typed, high-level OpenSubsonic/Subsonic endpoints.
//!
//! These wrap [`OpenSubsonicClient`] with concrete request parameters and map
//! server models into Hummingbird's [`SubsonicTrackRef`] playable identity.

use super::{
    client::OpenSubsonicClient,
    errors::SubsonicError,
    models::{
        Album, AlbumList2, Artist, Artists, Empty, Extension, Genres, Lyrics, LyricsList, OpenSubsonicExtensions,
        Playlist, Playlists, SearchResult3, Song, Starred2,
    },
};
use crate::providers::SubsonicTrackRef;

/// Maps a server [`Song`] into a self-contained [`SubsonicTrackRef`].
pub fn song_to_track_ref(server_id: &str, song: &Song) -> SubsonicTrackRef {
    SubsonicTrackRef {
        server_id: server_id.to_string(),
        id: song.id.clone(),
        title: song.title.clone(),
        artist: song.artist.clone(),
        album: song.album.clone(),
        album_id: song.album_id.clone().or_else(|| song.parent.clone()),
        artist_id: song.artist_id.clone(),
        duration: song.duration.map(|d| d.max(0) as u64),
        cover_art: song.cover_art.clone(),
        suffix: song.suffix.clone(),
        content_type: song.content_type.clone(),
        genre: song.genre.clone(),
        track_number: song.track,
        disc_number: song.disc_number,
    }
}

impl OpenSubsonicClient {
    /// `ping` — confirms connectivity and authentication.
    pub async fn ping(&self) -> Result<(), SubsonicError> {
        self.get_json::<Empty>("ping", &[]).await.map(|_| ())
    }

    /// `getOpenSubsonicExtensions` — returns the extensions the server advertises.
    pub async fn get_open_subsonic_extensions(&self) -> Result<Vec<Extension>, SubsonicError> {
        match self
            .get_json::<OpenSubsonicExtensions>("getOpenSubsonicExtensions", &[])
            .await
        {
            Ok(ext) => Ok(ext.extensions),
            // Plain Subsonic servers don't implement this; treat as no extensions.
            Err(SubsonicError::NotFound | SubsonicError::Unsupported) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// `getArtists` — all artists, flattened across letter indices.
    pub async fn get_artists(&self) -> Result<Vec<Artist>, SubsonicError> {
        let artists: Artists = self.get_json("getArtists", &[]).await?;
        Ok(artists.index.into_iter().flat_map(|i| i.artist).collect())
    }

    /// `getArtist` — a single artist including its albums.
    pub async fn get_artist(&self, id: &str) -> Result<Artist, SubsonicError> {
        self.get_json("getArtist", &[("id", id)]).await
    }

    /// `getAlbum` — a single album including its songs.
    pub async fn get_album(&self, id: &str) -> Result<Album, SubsonicError> {
        self.get_json("getAlbum", &[("id", id)]).await
    }

    /// `getSong` — a single song.
    pub async fn get_song(&self, id: &str) -> Result<Song, SubsonicError> {
        self.get_json("getSong", &[("id", id)]).await
    }

    /// `getAlbumList2` — a paginated list of albums by `type`.
    pub async fn get_album_list2(
        &self,
        list_type: &str,
        size: u32,
        offset: u32,
    ) -> Result<Vec<Album>, SubsonicError> {
        let size = size.to_string();
        let offset = offset.to_string();
        let list: AlbumList2 = self
            .get_json(
                "getAlbumList2",
                &[
                    ("type", list_type),
                    ("size", &size),
                    ("offset", &offset),
                ],
            )
            .await?;
        Ok(list.album)
    }

    /// `getGenres` — the server's genre list.
    pub async fn get_genres(&self) -> Result<Vec<super::models::Genre>, SubsonicError> {
        let genres: Genres = self.get_json("getGenres", &[]).await?;
        Ok(genres.genre)
    }

    /// `search3` — search across artists, albums, and songs.
    pub async fn search3(
        &self,
        query: &str,
        artist_count: u32,
        album_count: u32,
        song_count: u32,
    ) -> Result<SearchResult3, SubsonicError> {
        let artist_count = artist_count.to_string();
        let album_count = album_count.to_string();
        let song_count = song_count.to_string();
        self.get_json(
            "search3",
            &[
                ("query", query),
                ("artistCount", &artist_count),
                ("albumCount", &album_count),
                ("songCount", &song_count),
            ],
        )
        .await
    }

    /// `getPlaylists` — all server playlists (without entries).
    pub async fn get_playlists(&self) -> Result<Vec<Playlist>, SubsonicError> {
        let playlists: Playlists = self.get_json("getPlaylists", &[]).await?;
        Ok(playlists.playlist)
    }

    /// `getPlaylist` — a single playlist including its entries.
    pub async fn get_playlist(&self, id: &str) -> Result<Playlist, SubsonicError> {
        self.get_json("getPlaylist", &[("id", id)]).await
    }

    /// `getStarred2` — starred artists, albums, and songs.
    pub async fn get_starred2(&self) -> Result<Starred2, SubsonicError> {
        self.get_json("getStarred2", &[]).await
    }

    /// `star` — star a song/album/artist.
    pub async fn star(&self, id: &str) -> Result<(), SubsonicError> {
        self.post("star", &[("id", id)]).await
    }

    /// `unstar` — unstar a song/album/artist.
    pub async fn unstar(&self, id: &str) -> Result<(), SubsonicError> {
        self.post("unstar", &[("id", id)]).await
    }

    /// `scrobble` — report a completed play (or now-playing when
    /// `submission` is `false`).
    pub async fn scrobble(&self, id: &str, submission: bool) -> Result<(), SubsonicError> {
        let submission = if submission { "true" } else { "false" };
        self.post("scrobble", &[("id", id), ("submission", submission)])
            .await
    }

    /// `getLyricsBySongId` — OpenSubsonic structured lyrics.
    pub async fn get_lyrics_by_song_id(
        &self,
        id: &str,
    ) -> Result<Vec<super::models::StructuredLyrics>, SubsonicError> {
        let list: LyricsList = self.get_json("getLyricsBySongId", &[("id", id)]).await?;
        Ok(list.structured_lyrics)
    }

    /// `getLyrics` — legacy plain-text lyrics (fallback).
    pub async fn get_lyrics(&self, artist: &str, title: &str) -> Result<Option<String>, SubsonicError> {
        match self
            .get_json::<Lyrics>("getLyrics", &[("artist", artist), ("title", title)])
            .await
        {
            Ok(lyrics) => Ok(lyrics.value),
            Err(SubsonicError::NotFound | SubsonicError::Unsupported) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Builds the authenticated `stream` request for a track.
    ///
    /// Returns a `RequestBuilder` so the caller can attach `Range` headers and
    /// stream incrementally. `max_bit_rate` of `None` requests the original.
    pub fn stream_request(
        &self,
        id: &str,
        max_bit_rate: Option<u32>,
    ) -> Result<zed_reqwest::RequestBuilder, SubsonicError> {
        let mut params: Vec<(&str, String)> = vec![("id", id.to_string())];
        if let Some(rate) = max_bit_rate {
            params.push(("maxBitRate", rate.to_string()));
        }
        let params: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        self.request(zed_reqwest::Method::GET, "stream", &params)
    }

    /// `getCoverArt` — downloads cover art for an item.
    pub async fn fetch_cover_art(
        &self,
        id: &str,
        size: Option<u32>,
    ) -> Result<Vec<u8>, SubsonicError> {
        let response = self
            .cover_art_request(id, size)?
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .map_err(SubsonicError::Transport)?;

        if !response.status().is_success() {
            return Err(SubsonicError::NotFound);
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(SubsonicError::Transport)
    }

    /// Builds the authenticated `getCoverArt` request for an item.
    pub fn cover_art_request(
        &self,
        id: &str,
        size: Option<u32>,
    ) -> Result<zed_reqwest::RequestBuilder, SubsonicError> {
        let mut params: Vec<(&str, String)> = vec![("id", id.to_string())];
        if let Some(size) = size {
            params.push(("size", size.to_string()));
        }
        let params: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        self.request(zed_reqwest::Method::GET, "getCoverArt", &params)
    }
}
