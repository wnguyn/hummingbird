#![allow(dead_code)]

//! Representative OpenSubsonic/Subsonic JSON response fixtures, shaped like
//! what Navidrome emits. Used by unit tests so no real server is required.

use super::models::{Album, Envelope, OpenSubsonicExtensions, Playlist, SearchResult3, Song};

/// `ping` success envelope.
pub const PING_OK: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "type": "navidrome",
    "serverVersion": "0.53.3",
    "openSubsonic": true
  }
}"#;

/// `ping` failure envelope (wrong credentials).
pub const PING_AUTH_FAILED: &str = r#"{
  "subsonic-response": {
    "status": "failed",
    "version": "1.16.1",
    "error": { "code": 40, "message": "Wrong username or password." }
  }
}"#;

/// A generic API error (data not found).
pub const NOT_FOUND: &str = r#"{
  "subsonic-response": {
    "status": "failed",
    "version": "1.16.1",
    "error": { "code": 70, "message": "Song not found: abc" }
  }
}"#;

pub const OPEN_SUBSONIC_EXTENSIONS: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "openSubsonicExtensions": [
      { "name": "songLyrics", "versions": [1] },
      { "name": "mediaRetrieval", "versions": [1] }
    ]
  }
}"#;

pub const GET_ARTISTS: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "artists": {
      "ignoredArticles": "The El La",
      "index": [
        { "name": "A", "artist": [ { "id": "a1", "name": "Aphex Twin" } ] },
        { "name": "B", "artist": [ { "id": "b1", "name": "Boards of Canada" } ] }
      ]
    }
  }
}"#;

pub const GET_ALBUM: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "album": {
      "id": "alb-1",
      "name": "Selected Ambient Works 85-92",
      "artist": "Aphex Twin",
      "artistId": "a1",
      "coverArt": "al-1",
      "songCount": 2,
      "duration": 420,
      "year": 1992,
      "genre": "Electronic",
      "releaseDate": { "year": 1992, "month": 2, "day": 12 },
      "song": [
        {
          "id": "s-1",
          "parent": "alb-1",
          "title": "Xtal",
          "artist": "Aphex Twin",
          "artistId": "a1",
          "album": "Selected Ambient Works 85-92",
          "albumId": "alb-1",
          "track": 1,
          "discNumber": 1,
          "coverArt": "al-1",
          "duration": 279,
          "contentType": "audio/mpeg",
          "suffix": "mp3",
          "genre": "Electronic"
        },
        {
          "id": "s-2",
          "parent": "alb-1",
          "title": "Tha",
          "artist": "Aphex Twin",
          "artistId": "a1",
          "album": "Selected Ambient Works 85-92",
          "albumId": "alb-1",
          "track": 2,
          "discNumber": 1,
          "coverArt": "al-1",
          "duration": 216,
          "contentType": "audio/mpeg",
          "suffix": "mp3",
          "genre": "Electronic"
        }
      ]
    }
  }
}"#;

pub const GET_SONG: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "song": {
      "id": "s-1",
      "parent": "alb-1",
      "title": "Xtal",
      "artist": "Aphex Twin",
      "artistId": "a1",
      "album": "Selected Ambient Works 85-92",
      "albumId": "alb-1",
      "track": 1,
      "coverArt": "al-1",
      "duration": 279,
      "contentType": "audio/mpeg",
      "suffix": "mp3"
    }
  }
}"#;

pub const GET_ALBUM_LIST2: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "albumList2": {
      "album": [
        { "id": "alb-1", "name": "Selected Ambient Works 85-92", "artist": "Aphex Twin", "artistId": "a1", "coverArt": "al-1", "songCount": 13, "duration": 4464, "year": 1992, "genre": "Electronic" },
        { "id": "alb-2", "name": "Music Has the Right to Children", "artist": "Boards of Canada", "artistId": "b1", "coverArt": "al-2", "songCount": 17, "duration": 3840, "year": 1998, "genre": "Electronic" }
      ]
    }
  }
}"#;

pub const SEARCH3: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "searchResult3": {
      "artist": [ { "id": "a1", "name": "Aphex Twin" } ],
      "album": [ { "id": "alb-1", "name": "Selected Ambient Works 85-92", "artist": "Aphex Twin", "artistId": "a1", "coverArt": "al-1" } ],
      "song": [
        { "id": "s-1", "parent": "alb-1", "title": "Xtal", "artist": "Aphex Twin", "artistId": "a1", "album": "Selected Ambient Works 85-92", "albumId": "alb-1", "duration": 279, "suffix": "mp3", "coverArt": "al-1" }
      ]
    }
  }
}"#;

pub const GET_PLAYLISTS: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "playlists": {
      "playlist": [
        { "id": "pl-1", "name": "Chill", "songCount": 2, "duration": 495, "created": "2024-01-01T00:00:00Z", "changed": "2024-01-02T00:00:00Z", "owner": "me" }
      ]
    }
  }
}"#;

pub const GET_PLAYLIST: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "playlist": {
      "id": "pl-1",
      "name": "Chill",
      "songCount": 2,
      "duration": 495,
      "created": "2024-01-01T00:00:00Z",
      "owner": "me",
      "entry": [
        { "id": "s-1", "parent": "alb-1", "title": "Xtal", "artist": "Aphex Twin", "artistId": "a1", "album": "Selected Ambient Works 85-92", "albumId": "alb-1", "duration": 279, "suffix": "mp3", "coverArt": "al-1" },
        { "id": "s-2", "parent": "alb-1", "title": "Tha", "artist": "Aphex Twin", "artistId": "a1", "album": "Selected Ambient Works 85-92", "albumId": "alb-1", "duration": 216, "suffix": "mp3", "coverArt": "al-1" }
      ]
    }
  }
}"#;

pub const GET_STARRED2: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "starred2": {
      "song": [
        { "id": "s-1", "parent": "alb-1", "title": "Xtal", "artist": "Aphex Twin", "artistId": "a1", "album": "Selected Ambient Works 85-92", "albumId": "alb-1", "duration": 279, "suffix": "mp3", "coverArt": "al-1" }
      ]
    }
  }
}"#;

pub const GET_LYRICS_BY_SONG_ID: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "lyricsList": {
      "structuredLyrics": [
        {
          "lang": "en",
          "synced": true,
          "displayArtist": "Aphex Twin",
          "displayTitle": "Xtal",
          "line": [
            { "start": 0, "value": "First line" },
            { "start": 10000, "value": "Second line" }
          ]
        }
      ]
    }
  }
}"#;

/// Parses a fixture envelope into a concrete payload type.
pub fn parse<T: serde::de::DeserializeOwned>(json: &str) -> T {
    let envelope: Envelope<T> = serde_json::from_str(json).expect("fixture must parse");
    assert_eq!(envelope.response.status, "ok", "fixture must be a success");
    envelope.response.body
}

pub fn sample_album() -> Album {
    parse::<super::models::AlbumWrapper>(GET_ALBUM).album
}

pub fn sample_song() -> Song {
    parse::<super::models::SongWrapper>(GET_SONG).song
}

pub fn sample_playlist() -> Playlist {
    parse::<super::models::PlaylistWrapper>(GET_PLAYLIST).playlist
}

pub fn sample_search() -> SearchResult3 {
    parse::<super::models::SearchResult3Wrapper>(SEARCH3).search_result3
}

/// Parses the extensions fixture into a concrete list.
pub fn sample_extensions_list() -> Vec<super::models::Extension> {
    let ext: OpenSubsonicExtensions = parse(OPEN_SUBSONIC_EXTENSIONS);
    ext.extensions
}
