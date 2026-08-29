//! Typed errors for the OpenSubsonic client.
//!
//! All error text is user-facing where it escapes the client, so credential and
//! authenticated URL details are deliberately excluded from every variant.

use thiserror::Error;

/// Errors surfaced by the OpenSubsonic client.
#[derive(Debug, Error)]
pub enum SubsonicError {
    /// The transport layer failed (DNS, TLS, connection refused, timeout...).
    #[error("Could not reach the server: {0}")]
    Transport(#[from] zed_reqwest::Error),

    /// The server responded with an authentication failure.
    #[error("Authentication failed (wrong username or password)")]
    AuthenticationFailed,

    /// The server returned a Subsonic API error element.
    #[error("{message}")]
    Api { code: i32, message: String },

    /// The server responded but the body was not valid Subsonic JSON.
    #[error("The server returned an unexpected response")]
    InvalidResponse,

    /// The requested resource does not exist (e.g. a track was removed).
    #[error("Not found on server")]
    NotFound,

    /// The server does not implement the requested endpoint.
    #[allow(dead_code)]
    #[error("The server does not support this feature")]
    Unsupported,

    /// No client is registered for the requested server.
    #[error("Server is not connected")]
    NotConnected,
}

/// The well-known Subsonic status codes (subset used for classification).
#[allow(dead_code)]
pub mod codes {
    pub const GENERIC: i32 = 0;
    pub const REQUIRED_PARAMETER_MISSING: i32 = 10;
    pub const INCOMPATIBLE_CLIENT_VERSION: i32 = 20;
    pub const INCOMPATIBLE_SERVER_VERSION: i32 = 30;
    pub const WRONG_USERNAME_OR_PASSWORD: i32 = 40;
    pub const TOKEN_AUTHENTICATION_NOT_SUPPORTED: i32 = 41;
    pub const USER_NOT_AUTHORIZED: i32 = 50;
    pub const DATA_NOT_FOUND: i32 = 70;
}

impl SubsonicError {
    /// Classify a Subsonic status code into a typed error.
    pub fn from_api(code: i32, message: String) -> Self {
        match code {
            codes::WRONG_USERNAME_OR_PASSWORD | codes::TOKEN_AUTHENTICATION_NOT_SUPPORTED => {
                Self::AuthenticationFailed
            }
            codes::DATA_NOT_FOUND => Self::NotFound,
            _ => Self::Api { code, message },
        }
    }
}
