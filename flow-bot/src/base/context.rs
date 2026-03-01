use std::sync::Arc;

use dashmap::DashMap;
use futures::{SinkExt, stream::SplitSink};
use serde_json::json;
use tokio::{
    net::TcpStream,
    sync::{Mutex, oneshot},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

use crate::{
    api::{ApiResponse, api_ext::ApiExt},
    error::FlowError,
};

/// A guard that removes the pending request entry when dropped.
/// This ensures cleanup happens even if the response arrives after timeout.
struct PendingRequestGuard {
    pending_requests: Arc<DashMap<String, oneshot::Sender<String>>>,
    echo: Option<String>,
}

impl PendingRequestGuard {
    fn new(pending_requests: Arc<DashMap<String, oneshot::Sender<String>>>, echo: String) -> Self {
        Self {
            pending_requests,
            echo: Some(echo),
        }
    }

    /// Disarm the guard, preventing cleanup on drop.
    fn disarm(mut self) {
        self.echo = None;
    }
}

impl Drop for PendingRequestGuard {
    fn drop(&mut self) {
        if let Some(echo) = self.echo.take() {
            self.pending_requests.remove(&echo);
        }
    }
}

/// Enum to wrap different WebSocket sink types for server and client modes
pub enum WebSocketSink {
    /// Server mode: plain TCP stream (bot acts as WebSocket server)
    Server(SplitSink<WebSocketStream<TcpStream>, Message>),
    /// Client mode: possibly TLS stream (bot acts as WebSocket client)
    Client(SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>),
}

impl WebSocketSink {
    /// Send a message through the websocket
    pub async fn send(&mut self, msg: Message) -> Result<(), FlowError> {
        match self {
            WebSocketSink::Server(sink) => sink.send(msg).await.map_err(FlowError::WebSocketError),
            WebSocketSink::Client(sink) => sink.send(msg).await.map_err(FlowError::WebSocketError),
        }
    }
}

pub struct Context {
    pub(crate) sink: Mutex<Option<WebSocketSink>>,
    pending_requests: Arc<DashMap<String, oneshot::Sender<String>>>,
    request_timeout_secs: u64,
}

impl Context {
    /// Create a new context with custom timeout setting
    pub(crate) fn new(request_timeout_secs: u64) -> Self {
        Self {
            sink: Mutex::new(None),
            pending_requests: Arc::new(DashMap::new()),
            request_timeout_secs,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(30)
    }
}

impl Context {
    /// Set the websocket sink for sending API requests
    pub(crate) async fn set_sink(&self, sink: WebSocketSink) {
        let mut ws_sink = self.sink.lock().await;
        *ws_sink = Some(sink);
    }

    pub(crate) async fn send_obj<T, R>(
        &self,
        action: String,
        obj: T,
    ) -> Result<ApiResponse<R>, FlowError>
    where
        T: serde::Serialize,
        R: for<'de> serde::Deserialize<'de>,
    {
        // Generate random echo string
        let echo = uuid::Uuid::new_v4().to_string();

        // Create oneshot channel for this specific request
        let (tx, rx) = oneshot::channel();

        // Register the request BEFORE sending (lock-free)
        self.pending_requests.insert(echo.clone(), tx);

        // Create a guard to ensure cleanup happens even if response arrives after timeout.
        // This prevents unbounded growth of pending_requests.
        let guard = PendingRequestGuard::new(self.pending_requests.clone(), echo.clone());

        // Build and send the message
        let msg = json!({
            "action": action,
            "params": obj,
            "echo": echo,
        });
        let text = serde_json::to_string(&msg)?;
        let msg = Message::Text(text.into());

        // Send message and release lock immediately
        {
            let mut sink = self.sink.lock().await;
            let sink = sink.as_mut().ok_or(FlowError::NoConnection)?;
            sink.send(msg).await?;
        }

        // Wait for response with configurable timeout
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(self.request_timeout_secs),
            rx,
        )
        .await;

        match response {
            Ok(Ok(data)) => {
                // Success: disarm the guard since on_recv_echo will handle cleanup
                guard.disarm();
                Ok(serde_json::from_str(&data)?)
            }
            Ok(Err(_)) => {
                // Sender dropped: let guard clean up the entry
                Err(FlowError::NoResponse)
            }
            Err(_) => {
                // Timeout: let guard clean up the entry
                Err(FlowError::Timeout(self.request_timeout_secs * 1000))
            }
        }
    }

    pub(crate) fn on_recv_echo(&self, echo: String, data: String) {
        // Try to remove and send immediately without spawning a task.
        // This reduces latency and prevents race conditions with the timeout handler.
        if let Some((_, tx)) = self.pending_requests.remove(&echo) {
            let _ = tx.send(data); // Ignore error if receiver dropped
        }
        // If echo not found, the request was either already timed out (guard cleaned up)
        // or is being processed - silently ignore.
    }

    pub async fn get_self_id(&self) -> Result<i64, FlowError> {
        let info = self.get_login_info().await?;
        Ok(info.user_id)
    }
}

pub type BotContext = Arc<Context>;
