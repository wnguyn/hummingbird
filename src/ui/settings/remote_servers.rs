//! Settings UI for remote OpenSubsonic/Subsonic music servers.

use cntp_i18n::tr;
use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, px,
};
use gpui::prelude::FluentBuilder;

use crate::{
    providers::opensubsonic::{
        self, ServerConfig, Secret, DEFAULT_API_VERSION, DEFAULT_CLIENT_NAME,
    },
    settings::{Settings, SettingsGlobal, save_settings},
    ui::{
        components::{
            button::{ButtonIntent, button},
            callout::callout,
            label::label,
            section_header::section_header,
            textbox::Textbox,
        },
        theme::Theme,
    },
};

/// Connection status shown under the server form.
#[derive(Clone, PartialEq)]
pub enum ConnectionStatus {
    Idle,
    Testing,
    Success(SharedString),
    Error(SharedString),
}

fn generate_server_id() -> String {
    format!("{:016x}", rand::random::<u64>())
}

/// Derives a friendly display name from a server URL (e.g. hostname).
fn name_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "OpenSubsonic server".to_string())
}

pub struct RemoteServersSettings {
    settings: Entity<Settings>,
    url_input: Entity<Textbox>,
    username_input: Entity<Textbox>,
    password_input: Entity<Textbox>,
    status: Entity<ConnectionStatus>,
}

impl RemoteServersSettings {
    pub fn new(cx: &mut App) -> Entity<Self> {
        let url_input = Textbox::new_with_submit(cx, Default::default(), |_| {});
        let username_input = Textbox::new_with_submit(cx, Default::default(), |_| {});
        let password_input = Textbox::new_with_submit(cx, Default::default(), |_| {});

        cx.new(|cx| {
            let settings = cx.global::<SettingsGlobal>().model.clone();
            let status = cx.new(|_| ConnectionStatus::Idle);
            cx.observe(&settings, |_, _, cx| cx.notify()).detach();
            cx.observe(&status, |_, _, cx| cx.notify()).detach();

            Self {
                settings,
                url_input,
                username_input,
                password_input,
                status,
            }
        })
    }

    fn current_config(&self, cx: &App) -> ServerConfig {
        let url = self.url_input.read(cx).value(cx).to_string();
        let username = self.username_input.read(cx).value(cx).to_string();
        let password = self.password_input.read(cx).value(cx).to_string();

        ServerConfig {
            id: generate_server_id(),
            name: name_from_url(&url),
            url,
            username,
            password: Secret::new(password),
            api_version: DEFAULT_API_VERSION.to_string(),
            client_name: DEFAULT_CLIENT_NAME.to_string(),
        }
    }

    fn set_status(&self, cx: &mut App, status: ConnectionStatus) {
        self.status.update(cx, |s, cx| {
            *s = status;
            cx.notify();
        });
    }

    /// Calls `ping` on the server and reports success only on successful auth.
    fn test_connection(&self, cx: &mut App) {
        let config = self.current_config(cx);
        if config.url.is_empty() || config.username.is_empty() {
            self.set_status(cx, ConnectionStatus::Error(tr!("SERVERS_FIELDS_REQUIRED", "Server URL and username are required.").into()));
            return;
        }

        let status = self.status.clone();
        self.set_status(cx, ConnectionStatus::Testing);

        cx.spawn(async move |cx| {
            let result = match config.client() {
                Ok(client) => client.ping().await.map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            };

            status.update(cx, |s, cx| {
                *s = match result {
                    Ok(()) => ConnectionStatus::Success(tr!("SERVERS_TEST_OK", "Connected successfully.").into()),
                    Err(e) => ConnectionStatus::Error(e.into()),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Constructs a client, registers it, and persists the configuration.
    fn save_server(&self, cx: &mut App) {
        let config = self.current_config(cx);
        if config.url.is_empty() || config.username.is_empty() {
            self.set_status(cx, ConnectionStatus::Error(tr!("SERVERS_FIELDS_REQUIRED", "Server URL and username are required.").into()));
            return;
        }

        let client = match config.client() {
            Ok(client) => client,
            Err(e) => {
                self.set_status(cx, ConnectionStatus::Error(e.to_string().into()));
                return;
            }
        };

        opensubsonic::register_client(config.id.clone(), client);
        self.settings.update(cx, move |settings, cx| {
            settings.opensubsonic.servers.push(config.clone());
            save_settings(cx, settings);
            cx.notify();
        });

        self.url_input.update(cx, |input, cx| input.set_value(cx, "".into()));
        self.username_input.update(cx, |input, cx| input.set_value(cx, "".into()));
        self.password_input.update(cx, |input, cx| input.set_value(cx, "".into()));
        self.set_status(cx, ConnectionStatus::Success(tr!("SERVERS_SAVED", "Server saved and connected.").into()));
    }

    fn remove_server(&self, cx: &mut App, server_id: String) {
        opensubsonic::remove_client(&server_id);
        self.settings.update(cx, move |settings, cx| {
            settings.opensubsonic.servers.retain(|s| s.id != server_id);
            save_settings(cx, settings);
            cx.notify();
        });
    }
}

impl Render for RemoteServersSettings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let servers = self.settings.read(cx).opensubsonic.servers.clone();
        let status = self.status.read(cx).clone();

        let mut body = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(section_header(tr!("SERVERS", "Remote Music Servers")).subtitle(tr!(
                "SERVERS_SUBTITLE",
                "Connect to an OpenSubsonic or Subsonic server such as Navidrome."
            )));

        for server in &servers {
            body = body.child(
                label("servers-connected", server.name.clone())
                    .subtext(server.url.clone())
                    .w_full()
                    .child(
                        button()
                            .id(format!("servers-remove-{}", server.id))
                            .intent(ButtonIntent::Danger)
                            .child(tr!("SERVERS_REMOVE", "Remove"))
                            .on_click({
                                let id = server.id.clone();
                                let this = cx.entity();
                                move |_, _, cx| this.update(cx, |this, cx| this.remove_server(cx, id.clone()))
                            }),
                    ),
            );
        }

        if servers.is_empty() {
            body = body.child(div().text_sm().text_color(theme.text_secondary).child(tr!(
                "SERVERS_NONE",
                "No remote servers connected yet."
            )));
        }

        body = body
            .child(section_header(tr!("SERVERS_ADD", "Connect a server")))
            .child(label("servers-url", tr!("SERVERS_URL", "Server URL")).w_full().child(self.url_input.clone()))
            .child(label("servers-username", tr!("SERVERS_USERNAME", "Username")).w_full().child(self.username_input.clone()))
            .child(label("servers-password", tr!("SERVERS_PASSWORD", "Password")).w_full().child(self.password_input.clone()))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(
                        button()
                            .id("servers-test")
                            .intent(ButtonIntent::Secondary)
                            .child(tr!("SERVERS_TEST", "Test Connection"))
                            .on_click({
                                let this = cx.entity();
                                move |_, _, cx| this.update(cx, |this, cx| this.test_connection(cx))
                            }),
                    )
                    .child(
                        button()
                            .id("servers-save")
                            .intent(ButtonIntent::Primary)
                            .child(tr!("SERVERS_SAVE", "Save & Connect"))
                            .on_click({
                                let this = cx.entity();
                                move |_, _, cx| this.update(cx, |this, cx| this.save_server(cx))
                            }),
                    ),
            );

        match status {
            ConnectionStatus::Idle => {}
            ConnectionStatus::Testing => {
                body = body.child(div().text_sm().text_color(theme.text_secondary).child(tr!(
                    "SERVERS_TESTING",
                    "Testing connection…"
                )));
            }
            ConnectionStatus::Success(msg) => {
                body = body.child(callout(msg));
            }
            ConnectionStatus::Error(msg) => {
                body = body.child(callout(msg).icon(crate::ui::components::icons::ALERT_CIRCLE));
            }
        }

        body
    }
}
