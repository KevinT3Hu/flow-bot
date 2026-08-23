//! HTTP communication type: flow-bot calls the OneBot implementation's HTTP
//! API endpoints. Carries API calls only — no events.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::Value;

use crate::event::BotEvent;
use tokio::sync::mpsc;

use crate::{
    base::{
        bot::FlowBot,
        transport::{ApiTransport, HttpConfig, validate_http_url},
    },
    error::FlowError,
};

/// Calls `{base_url}/{action}` over HTTP with a JSON body, per the OneBot 11
/// HTTP communication spec.
pub(crate) struct HttpClientTransport {
    client: reqwest::Client,
    base: String,
    access_token: Option<String>,
}

impl HttpClientTransport {
    pub(crate) fn new(cfg: &HttpConfig) -> Result<Self, FlowError> {
        let url = validate_http_url(&cfg.base_url).map_err(FlowError::InvalidConfig)?;
        Ok(Self {
            client: reqwest::Client::new(),
            base: url.as_str().trim_end_matches('/').to_owned(),
            access_token: cfg.access_token.clone(),
        })
    }
}

#[async_trait]
impl ApiTransport for HttpClientTransport {
    async fn send_request(
        &self,
        action: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<String, FlowError> {
        let mut request = self
            .client
            .post(format!("{}/{action}", self.base))
            .timeout(timeout)
            .json(&params);
        if let Some(token) = &self.access_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;
        match response.status() {
            reqwest::StatusCode::OK => Ok(response.text().await?),
            // The spec's HTTP status codes, mapped to retcode semantics.
            reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
                Err(FlowError::InvalidAuth)
            }
            reqwest::StatusCode::NOT_FOUND => Err(FlowError::ApiError {
                status: "failed".to_owned(),
                retcode: 1404,
                message: Some(format!("API `{action}` does not exist")),
            }),
            reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::NOT_ACCEPTABLE => {
                Err(FlowError::ApiError {
                    status: "failed".to_owned(),
                    retcode: 1400,
                    message: Some(format!("bad request for API `{action}`")),
                })
            }
            status => Err(FlowError::ApiError {
                status: "failed".to_owned(),
                retcode: status.as_u16() as i32,
                message: Some(format!("unexpected HTTP status {status}")),
            }),
        }
    }
}

/// Placeholder transport for connection types that cannot call APIs (HTTP
/// POST without a configured HTTP API endpoint).
pub(crate) struct NoApiTransport;

#[async_trait]
impl ApiTransport for NoApiTransport {
    async fn send_request(
        &self,
        _action: &str,
        _params: Value,
        _timeout: Duration,
    ) -> Result<String, FlowError> {
        Err(FlowError::ApiUnavailable)
    }
}

pub(crate) async fn run(
    bot: &FlowBot,
    cfg: &HttpConfig,
    _events: mpsc::Sender<BotEvent>,
) -> Result<(), FlowError> {
    let transport: Arc<dyn ApiTransport> = Arc::new(HttpClientTransport::new(cfg)?);
    bot.context().set_transport(transport);
    bot.init_services_once().await;
    tracing::info!(
        "HTTP connection ready at {} (API calls only; this connection type receives no events)",
        cfg.base_url
    );
    bot.wait_shutdown().await;
    bot.context().clear_transport();
    Ok(())
}
