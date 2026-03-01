#![feature(try_trait_v2)]
#![feature(adt_const_params)]
#![allow(incomplete_features)]
#![feature(unsized_const_params)]

//! An onebot-11 SDK that simplifies bot creation.
//!
//! Flow-bot is carefully crafted to provide a mechanism similar to that of axum so if you are familiar with axum, you will find it easy to use.
//!
//! The basic unit of event processing in flow-bot is a handler. A handler is a function that optionally takes [`BotContext`] and a [`BotEvent`] or any of the extractors as arguments and returns a [`HandlerControl`].
//! Handlers can parse the incoming event and respond to it. The returned value serves as a control flow signal to determine the flow of the event processing which is where the name comes from.
//!
//! [`BotContext`]: crate::base::context::BotContext
//! [`BotEvent`]: crate::event::BotEvent
//!
//! # Example (Client Mode)
//! ```no_run
//! use flow_bot::{
//!     FlowBotBuilder,
//!     base::{connect::ClientConnectionConfig, extract::Message, handler::HandlerControl},
//! };
//!
//! async fn on_message(msg: Message) -> HandlerControl {
//!     println!("{:?}", msg.message);
//!     HandlerControl::Continue
//! }
//!
//! async fn main() {
//!     let bot = FlowBotBuilder::new(ClientConnectionConfig {
//!         target: "ws://localhost:3001".to_string(),
//!         auth: None,
//!         reconnection: Default::default(),
//!     })
//!     .with_state(())
//!     .with_handler(on_message)
//!     .build();
//!
//!     bot.run().await.unwrap();
//! }
//! ```
//!
//! # Connection Modes
//!
//! Flow-bot supports two WebSocket connection modes:
//!
//! ## Server Mode
//! In server mode, the bot acts as a WebSocket **server** and waits for OneBot client connections.
//! This is useful when you want the bot to control the connection endpoint.
//!
//! ## Client Mode (Default)
//! In client mode, the bot acts as a WebSocket **client** and connects to a OneBot server.
//! This is the most common setup where your OneBot implementation (like go-cqhttp, NapCat, etc.)
//! provides a WebSocket server for clients to connect to.

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use base::{
    connect::{ClientConnectionConfig, ConnectionMode, ServerConnectionConfig},
    context::{BotContext, Context, WebSocketSink},
};
use error::FlowError;
use futures::{
    StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, accept_hdr_async, connect_async,
    tungstenite::{
        Message, Utf8Bytes,
        client::IntoClientRequest,
        handshake::server::{ErrorResponse, Request, Response},
        http::HeaderValue,
    },
};

pub mod api;
pub mod base;
pub mod error;
pub mod runtime;
#[cfg(feature = "api-server")]
pub mod web;

// Re-export event and message modules from flow-bot-onebot11
pub use flow_bot_onebot11::event;
pub use flow_bot_onebot11::message;

pub struct FlowBot {
    context: BotContext,
    connection: ConnectionMode,
    reconnect_attempt: AtomicU32,
    runtime: Option<Arc<runtime::FlowBotRuntime>>,
}

pub struct FlowBotBuilder {
    connection: ConnectionMode,
    runtime: Option<Arc<runtime::FlowBotRuntime>>,
    context: BotContext,
}

impl FlowBotBuilder {
    /// Create a new FlowBotBuilder with the given connection configuration.
    pub fn new(connection: impl Into<ConnectionMode>) -> Self {
        Self {
            connection: connection.into(),
            runtime: None,
            context: BotContext::new(Context::default()),
        }
    }

    /// Create a new FlowBotBuilder with server WebSocket connection configuration.
    /// In server mode, the bot acts as a server waiting for OneBot client connections.
    pub fn new_server(config: ServerConnectionConfig) -> Self {
        Self {
            connection: ConnectionMode::Server(config),
            runtime: None,
            context: BotContext::new(Context::default()),
        }
    }

    /// Create a new FlowBotBuilder with client WebSocket connection configuration.
    /// In client mode, the bot acts as a client connecting to a OneBot server.
    pub fn new_client(config: ClientConnectionConfig) -> Self {
        Self {
            connection: ConnectionMode::Client(config),
            runtime: None,
            context: BotContext::new(Context::default()),
        }
    }

    /// Return a clone of the shared [`BotContext`] that will be embedded in the
    /// built [`FlowBot`].
    ///
    /// Call this **before** [`build`](Self::build) to obtain the context and
    /// pass it to a [`FlowBotRuntime`](runtime::FlowBotRuntime).  Because both
    /// the runtime and the bot will hold the same `Arc<Context>`, the WebSocket
    /// sink that `bot.run()` stores on the context will be immediately visible
    /// to every plugin that calls an outbound API method such as
    /// `send_private_message`.
    pub fn context(&self) -> BotContext {
        self.context.clone()
    }

    /// Add a WASM plugin runtime to the bot.
    /// The runtime will receive all incoming events for plugin processing.
    pub fn with_runtime(mut self, runtime: Arc<runtime::FlowBotRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Build the FlowBot.
    ///
    /// The bot will use the context that was created when this builder was
    /// constructed.  If you previously called [`context()`](Self::context) to
    /// share that context with a runtime, both the bot and the runtime will
    /// operate on the same `Arc<Context>`.
    pub fn build(self) -> FlowBot {
        FlowBot {
            context: self.context,
            connection: self.connection,
            reconnect_attempt: AtomicU32::new(0),
            runtime: self.runtime,
        }
    }
}

impl From<ServerConnectionConfig> for ConnectionMode {
    fn from(config: ServerConnectionConfig) -> Self {
        ConnectionMode::Server(config)
    }
}

impl From<ClientConnectionConfig> for ConnectionMode {
    fn from(config: ClientConnectionConfig) -> Self {
        ConnectionMode::Client(config)
    }
}

impl FlowBot {
    /// Get a reference to the bot context.
    /// This is useful when creating a FlowBotRuntime or accessing API methods.
    pub fn context(&self) -> BotContext {
        self.context.clone()
    }

    /// Run the bot.
    /// This will connect to the server and start processing events.
    /// This method will never return unless an error occurs or reconnection attempts are exhausted.
    pub async fn run(&self) -> Result<(), FlowError> {
        use base::connect::ReconnectionStrategy;

        match self.connection.reconnection_strategy() {
            ReconnectionStrategy::None => self.run_once().await,
            ReconnectionStrategy::Infinite {
                initial_delay_ms,
                max_delay_ms,
            } => {
                self.run_with_infinite_reconnect(*initial_delay_ms, *max_delay_ms)
                    .await
            }
            ReconnectionStrategy::Limited {
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
            } => {
                self.run_with_limited_reconnect(*max_attempts, *initial_delay_ms, *max_delay_ms)
                    .await
            }
        }
    }

    async fn run_once(&self) -> Result<(), FlowError> {
        match &self.connection {
            ConnectionMode::Server(config) => {
                let (write, read) = self.connect_server(config).await?;
                // Connection established successfully, reset attempt counter
                self.reconnect_attempt.store(0, Ordering::Relaxed);
                self.run_server_loop(write, read).await
            }
            ConnectionMode::Client(config) => {
                let (write, read) = self.connect_client(config).await?;
                // Connection established successfully, reset attempt counter
                self.reconnect_attempt.store(0, Ordering::Relaxed);
                self.run_client_loop(write, read).await
            }
        }
    }

    /// Calculate exponential backoff delay with overflow protection
    fn calculate_backoff(&self, initial_delay_ms: u64, max_delay_ms: u64) -> u64 {
        let attempt = self.reconnect_attempt.load(Ordering::Relaxed);
        // Use saturating operations to prevent overflow
        let multiplier = 2_u64.saturating_pow(attempt.min(32)); // Cap exponent at 32 to prevent huge values
        initial_delay_ms.saturating_mul(multiplier).min(max_delay_ms)
    }

    async fn run_with_infinite_reconnect(
        &self,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<(), FlowError> {
        loop {
            let current_delay = self.calculate_backoff(initial_delay_ms, max_delay_ms);

            match self.run_once().await {
                Ok(_) => {
                    eprintln!("Connection closed. Reconnecting in {}ms...", current_delay);
                }
                Err(e) => {
                    eprintln!(
                        "Connection error: {}. Reconnecting in {}ms...",
                        e, current_delay
                    );
                }
            }

            self.reconnect_attempt.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(tokio::time::Duration::from_millis(current_delay)).await;
        }
    }

    async fn run_with_limited_reconnect(
        &self,
        max_attempts: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<(), FlowError> {
        loop {
            let attempt = self.reconnect_attempt.load(Ordering::Relaxed);

            if attempt >= max_attempts {
                return Err(FlowError::ReconnectionFailed(max_attempts));
            }

            let current_delay = self.calculate_backoff(initial_delay_ms, max_delay_ms);

            match self.run_once().await {
                Ok(_) => {
                    // Connection was successful and has now closed
                    // Counter was already reset to 0 in run_once
                    eprintln!("Connection closed. Reconnecting in {}ms...", current_delay);
                }
                Err(e) => {
                    eprintln!(
                        "Connection error: {}. Reconnecting in {}ms... (attempt {}/{})",
                        e,
                        current_delay,
                        attempt + 1,
                        max_attempts
                    );
                }
            }

            self.reconnect_attempt.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(tokio::time::Duration::from_millis(current_delay)).await;
        }
    }

    /// Connect in server mode: bot acts as a WebSocket server
    async fn connect_server(
        &self,
        config: &ServerConnectionConfig,
    ) -> Result<
        (
            SplitSink<WebSocketStream<TcpStream>, Message>,
            SplitStream<WebSocketStream<TcpStream>>,
        ),
        FlowError,
    > {
        let listener = TcpListener::bind(&config.target)
            .await
            .map_err(FlowError::IoError)?;
        tracing::info!("WebSocket server listening on: {}", config.target);

        // Accept a single connection
        // In server mode, we wait for a client to connect
        let (stream, addr) = listener.accept().await.map_err(FlowError::IoError)?;
        tracing::info!("Client connected from: {}", addr);

        // Validate auth header if configured
        let expected_auth = config.auth.clone();
        let ws_stream = accept_hdr_async(stream, |req: &Request, response: Response| {
            if let Some(ref expected) = expected_auth {
                let auth_header = req
                    .headers()
                    .get("authorization")
                    .and_then(|v| v.to_str().ok());

                let provided = auth_header.and_then(|h| {
                    h.strip_prefix("Bearer ").or_else(|| h.strip_prefix("bearer "))
                });

                if provided != Some(expected.as_str()) {
                    tracing::warn!("WebSocket connection rejected: invalid or missing authorization");
                    return Err(ErrorResponse::new(Some(
                        "Invalid or missing Authorization header".to_string()
                    )));
                }
                tracing::debug!("WebSocket client authorized successfully");
            }
            Ok(response)
        })
        .await?;

        Ok(ws_stream.split())
    }

    /// Connect in client mode: bot acts as a WebSocket client
    async fn connect_client(
        &self,
        config: &ClientConnectionConfig,
    ) -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        FlowError,
    > {
        let mut request = config.target.clone().into_client_request()?;
        if let Some(auth) = &config.auth {
            let auth_header: HeaderValue = auth.parse().map_err(|_| {
                FlowError::InvalidConfig(format!("Invalid authorization header value: {}", auth))
            })?;
            request.headers_mut().append("Authorization", auth_header);
        }

        let (ws_stream, _) = connect_async(request).await?;
        Ok(ws_stream.split())
    }

    /// Run message loop for server mode (server mode - accepts plain TcpStream connections)
    async fn run_server_loop(
        &self,
        write: SplitSink<WebSocketStream<TcpStream>, Message>,
        mut read: SplitStream<WebSocketStream<TcpStream>>,
    ) -> Result<(), FlowError> {
        // Store the sink in context for API calls
        self.context.set_sink(WebSocketSink::Server(write)).await;

        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                if let Some(echo) = Self::check_is_echo(&text) {
                    self.context.on_recv_echo(echo, text.to_string());
                    continue;
                }
                self.handle_event(text).await?;
            }
        }
        Ok(())
    }

    /// Run message loop for client mode (client mode - connects to server)
    async fn run_client_loop(
        &self,
        write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> Result<(), FlowError> {
        // Store the sink in context for API calls
        self.context.set_sink(WebSocketSink::Client(write)).await;

        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                if let Some(echo) = Self::check_is_echo(&text) {
                    self.context.on_recv_echo(echo, text.to_string());
                    continue;
                }
                self.handle_event(text).await?;
            }
        }
        Ok(())
    }

    async fn handle_event(&self, text: Utf8Bytes) -> Result<(), FlowError> {
        // Log the incoming event for debugging
        tracing::debug!("Received event: {}", text);

        // Pass the event to the runtime if one is attached
        if let Some(runtime) = &self.runtime
            && let Err(e) = runtime.handle_event(text.as_bytes()).await {
                tracing::error!("Runtime failed to handle event: {}", e);
                // Don't propagate the error - log it and continue processing
            }

        Ok(())
    }

    fn check_is_echo(msg: &str) -> Option<String> {
        let msg = serde_json::from_str::<serde_json::Value>(msg).ok()?;
        if let serde_json::Value::Object(obj) = msg
            && let Some(serde_json::Value::String(echo)) = obj.get("echo")
        {
            return Some(echo.clone());
        }
        None
    }
}
