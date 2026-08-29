//! Remote (OpenSubsonic) library browsing view.
//!
//! Reuses Hummingbird's existing components and styling to browse a connected
//! server's albums, artists, and search results. Playback goes through the
//! normal queue/playback pipeline by way of [`SubsonicTrackRef`] queue items.
use std::collections::HashSet;

use cntp_i18n::tr;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, FontWeight, IntoElement, ObjectFit,
    ParentElement, Render, SharedString, Styled, Window, div, px,
};
use gpui::prelude::FluentBuilder;

use crate::{
    playback::queue::QueueItemData,
    providers::{
        SubsonicTrackRef,
        opensubsonic::{
            self,
            library::song_to_track_ref,
            models::{Album, Artist, SearchResult3, Song},
        },
    },
    ui::{
        components::{
            button::button,
            icons::{icon, STAR, STAR_FILLED},
            managed_image::{ManagedImageKey, managed_image},
            textbox::Textbox,
        },
        library::context_menus::play_now,
        theme::Theme,
    },
};

/// The active screen of the remote library.
#[derive(Clone, PartialEq)]
enum RemoteScreen {
    Loading,
    Albums {
        albums: Vec<Album>,
        error: Option<SharedString>,
    },
    Artists {
        artists: Vec<Artist>,
        error: Option<SharedString>,
    },
    Search {
        songs: Vec<Song>,
        error: Option<SharedString>,
    },
    Album {
        album: Album,
        error: Option<SharedString>,
    },
}

pub struct RemoteLibrary {
    server_id: String,
    screen: Entity<RemoteScreen>,
    search_input: Entity<Textbox>,
    focus_handle: FocusHandle,
    starred: Entity<HashSet<String>>,
}

impl RemoteLibrary {
    pub fn new(cx: &mut App, server_id: String) -> Entity<Self> {
        let search_input = Textbox::new_with_submit(cx, Default::default(), |_| {});
        cx.new(|cx| {
            let screen = cx.new(|_| RemoteScreen::Loading);
            let starred = cx.new(|_| HashSet::new());
            cx.observe(&screen, |_, _, cx| cx.notify()).detach();

            let library = Self {
                server_id,
                screen,
                search_input,
                focus_handle: cx.focus_handle(),
                starred,
            };
            library.load_albums(cx);
            library
        })
    }

    fn load_albums(&self, cx: &mut App) {
        self.load_loading(cx);
        let server_id = self.server_id.clone();
        let screen = self.screen.clone();
        cx.spawn(async move |cx| {
            let result = fetch_albums(&server_id).await;
            screen.update(cx, |s, cx| {
                *s = match result {
                    Ok(albums) => RemoteScreen::Albums { albums, error: None },
                    Err(e) => RemoteScreen::Albums {
                        albums: Vec::new(),
                        error: Some(e.into()),
                    },
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn load_artists(&self, cx: &mut App) {
        self.load_loading(cx);
        let server_id = self.server_id.clone();
        let screen = self.screen.clone();
        cx.spawn(async move |cx| {
            let result = fetch_artists(&server_id).await;
            screen.update(cx, |s, cx| {
                *s = match result {
                    Ok(artists) => RemoteScreen::Artists {
                        artists,
                        error: None,
                    },
                    Err(e) => RemoteScreen::Artists {
                        artists: Vec::new(),
                        error: Some(e.into()),
                    },
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn search(&self, cx: &mut App, query: SharedString) {
        let server_id = self.server_id.clone();
        let screen = self.screen.clone();
        cx.spawn(async move |cx| {
            let result = fetch_search(&server_id, &query).await;
            screen.update(cx, |s, cx| {
                *s = match result {
                    Ok(songs) => RemoteScreen::Search { songs, error: None },
                    Err(e) => RemoteScreen::Search {
                        songs: Vec::new(),
                        error: Some(e.into()),
                    },
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn open_album(&self, cx: &mut App, album_id: String) {
        self.load_loading(cx);
        let server_id = self.server_id.clone();
        let screen = self.screen.clone();
        let starred = self.starred.clone();
        cx.spawn(async move |cx| {
            let result = fetch_album(&server_id, &album_id).await;
            screen.update(cx, |s, cx| {
                *s = match result {
                    Ok(album) => {
                        starred.update(cx, |set, cx| {
                            set.clear();
                            set.extend(album.song.iter().filter(|s| s.starred.is_some()).map(|s| s.id.clone()));
                            cx.notify();
                        });
                        RemoteScreen::Album { album, error: None }
                    }
                    Err(e) => RemoteScreen::Album {
                        album: empty_album(album_id),
                        error: Some(e.into()),
                    },
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_star(&self, cx: &mut App, song: &Song) {
        let server_id = self.server_id.clone();
        let song_id = song.id.clone();
        let starred = self.starred.clone();
        let now_starred = song.starred.is_some();
        let client = opensubsonic::client_for(&server_id);

        // Optimistically toggle the overlay.
        starred.update(cx, |set, cx| {
            if now_starred {
                set.remove(&song_id);
            } else {
                set.insert(song_id.clone());
            }
            cx.notify();
        });

        cx.spawn(async move |cx| {
            let result = match client {
                Some(client) if now_starred => client.unstar(&song_id).await.map_err(|e| e.to_string()),
                Some(client) => client.star(&song_id).await.map_err(|e| e.to_string()),
                None => Err("server not connected".to_string()),
            };

            if let Err(_) = result {
                // Roll back on failure.
                starred.update(cx, |set, cx| {
                    if now_starred {
                        set.insert(song_id.clone());
                    } else {
                        set.remove(&song_id);
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn load_loading(&self, cx: &mut App) {
        self.screen.update(cx, |s, cx| {
            *s = RemoteScreen::Loading;
            cx.notify();
        });
    }
}

fn empty_album(id: String) -> Album {
    Album {
        id,
        name: String::new(),
        artist: None,
        artist_id: None,
        cover_art: None,
        song_count: None,
        duration: None,
        created: None,
        year: None,
        genre: None,
        song: Vec::new(),
        release_date: None,
        genres: Vec::new(),
    }
}

async fn fetch_albums(server_id: &str) -> Result<Vec<Album>, String> {
    let client =
        opensubsonic::client_for(server_id).ok_or_else(|| "server not connected".to_string())?;
    client
        .get_album_list2("alphabeticalByName", 200, 0)
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_artists(server_id: &str) -> Result<Vec<Artist>, String> {
    let client =
        opensubsonic::client_for(server_id).ok_or_else(|| "server not connected".to_string())?;
    client.get_artists().await.map_err(|e| e.to_string())
}

async fn fetch_search(server_id: &str, query: &str) -> Result<Vec<Song>, String> {
    let client =
        opensubsonic::client_for(server_id).ok_or_else(|| "server not connected".to_string())?;
    let result: SearchResult3 = client
        .search3(query, 20, 20, 50)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.song)
}

async fn fetch_album(server_id: &str, album_id: &str) -> Result<Album, String> {
    let client =
        opensubsonic::client_for(server_id).ok_or_else(|| "server not connected".to_string())?;
    client.get_album(album_id).await.map_err(|e| e.to_string())
}

fn play_remote_song(cx: &mut App, server_id: &str, song: &Song) {
    let track: SubsonicTrackRef = song_to_track_ref(server_id, song);
    let item = QueueItemData::new_subsonic(cx, track);
    play_now(cx, item);
}

impl Render for RemoteLibrary {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let server_id = self.server_id.clone();
        let screen = self.screen.read(cx).clone();

        let header = div()
            .flex()
            .gap(px(8.0))
            .child(
                button()
                    .id("remote-albums")
                    .child(tr!("ALBUMS", "Albums"))
                    .on_click({
                        let this = cx.entity();
                        move |_, _, cx| this.update(cx, |this, cx| this.load_albums(cx))
                    }),
            )
            .child(
                button()
                    .id("remote-artists")
                    .child(tr!("ARTISTS", "Artists"))
                    .on_click({
                        let this = cx.entity();
                        move |_, _, cx| this.update(cx, |this, cx| this.load_artists(cx))
                    }),
            )
            .child(
                button()
                    .id("remote-search")
                    .child(tr!("SEARCH", "Search"))
                    .on_click({
                        let this = cx.entity();
                        move |_, _, cx| this.update(cx, |this, cx| {
                            let query = this.search_input.read(cx).value(cx);
                            this.search(cx, query);
                        })
                    }),
            );

        let body: gpui::AnyElement = match screen {
            RemoteScreen::Loading => div()
                .text_sm()
                .text_color(theme.text_secondary)
                .child(tr!("LOADING", "Loading…"))
                .into_any_element(),
            RemoteScreen::Albums { albums, error } => {
                let mut list = div().flex().flex_col().gap(px(4.0));
                if let Some(error) = error {
                    list = list.child(div().text_sm().text_color(theme.status_error).child(error));
                }
                for album in albums {
                    let server_id = server_id.clone();
                    let album_id = album.id.clone();
                    let cover_key = album.cover_art.clone().map(|cover| {
                        ManagedImageKey::RemoteCoverArt {
                            server_id: server_id.clone(),
                            cover_art: cover,
                        }
                    });
                    list = list.child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .p(px(4.0))
                            .cursor_pointer()
                            .hover(|this| this.bg(theme.list_item_hover))
                            .when_some(cover_key, |this, key| {
                                this.child(
                                    managed_image((album_id.clone(), "remote-album-art"), key)
                                        .w(px(36.0))
                                        .h(px(36.0))
                                        .object_fit(ObjectFit::Fill)
                                        .rounded(px(4.0)),
                                )
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(div().text_sm().child(album.name.clone()))
                                    .when_some(album.artist.clone(), |this, artist| {
                                        this.child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.text_secondary)
                                                .child(artist),
                                        )
                                    }),
                            )
                            .on_click({
                                let this = cx.entity();
                                move |_, _, cx| {
                                    this.update(cx, |this, cx| this.open_album(cx, album_id.clone()))
                                }
                            }),
                    );
                }
                list.into_any_element()
            }
            RemoteScreen::Artists { artists, error } => {
                let mut list = div().flex().flex_col().gap(px(2.0));
                if let Some(error) = error {
                    list = list.child(div().text_sm().text_color(theme.status_error).child(error));
                }
                for artist in artists {
                    list = list.child(
                        div()
                            .flex()
                            .p(px(4.0))
                            .hover(|this| this.bg(theme.list_item_hover))
                            .child(div().text_sm().child(artist.name.clone())),
                    );
                }
                list.into_any_element()
            }
            RemoteScreen::Search { songs, error } => {
                let mut list = div().flex().flex_col().gap(px(2.0));
                if let Some(error) = error {
                    list = list.child(div().text_sm().text_color(theme.status_error).child(error));
                }
                for song in songs {
                    let server_id = server_id.clone();
                    let title = song.title.clone();
                    let artist = song.artist.clone();
                    list = list.child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .p(px(4.0))
                            .cursor_pointer()
                            .hover(|this| this.bg(theme.list_item_hover))
                            .child(div().flex().flex_col().flex_grow(1.0).child(
                                div()
                                    .text_sm()
                                    .child(title)
                                    .when_some(artist, |this, a| {
                                        this.child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.text_secondary)
                                                .child(a),
                                        )
                                    }),
                            ))
                            .on_click({
                                let server_id = server_id.clone();
                                let song = song.clone();
                                move |_, _, cx| play_remote_song(cx, &server_id, &song)
                            }),
                    );
                }
                list.into_any_element()
            }
            RemoteScreen::Album { album, error } => {
                let mut list = div().flex().flex_col().gap(px(2.0));
                list = list.child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::BOLD)
                        .child(album.name.clone()),
                );
                if let Some(artist) = &album.artist {
                    list = list.child(
                        div()
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child(artist.clone()),
                    );
                }
                if let Some(error) = error {
                    list = list.child(div().text_sm().text_color(theme.status_error).child(error));
                }
                for song in &album.song {
                    let server_id = server_id.clone();
                    let title = song.title.clone();
                    let track_no = song.track;
                    let duration = song.duration;
                    let is_starred = self
                        .starred
                        .read(cx)
                        .contains(&song.id);
                    list = list.child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .p(px(4.0))
                            .cursor_pointer()
                            .hover(|this| this.bg(theme.list_item_hover))
                            .child(
                                div()
                                    .w(px(24.0))
                                    .text_xs()
                                    .text_color(theme.text_secondary)
                                    .child(track_no.map(|n| n.to_string()).unwrap_or_default()),
                            )
                            .child(div().text_sm().child(title.clone()))
                            .child(
                                div()
                                    .ml_auto()
                                    .text_xs()
                                    .text_color(theme.text_secondary)
                                    .child(duration.map(format_duration).unwrap_or_default()),
                            )
                            .child(
                                div()
                                    .id(format!("remote-star-{}", song.id))
                                    .cursor_pointer()
                                    .on_click({
                                        let this = cx.entity();
                                        let song = song.clone();
                                        move |_, _, cx| {
                                            cx.stop_propagation();
                                            this.update(cx, |this, cx| this.toggle_star(cx, &song))
                                        }
                                    })
                                    .child(
                                        icon(if is_starred { STAR_FILLED } else { STAR })
                                            .size(px(14.0))
                                            .text_color(if is_starred {
                                                theme.liked_song
                                            } else {
                                                theme.text_secondary
                                            }),
                                    ),
                            )
                            .on_click({
                                let server_id = server_id.clone();
                                let song = song.clone();
                                move |_, _, cx| play_remote_song(cx, &server_id, &song)
                            }),
                    );
                }
                list.into_any_element()
            }
        };

        div()
            .id("remote-library")
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .p(px(12.0))
            .gap(px(12.0))
            .child(header)
            .child(
                div()
                    .flex()
                    .w_full()
                    .child(self.search_input.clone())
                    .when(!matches!(screen, RemoteScreen::Search { .. }), |this| this.occlude()),
            )
            .child(div().flex_grow(1.0).overflow_y_scroll().child(body))
    }
}

fn format_duration(seconds: i64) -> String {
    let m = seconds / 60;
    let s = seconds % 60;
    format!("{m}:{s:02}")
}
