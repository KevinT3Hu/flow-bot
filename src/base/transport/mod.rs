//! Connection configurations for the four OneBot 11 communication types.
//!
//! All four types share one surface API: a [`FlowBot`](crate::FlowBot) built
//! from a [`ConnectionConfig`] behaves identically regardless of the
//! underlying transport — handlers receive events through the same extractors
//! and API calls go through the same [`ApiExt`](crate::api::api_ext::ApiExt).
//!
//! | Variant | Direction | Events | API calls |
//! |---|---|---|---|
//! | [`ForwardWebSocket`](ConnectionConfig::ForwardWebSocket) | SDK dials the implementation's WS server | yes | over the same socket |
//! | [`ReverseWebSocket`](ConnectionConfig::ReverseWebSocket) | implementation dials the SDK's WS server | yes | over the same socket |
//! | [`Http`](ConnectionConfig::Http) | SDK calls the implementation's HTTP server | no | one HTTP request per call |
//! | [`HttpPost`](ConnectionConfig::HttpPost) | implementation POSTs events to the SDK's HTTP server | yes | optional [`HttpConfig`] endpoint |

pub(crate) mod forward_ws;
pub(crate) mod http;
pub(crate) mod http_post;
pub(crate) mod reverse_ws;
pub(crate) mod ws;

use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;
use serde_json::Value;

use crate::error::FlowError;

/// Reconnection strategy configuration.
///
/// Only the forward WebSocket connection reconnects; server-side connection
/// types keep listening and let the implementation retry instead.
#[derive(Clone, Debug)]
pub enum ReconnectionStrategy {
    /// Reconnect endlessly with exponential backoff.
    Infinite {
        /// Initial delay in milliseconds (default: 1000).
        initial_delay_ms: u64,
        /// Maximum delay in milliseconds (default: 60000).
        max_delay_ms: u64,
    },
    /// Reconnect for a limited number of attempts.
    Limited {
        /// Maximum number of reconnection attempts.
        max_attempts: u32,
        /// Initial delay in milliseconds (default: 1000).
        initial_delay_ms: u64,
        /// Maximum delay in milliseconds (default: 60000).
        max_delay_ms: u64,
    },
    /// Do not reconnect.
    None,
}

impl Default for ReconnectionStrategy {
    fn default() -> Self {
        Self::Infinite {
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
        }
    }
}

/// Forward WebSocket (正向 WebSocket): the OneBot implementation runs a
/// WebSocket server and flow-bot connects to it as a client.
///
/// The URL selects the endpoint: `/` for events and API calls on one
/// connection, `/api` for API calls only, `/event` for events only.
#[derive(Clone, Debug)]
pub struct ForwardWebSocketConfig {
    /// WebSocket URL of the implementation, e.g. `ws://127.0.0.1:6700/`.
    pub url: String,
    /// Access token sent as `Authorization: Bearer <token>`.
    pub access_token: Option<String>,
    /// What to do when the connection drops.
    pub reconnection: ReconnectionStrategy,
}

/// Reverse WebSocket (反向 WebSocket): flow-bot runs a WebSocket server and
/// the OneBot implementation connects to it as a client.
///
/// The implementation announces itself through the `X-Self-ID` and
/// `X-Client-Role` (`API`, `Event` or `Universal`) handshake headers. API
/// calls travel over API/Universal connections, events arrive on
/// Event/Universal connections. The implementation is responsible for
/// reconnecting.
#[derive(Clone, Debug)]
pub struct ReverseWebSocketConfig {
    /// Address for the SDK's WebSocket server to listen on.
    pub bind: SocketAddr,
    /// Only accept connections to this path (e.g. `/ws`); accept any path if `None`.
    pub path: Option<String>,
    /// If set, the implementation's handshake must carry
    /// `Authorization: Bearer <token>` or it is rejected.
    pub access_token: Option<String>,
}

/// HTTP: the OneBot implementation runs an HTTP server and flow-bot calls its
/// API endpoints. This communication type carries no events.
#[derive(Clone, Debug)]
pub struct HttpConfig {
    /// Base URL of the implementation's HTTP API, e.g. `http://127.0.0.1:5700`.
    pub base_url: String,
    /// Access token sent as `Authorization: Bearer <token>`.
    pub access_token: Option<String>,
}

/// HTTP POST: flow-bot runs an HTTP server and the OneBot implementation POSTs
/// events to it.
///
/// Outbound API calls need a separate [`HttpConfig`] endpoint (implementations
/// serving HTTP POST events usually also serve the HTTP API); without one, API
/// calls fail with [`FlowError::ApiUnavailable`].
#[derive(Clone, Debug)]
pub struct HttpPostConfig {
    /// Address for the SDK's HTTP server to listen on.
    pub bind: SocketAddr,
    /// Route path receiving event posts (e.g. `/`; must start with `/`).
    pub path: String,
    /// If set, every event POST must carry a matching `X-Signature`
    /// (`sha1=` + hex HMAC-SHA1 of the raw body keyed with the secret).
    pub secret: Option<String>,
    /// Optional HTTP API endpoint used for outbound API calls.
    pub api: Option<HttpConfig>,
    /// How long each event POST waits for the handler chain to finish before
    /// answering, so that quick operations attached by handlers can travel
    /// in the response body (the spec's HTTP-POST quick-operation channel).
    /// Events whose handlers exceed it are answered with `204 No Content`;
    /// handlers attaching operations after the deadline fall back to the
    /// `.handle_quick_operation` API when [`api`](Self::api) is configured.
    pub response_timeout: Duration,
}

/// The four OneBot 11 communication types.
#[derive(Clone, Debug)]
pub enum ConnectionConfig {
    /// Forward WebSocket (正向): SDK dials the implementation's server.
    ForwardWebSocket(ForwardWebSocketConfig),
    /// Reverse WebSocket (反向): the implementation dials the SDK's server.
    ReverseWebSocket(ReverseWebSocketConfig),
    /// HTTP: SDK calls the implementation's HTTP API; no events.
    Http(HttpConfig),
    /// HTTP POST: the implementation POSTs events to the SDK's HTTP server.
    HttpPost(HttpPostConfig),
}

impl ConnectionConfig {
    /// Validate the configuration (URL schemes, TLS availability, paths).
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::ForwardWebSocket(cfg) => {
                validate_ws_url(&cfg.url)?;
            }
            Self::ReverseWebSocket(cfg) => {
                if let Some(path) = &cfg.path
                    && !path.starts_with('/')
                {
                    return Err(format!("connection path must start with `/`: {path}"));
                }
            }
            Self::Http(cfg) => {
                validate_http_url(&cfg.base_url)?;
            }
            Self::HttpPost(cfg) => {
                if !cfg.path.starts_with('/') {
                    return Err(format!("connection path must start with `/`: {}", cfg.path));
                }
                if let Some(api) = &cfg.api {
                    validate_http_url(&api.base_url)?;
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_ws_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("invalid URL `{url}`: {e}"))?;
    match parsed.scheme() {
        "ws" => Ok(parsed),
        "wss" => {
            if cfg!(feature = "tls") {
                Ok(parsed)
            } else {
                Err(format!("`wss://` requires the `tls` crate feature: {url}"))
            }
        }
        other => Err(format!(
            "expected a `ws://` or `wss://` URL, got scheme `{other}`: {url}"
        )),
    }
}

pub(crate) fn validate_http_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("invalid URL `{url}`: {e}"))?;
    match parsed.scheme() {
        "http" => Ok(parsed),
        "https" => {
            if cfg!(feature = "tls") {
                Ok(parsed)
            } else {
                Err(format!(
                    "`https://` requires the `tls` crate feature: {url}"
                ))
            }
        }
        other => Err(format!(
            "expected an `http://` or `https://` URL, got scheme `{other}`: {url}"
        )),
    }
}

/// Transport-agnostic outbound path for OneBot API calls.
///
/// Returns the raw JSON response envelope (`{status, retcode, data, echo}`) as
/// received; envelope checking (retcode) happens in
/// [`Context::send_obj`](crate::base::context::Context::send_obj).
#[async_trait]
pub(crate) trait ApiTransport: Send + Sync {
    async fn send_request(
        &self,
        action: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<String, FlowError>;
}
