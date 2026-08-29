//! HTTP client for OpenSubsonic/Subsonic servers.
//!
//! Builds authenticated request URLs using Subsonic token authentication and
//! parses the `subsonic-response` envelope. Credentials are only ever placed
//! in query parameters for the token computation — never logged.

use std::time::Duration;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{
    auth,
    credentials::Secret,
    errors::{SubsonicError, codes},
    models::{Empty, Envelope},
};

pub const DEFAULT_API_VERSION: &str = "1.16.1";
pub const DEFAULT_CLIENT_NAME: &str = "Hummingbird";

/// Metadata request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// Connect timeout for all requests.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// Persistent configuration for a single OpenSubsonic server.
///
/// The `id` is a stable, locally-generated identifier used to namespace remote
/// items so they never collide with local tracks or with another server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub username: String,
    pub password: Secret,
    pub api_version: String,
    pub client_name: String,
}

impl ServerConfig {
    pub fn client(&self) -> Result<OpenSubsonicClient, SubsonicError> {
        OpenSubsonicClient::new(self.clone())
    }
}

/// Normalizes a server base URL so trailing slashes and subpaths work.
///
/// The returned URL always has a trailing slash so `Url::join` resolves the
/// `rest/…` path relative to a subpath-mounted server correctly.
pub fn normalize_base_url(input: &str) -> Result<url::Url, SubsonicError> {
    let trimmed = input.trim();
    let mut url = url::Url::parse(trimmed).map_err(|_| SubsonicError::InvalidResponse)?;

    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(SubsonicError::InvalidResponse),
    }

    let path = url.path().trim_end_matches('/');
    url.set_path(&format!("{path}/"));

    // Drop any query/fragment a user may have pasted in.
    url.set_query(None);
    url.set_fragment(None);

    Ok(url)
}

#[derive(Clone)]
pub struct OpenSubsonicClient {
    client: zed_reqwest::Client,
    base_url: url::Url,
    username: String,
    password: Secret,
    api_version: String,
    client_name: String,
}

impl OpenSubsonicClient {
    pub fn new(config: ServerConfig) -> Result<Self, SubsonicError> {
        let base_url = normalize_base_url(&config.url)?;
        let client = zed_reqwest::Client::builder()
            .user_agent(format!("{}/{}", config.client_name, crate::VERSION_STRING))
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(SubsonicError::Transport)?;
        Ok(Self {
            client,
            base_url,
            username: config.username,
            password: config.password,
            api_version: config.api_version,
            client_name: config.client_name,
        })
    }

    #[allow(dead_code)]
    pub fn base_url(&self) -> &url::Url {
        &self.base_url
    }

    #[allow(dead_code)]
    pub fn client(&self) -> &zed_reqwest::Client {
        &self.client
    }

    /// Builds the `/rest/<endpoint>` URL, including auth query parameters.
    fn endpoint_url(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<url::Url, SubsonicError> {
        let mut url = self
            .base_url
            .join(&format!("rest/{endpoint}"))
            .map_err(|_| SubsonicError::InvalidResponse)?;

        let salt = auth::salt();
        let token = auth::token(self.password.expose(), &salt);

        {
            let mut pairs = url.query_pairs_mut();
            pairs
                .append_pair("u", &self.username)
                .append_pair("s", &salt)
                .append_pair("t", &token)
                .append_pair("v", &self.api_version)
                .append_pair("c", &self.client_name)
                .append_pair("f", "json");
            for (key, value) in params {
                pairs.append_pair(key, value);
            }
        }

        Ok(url)
    }

    /// Builds an authenticated request for an endpoint (used for streaming and
    /// cover art, which need raw access rather than JSON parsing).
    pub fn request(
        &self,
        method: zed_reqwest::Method,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<zed_reqwest::RequestBuilder, SubsonicError> {
        let url = self.endpoint_url(endpoint, params)?;
        Ok(self.client.request(method, url))
    }

    /// GET an endpoint and deserialize its payload, mapping Subsonic errors to
    /// typed [`SubsonicError`]s.
    pub async fn get_json<T>(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<T, SubsonicError>
    where
        T: DeserializeOwned,
    {
        let response = self
            .request(zed_reqwest::Method::GET, endpoint, params)?
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(SubsonicError::Transport)?;

        let status = response.status();
        if status == zed_reqwest::StatusCode::UNAUTHORIZED
            || status == zed_reqwest::StatusCode::FORBIDDEN
        {
            return Err(SubsonicError::AuthenticationFailed);
        }
        if status == zed_reqwest::StatusCode::NOT_FOUND {
            return Err(SubsonicError::NotFound);
        }
        if !status.is_success() {
            return Err(SubsonicError::Api {
                code: codes::GENERIC,
                message: format!("HTTP {status}"),
            });
        }

        let bytes = response.bytes().await.map_err(SubsonicError::Transport)?;
        Self::parse_envelope(&bytes)
    }

    /// POST an endpoint that returns no meaningful body (e.g. `star`, `scrobble`).
    pub async fn post(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<(), SubsonicError> {
        let response = self
            .request(zed_reqwest::Method::POST, endpoint, params)?
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(SubsonicError::Transport)?;

        let status = response.status();
        if status == zed_reqwest::StatusCode::UNAUTHORIZED
            || status == zed_reqwest::StatusCode::FORBIDDEN
        {
            return Err(SubsonicError::AuthenticationFailed);
        }
        if status == zed_reqwest::StatusCode::NOT_FOUND {
            return Err(SubsonicError::NotFound);
        }
        if !status.is_success() {
            return Err(SubsonicError::Api {
                code: codes::GENERIC,
                message: format!("HTTP {status}"),
            });
        }

        let bytes = response.bytes().await.map_err(SubsonicError::Transport)?;
        Self::parse_envelope::<Empty>(&bytes).map(|_| ())
    }

    /// Parses a `subsonic-response` envelope, surfacing the Subsonic `error`
    /// element as a typed error.
    fn parse_envelope<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, SubsonicError> {
        let envelope: Envelope<T> =
            serde_json::from_slice(bytes).map_err(|_| SubsonicError::InvalidResponse)?;

        if let Some(error) = envelope.response.error {
            return Err(SubsonicError::from_api(
                error.code,
                error.message.unwrap_or_default(),
            ));
        }

        if envelope.response.status != "ok" {
            return Err(SubsonicError::Api {
                code: codes::GENERIC,
                message: "Unknown server error".to_string(),
            });
        }

        Ok(envelope.response.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_trailing_slash() {
        let url = normalize_base_url("https://example.com/navidrome/").unwrap();
        assert_eq!(url.as_str(), "https://example.com/navidrome/");
    }

    #[test]
    fn normalize_adds_trailing_slash() {
        let url = normalize_base_url("https://example.com/navidrome").unwrap();
        assert_eq!(url.as_str(), "https://example.com/navidrome/");
    }

    #[test]
    fn normalize_rejects_non_http_schemes() {
        assert!(normalize_base_url("ftp://example.com").is_err());
    }

    #[test]
    fn normalize_strips_query_and_fragment() {
        let url = normalize_base_url("https://example.com/navidrome?x=1#frag").unwrap();
        assert_eq!(url.as_str(), "https://example.com/navidrome/");
    }
}
