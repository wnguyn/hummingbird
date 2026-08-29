//! Tests covering JSON deserialization of representative Navidrome responses,
//! Subsonic error classification, model mapping, and identity collision safety.

use crate::providers::{
    PlaybackSource, SubsonicTrackRef,
    opensubsonic::{
        fixtures,
        library::song_to_track_ref,
        models::{Envelope, Empty},
    },
};

#[test]
fn deserializes_ping_success() {
    let envelope: Envelope<Empty> = serde_json::from_str(fixtures::PING_OK).unwrap();
    assert_eq!(envelope.response.status, "ok");
    assert_eq!(envelope.response.server_type.as_deref(), Some("navidrome"));
    assert!(envelope.response.open_subsonic.unwrap());
}

#[test]
fn deserializes_auth_failure_error() {
    let envelope: Envelope<Empty> = serde_json::from_str(fixtures::PING_AUTH_FAILED).unwrap();
    let error = envelope.response.error.expect("error element");
    assert_eq!(error.code, 40);
    assert_eq!(error.message.as_deref(), Some("Wrong username or password."));
}

#[test]
fn classifies_auth_and_not_found_codes() {
    let auth = crate::providers::opensubsonic::errors::SubsonicError::from_api(
        40,
        "Wrong username or password.".into(),
    );
    assert!(matches!(
        auth,
        crate::providers::opensubsonic::errors::SubsonicError::AuthenticationFailed
    ));

    let not_found =
        crate::providers::opensubsonic::errors::SubsonicError::from_api(70, "missing".into());
    assert!(matches!(
        not_found,
        crate::providers::opensubsonic::errors::SubsonicError::NotFound
    ));
}

#[test]
fn deserializes_album_with_songs() {
    let album = fixtures::sample_album();
    assert_eq!(album.id, "alb-1");
    assert_eq!(album.name, "Selected Ambient Works 85-92");
    assert_eq!(album.song.len(), 2);
    assert_eq!(album.song[0].title, "Xtal");
    assert_eq!(album.song[0].duration, Some(279));
    assert_eq!(album.song[0].suffix.as_deref(), Some("mp3"));
    assert_eq!(album.release_date.as_ref().map(|d| d.year), Some(1992));
}

#[test]
fn deserializes_search3() {
    let search = fixtures::sample_search();
    assert_eq!(search.artist.len(), 1);
    assert_eq!(search.album.len(), 1);
    assert_eq!(search.song.len(), 1);
    assert_eq!(search.song[0].id, "s-1");
}

#[test]
fn deserializes_playlist_with_entries() {
    let playlist = fixtures::sample_playlist();
    assert_eq!(playlist.name, "Chill");
    assert_eq!(playlist.song_count, 2);
    assert_eq!(playlist.entry.len(), 2);
}

#[test]
fn deserializes_artists_indices() {
    let artists: crate::providers::opensubsonic::models::Artists =
        fixtures::parse(fixtures::GET_ARTISTS);
    let names: Vec<&str> = artists
        .index
        .iter()
        .flat_map(|i| i.artist.iter())
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(names, vec!["Aphex Twin", "Boards of Canada"]);
}

#[test]
fn deserializes_starred2() {
    let starred: crate::providers::opensubsonic::models::Starred2 =
        fixtures::parse(fixtures::GET_STARRED2);
    assert_eq!(starred.song.len(), 1);
}

#[test]
fn deserializes_structured_lyrics() {
    let lyrics: crate::providers::opensubsonic::models::LyricsList =
        fixtures::parse(fixtures::GET_LYRICS_BY_SONG_ID);
    let structured = &lyrics.structured_lyrics[0];
    assert!(structured.synced.unwrap());
    assert_eq!(structured.line.len(), 2);
    assert_eq!(structured.line[0].value.as_deref(), Some("First line"));
}

#[test]
fn maps_song_to_track_ref_with_album_id_fallback() {
    let song = fixtures::sample_song();
    let track = song_to_track_ref("server-1", &song);
    assert_eq!(track.server_id, "server-1");
    assert_eq!(track.id, "s-1");
    assert_eq!(track.title, "Xtal");
    assert_eq!(track.duration, Some(279));
    assert_eq!(track.album_id.as_deref(), Some("alb-1"));
}

#[test]
fn maps_song_album_id_falls_back_to_parent() {
    let mut song = fixtures::sample_song();
    song.album_id = None;
    song.parent = Some("parent-album".to_string());
    let track = song_to_track_ref("server-1", &song);
    assert_eq!(track.album_id.as_deref(), Some("parent-album"));
}

#[test]
fn local_and_remote_identities_do_not_collide() {
    use std::path::PathBuf;

    let local = PlaybackSource::Local(PathBuf::from("/music/s-1.mp3"));
    let remote = PlaybackSource::Subsonic(SubsonicTrackRef {
        server_id: "server-1".into(),
        id: "s-1".into(),
        title: "Xtal".into(),
        artist: None,
        album: None,
        album_id: None,
        artist_id: None,
        duration: None,
        cover_art: None,
        suffix: None,
        content_type: None,
        genre: None,
        track_number: None,
        disc_number: None,
    });

    assert_ne!(local, remote);
    assert!(local.is_local());
    assert!(!remote.is_local());
    assert_eq!(remote.as_subsonic().map(|t| t.id.as_str()), Some("s-1"));
    assert_eq!(local.local_path().map(|p| p.as_path()), Some(std::path::Path::new("/music/s-1.mp3")));
}

#[test]
fn remote_identity_includes_server_id() {
    let a = SubsonicTrackRef {
        server_id: "server-1".into(),
        id: "s-1".into(),
        title: "Xtal".into(),
        artist: None,
        album: None,
        album_id: None,
        artist_id: None,
        duration: None,
        cover_art: None,
        suffix: None,
        content_type: None,
        genre: None,
        track_number: None,
        disc_number: None,
    };
    let b = SubsonicTrackRef { server_id: "server-2".into(), ..a.clone() };
    assert_ne!(a, b);
}

#[test]
fn playable_source_round_trips_serde() {
    use std::path::PathBuf;

    let source = PlaybackSource::Subsonic(SubsonicTrackRef {
        server_id: "server-1".into(),
        id: "s-1".into(),
        title: "Xtal".into(),
        artist: Some("Aphex Twin".into()),
        album: None,
        album_id: Some("alb-1".into()),
        artist_id: None,
        duration: Some(279),
        cover_art: Some("al-1".into()),
        suffix: Some("mp3".into()),
        content_type: None,
        genre: None,
        track_number: Some(1),
        disc_number: None,
    });

    let json = serde_json::to_string(&source).unwrap();
    let restored: PlaybackSource = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, source);

    let local = PlaybackSource::Local(PathBuf::from("/tmp/a.mp3"));
    let json = serde_json::to_string(&local).unwrap();
    let restored: PlaybackSource = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, local);
}

#[test]
fn queue_item_with_remote_source_round_trips() {
    use crate::playback::queue::QueueItemData;

    let json = serde_json::json!({
        "db_id": null,
        "db_album_id": null,
        "source": {
            "Subsonic": {
                "server_id": "server-1",
                "id": "s-1",
                "title": "Xtal",
                "artist": "Aphex Twin",
                "album": "Selected Ambient Works 85-92",
                "album_id": "alb-1",
                "artist_id": "a1",
                "duration": 279,
                "cover_art": "al-1",
                "suffix": "mp3",
                "content_type": "audio/mpeg",
                "genre": "Electronic",
                "track_number": 1,
                "disc_number": 1
            }
        }
    });

    let item: QueueItemData = serde_json::from_value(json).expect("valid queue item");
    let source = item.get_source();
    let track = source.as_subsonic().expect("subsonic source");
    assert_eq!(track.server_id, "server-1");
    assert_eq!(track.id, "s-1");
    assert_eq!(track.title, "Xtal");
    assert_eq!(track.duration, Some(279));

    let reserialized = serde_json::to_value(&item).unwrap();
    let again: QueueItemData = serde_json::from_value(reserialized).unwrap();
    assert_eq!(item, again);
}
