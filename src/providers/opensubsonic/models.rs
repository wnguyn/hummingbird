//! Serde models for OpenSubsonic/Subsonic JSON responses.
//!
//! All fields that other implementations may omit are `Option` (or defaulted),
//! so a server that implements only plain Subsonic still deserializes cleanly.

use serde::Deserialize;

/// The `subsonic-response` envelope wrapping every endpoint response.
#[derive(Deserialize)]
pub struct Envelope<T> {
    #[serde(rename = "subsonic-response")]
    pub response: Response<T>,
}

/// Common response metadata plus the endpoint-specific payload.
#[derive(Deserialize)]
pub struct Response<T> {
    pub status: String,
    pub version: Option<String>,
    #[serde(rename = "type")]
    pub server_type: Option<String>,
    #[serde(rename = "serverVersion")]
    pub server_version: Option<String>,
    #[serde(rename = "openSubsonic")]
    pub open_subsonic: Option<bool>,
    pub error: Option<ApiError>,
    #[serde(flatten)]
    pub body: T,
}

/// The `error` element present on failed responses.
#[derive(Deserialize, Clone, Debug)]
pub struct ApiError {
    pub code: i32,
    #[serde(default)]
    pub message: Option<String>,
}

/// Payload for endpoints that return no body (e.g. `ping`).
#[derive(Deserialize, Default)]
pub struct Empty {}

/// `getArtists` — a list of letter indices, each with its artists.
#[derive(Deserialize, Default)]
pub struct Artists {
    #[serde(default)]
    pub index: Vec<ArtistIndex>,
}

#[derive(Deserialize)]
pub struct ArtistIndex {
    pub name: String,
    #[serde(default)]
    pub artist: Vec<Artist>,
}

/// A full artist as returned by `getArtists`, `getArtist`, and search results.
#[derive(Deserialize, Clone, Debug)]
pub struct Artist {
    pub id: String,
    pub name: String,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
    #[serde(rename = "artistImageUrl")]
    pub artist_image_url: Option<String>,
    #[serde(rename = "albumCount")]
    pub album_count: Option<i64>,
    #[serde(default)]
    pub album: Vec<Album>,
}

/// A full album as returned by `getAlbum`, `getAlbumList2`, and search results.
#[derive(Deserialize, Clone, Debug)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    #[serde(rename = "artistId")]
    pub artist_id: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
    #[serde(rename = "songCount")]
    pub song_count: Option<i64>,
    pub duration: Option<i64>,
    pub created: Option<String>,
    pub year: Option<i64>,
    pub genre: Option<String>,
    #[serde(default)]
    pub song: Vec<Song>,
    #[serde(rename = "releaseDate")]
    pub release_date: Option<ReleaseDate>,
    #[serde(default)]
    pub genres: Vec<Genre>,
}

/// A song/track.
#[derive(Deserialize, Clone, Debug)]
pub struct Song {
    pub id: String,
    /// Album (directory) ID this song belongs to.
    pub parent: Option<String>,
    pub title: String,
    #[serde(rename = "isDir")]
    pub is_dir: Option<bool>,
    pub artist: Option<String>,
    #[serde(rename = "artistId")]
    pub artist_id: Option<String>,
    #[serde(default)]
    pub artists: Vec<ArtistIdRef>,
    pub album: Option<String>,
    #[serde(rename = "albumId")]
    pub album_id: Option<String>,
    pub track: Option<i64>,
    #[serde(rename = "discNumber")]
    pub disc_number: Option<i64>,
    pub year: Option<i64>,
    pub genre: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
    pub size: Option<i64>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub suffix: Option<String>,
    /// Duration in seconds.
    pub duration: Option<i64>,
    #[serde(rename = "bitRate")]
    pub bit_rate: Option<i64>,
    pub path: Option<String>,
    #[serde(rename = "playCount")]
    pub play_count: Option<i64>,
    pub created: Option<String>,
    pub starred: Option<String>,
    #[serde(rename = "userRating")]
    pub user_rating: Option<i64>,
    #[serde(rename = "playbackTime")]
    pub playback_time: Option<i64>,
}

/// OpenSubsonic artist reference embedded in songs/albums.
#[derive(Deserialize, Clone, Debug)]
pub struct ArtistIdRef {
    pub id: String,
    pub name: String,
}

/// `getAlbumList2` — a list of albums.
#[derive(Deserialize, Default)]
pub struct AlbumList2 {
    #[serde(default)]
    pub album: Vec<Album>,
}

/// `search3` results.
#[derive(Deserialize, Default)]
pub struct SearchResult3 {
    #[serde(default)]
    pub artist: Vec<Artist>,
    #[serde(default)]
    pub album: Vec<Album>,
    #[serde(default)]
    pub song: Vec<Song>,
}

/// `getPlaylists` — server playlists.
#[derive(Deserialize, Default)]
pub struct Playlists {
    #[serde(default)]
    pub playlist: Vec<Playlist>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub owner: Option<String>,
    pub public: Option<bool>,
    #[serde(rename = "songCount")]
    pub song_count: i64,
    pub duration: i64,
    pub created: String,
    pub changed: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
    #[serde(default)]
    pub entry: Vec<Song>,
}

/// `getStarred2` results.
#[derive(Deserialize, Default)]
pub struct Starred2 {
    #[serde(default)]
    pub artist: Vec<Artist>,
    #[serde(default)]
    pub album: Vec<Album>,
    #[serde(default)]
    pub song: Vec<Song>,
}

/// `getGenres` — genre list.
#[derive(Deserialize, Default)]
pub struct Genres {
    #[serde(default)]
    pub genre: Vec<Genre>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Genre {
    pub value: String,
    #[serde(rename = "songCount")]
    pub song_count: Option<i64>,
    #[serde(rename = "albumCount")]
    pub album_count: Option<i64>,
}

/// `getOpenSubsonicExtensions` — supported extensions.
#[derive(Deserialize, Default)]
pub struct OpenSubsonicExtensions {
    #[serde(rename = "openSubsonicExtensions", default)]
    pub extensions: Vec<Extension>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Extension {
    pub name: String,
    #[serde(default)]
    pub versions: Vec<i32>,
}

/// `getLyricsBySongId` — structured lyrics.
#[derive(Deserialize, Default)]
pub struct LyricsList {
    #[serde(rename = "structuredLyrics", default)]
    pub structured_lyrics: Vec<StructuredLyrics>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StructuredLyrics {
    #[serde(rename = "displayArtist")]
    pub display_artist: Option<String>,
    #[serde(rename = "displayTitle")]
    pub display_title: Option<String>,
    pub lang: Option<String>,
    pub synced: Option<bool>,
    #[serde(default)]
    pub line: Vec<LyricsLine>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LyricsLine {
    pub start: Option<i64>,
    pub value: Option<String>,
}

/// OpenSubsonic structured release date.
#[derive(Deserialize, Clone, Debug)]
pub struct ReleaseDate {
    pub year: i64,
    pub month: Option<i64>,
    pub day: Option<i64>,
}

/// `getLyrics` — legacy plain-text lyrics.
#[derive(Deserialize, Default)]
pub struct Lyrics {
    #[serde(default)]
    pub value: Option<String>,
}
