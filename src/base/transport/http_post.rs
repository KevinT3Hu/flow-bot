//! HTTP POST communication type: flow-bot runs an HTTP server and the OneBot
//! implementation POSTs events to it. Outbound API calls optionally travel
//! over a separate HTTP connection.

use std::sync::Arc;

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha1::Sha1;
use tokio::sync::mpsc;

use crate::{
    base::{
        bot::FlowBot,
        transport::{
            ApiTransport, HttpPostConfig,
            http::{HttpClientTransport, NoApiTransport},
        },
    },
    error::FlowError,
};

type HmacSha1 = Hmac<Sha1>;

#[derive(Clone)]
struct WebhookState {
    secret: Option<String>,
    events: mpsc::Sender<Value>,
}

pub(crate) async fn run(
    bot: &FlowBot,
    cfg: &HttpPostConfig,
    events: mpsc::Sender<Value>,
) -> Result<(), FlowError> {
    // Outbound API calls go over a separate HTTP connection if configured;
    // otherwise they fail fast with `ApiUnavailable`.
    let transport: Arc<dyn ApiTransport> = match &cfg.api {
        Some(api) => Arc::new(HttpClientTransport::new(api)?),
        None => Arc::new(NoApiTransport),
    };
    bot.context().set_transport(transport);

    let listener = tokio::net::TcpListener::bind(cfg.bind)
        .await
        .map_err(FlowError::ServerError)?;
    tracing::info!("HTTP POST server listening on {}", cfg.bind);
    bot.init_services_once().await;

    let state = WebhookState {
        secret: cfg.secret.clone(),
        events,
    };
    let app = Router::new()
        .route(&cfg.path, post(webhook))
        .with_state(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(bot.shutdown_signal())
        .await
        .map_err(FlowError::ServerError)?;
    Ok(())
}

async fn webhook(State(state): State<WebhookState>, headers: HeaderMap, body: Bytes) -> Response {
    if let Some(secret) = &state.secret {
        let provided = headers
            .get("x-signature")
            .and_then(|value| value.to_str().ok());
        if !verify_signature(secret, &body, provided) {
            tracing::warn!("rejected event POST with invalid X-Signature");
            return StatusCode::FORBIDDEN.into_response();
        }
    }

    let value: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(e) => {
            tracing::warn!("dropping malformed event body: {e}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    // Block until the event queue has room (backpressure). The OneBot 11 spec
    // requires the backend to always answer: 204 means "no quick operation".
    if state.events.send(value).await.is_err() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

/// Verify `X-Signature: sha1=<hex>` — HMAC-SHA1 of the raw body keyed with
/// the secret, compared in constant time.
pub(crate) fn verify_signature(secret: &str, body: &[u8], provided: Option<&str>) -> bool {
    let Some(provided) = provided.and_then(|sig| sig.strip_prefix("sha1=")) else {
        return false;
    };
    let Ok(mut mac) = HmacSha1::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    let digest = hex_lower(&mac.finalize().into_bytes());
    constant_time_eq(digest.as_bytes(), provided.as_bytes())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(secret: &str, body: &str) -> String {
        let mut mac = HmacSha1::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        let digest = mac.finalize().into_bytes();
        format!("sha1={}", hex_lower(&digest))
    }

    #[test]
    fn accepts_a_correct_signature() {
        let body = r#"{"post_type":"message"}"#;
        let sig = sign("secret", body);
        assert!(verify_signature("secret", body.as_bytes(), Some(&sig)));
    }

    #[test]
    fn rejects_wrong_secret_wrong_body_or_missing_header() {
        let body = r#"{"post_type":"message"}"#;
        let sig = sign("secret", body);
        assert!(!verify_signature("other", body.as_bytes(), Some(&sig)));
        assert!(!verify_signature(
            "secret",
            r#"{"post_type":"notice"}"#.as_bytes(),
            Some(&sig)
        ));
        assert!(!verify_signature("secret", body.as_bytes(), None));
        // Not prefixed with "sha1=".
        assert!(!verify_signature(
            "secret",
            body.as_bytes(),
            Some(sig.trim_start_matches("sha1="))
        ));
    }
}
