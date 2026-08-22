//! Shared plumbing for the two WebSocket connection types: an outbound writer
//! task, echo-keyed pending API requests, and inbound frame routing.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use dashmap::DashMap;
use futures::{SinkExt, stream::SplitSink};
use serde_json::{Value, json};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

use crate::{base::transport::ApiTransport, error::FlowError};

/// Anything that can send one outbound text frame. Generalizes over the
/// tokio-tungstenite (forward WS) and axum (reverse WS) sinks.
#[async_trait]
pub(crate) trait FrameWriter: Send {
    async fn send_text(&mut self, text: String) -> Result<(), FlowError>;
}

#[async_trait]
impl FrameWriter for SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message> {
    async fn send_text(&mut self, text: String) -> Result<(), FlowError> {
        self.send(Message::Text(text.into()))
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl FrameWriter for SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message> {
    async fn send_text(&mut self, text: String) -> Result<(), FlowError> {
        self.send(axum::extract::ws::Message::Text(text.into()))
            .await
            .map_err(|e| FlowError::ServerError(std::io::Error::other(e)))
    }
}

/// One live WebSocket connection to a OneBot implementation: the outbound
/// writer channel plus the pending echo-keyed API requests.
pub(crate) struct WsSession {
    writer: mpsc::Sender<String>,
    pending: DashMap<String, oneshot::Sender<String>>,
}

impl WsSession {
    /// Spawn the detached writer task for `write` and return the session
    /// handle. The writer task fails all pending requests when the socket
    /// errors out or the session is dropped.
    pub(crate) fn spawn<W>(write: W) -> Arc<Self>
    where
        W: FrameWriter + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<String>(128);
        let session = Arc::new(Self {
            writer: tx,
            pending: DashMap::new(),
        });
        let writer_session = Arc::downgrade(&session);
        tokio::spawn(async move {
            let mut write = write;
            while let Some(text) = rx.recv().await {
                if write.send_text(text).await.is_err() {
                    break;
                }
            }
            if let Some(session) = writer_session.upgrade() {
                session.fail_pending();
            }
        });
        session
    }

    /// Fail every in-flight API request immediately. Dropping the oneshot
    /// senders makes the waiting calls error out instead of timing out.
    pub(crate) fn fail_pending(&self) {
        self.pending.clear();
    }

    /// Route one inbound text frame: complete a pending API response (the
    /// `echo` field matches an outstanding request) or forward an event into
    /// the dispatch queue.
    ///
    /// A frame that is not valid JSON is logged and skipped instead of tearing
    /// down the connection, and blocks until the event queue has room
    /// (backpressure).
    pub(crate) async fn handle_frame(&self, text: &str, events: &mpsc::Sender<Value>) {
        let value: Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!("dropping malformed WebSocket frame: {e}");
                return;
            }
        };
        if let Some(echo) = value.get("echo").and_then(Value::as_str)
            && let Some((_, tx)) = self.pending.remove(echo)
        {
            let _ = tx.send(text.to_owned());
            return;
        }
        if events.send(value).await.is_err() {
            tracing::debug!("event dropped: the dispatcher is no longer running");
        }
    }
}

#[async_trait]
impl ApiTransport for WsSession {
    async fn send_request(
        &self,
        action: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<String, FlowError> {
        let echo = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.insert(echo.clone(), tx);
        let msg = json!({ "action": action, "params": params, "echo": echo }).to_string();

        let request = async {
            self.writer
                .send(msg)
                .await
                .map_err(|_| FlowError::NoConnection)?;
            match rx.await {
                Ok(raw) => Ok(raw),
                // The pending slot was failed: the connection was lost.
                Err(_) => Err(FlowError::NoConnection),
            }
        };

        match tokio::time::timeout(timeout, request).await {
            Ok(result) => result,
            Err(_) => {
                self.pending.remove(&echo);
                Err(FlowError::Timeout(timeout.as_millis() as u64))
            }
        }
    }
}
