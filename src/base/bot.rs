use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde_json::Value;
use tokio::{net::TcpStream, sync::Notify, sync::Semaphore};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use crate::{
    base::{
        connect::{ReconnectionStrategy, ReverseConnectionConfig},
        context::BotContext,
        control::HandlerControl,
        middleware::EventProcessor,
    },
    error::FlowError,
    event::Event,
};

pub struct FlowBot {
    pub(crate) processors: Arc<Vec<Arc<dyn EventProcessor>>>,
    pub(crate) context: BotContext,
    pub(crate) connection: ReverseConnectionConfig,
    pub(crate) reconnect_attempt: AtomicU32,
    pub(crate) concurrent_limit: Arc<Semaphore>,
    pub(crate) shutdown: Notify,
    pub(crate) shutdown_requested: AtomicBool,
}

impl FlowBot {
    /// Run the bot.
    /// This will connect to the server and start processing events.
    /// This method returns `Ok(())` when graceful shutdown is requested via [`shutdown`](Self::shutdown),
    /// or when the connection closes and reconnection is not configured.
    /// It returns `Err` on fatal errors or when reconnection attempts are exhausted.
    pub async fn run(&self) -> Result<(), FlowError> {
        self.shutdown_requested.store(false, Ordering::Relaxed);
        match &self.connection.reconnection {
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

    /// Request graceful shutdown.
    ///
    /// The bot will stop accepting new events and return from [`run`](Self::run)
    /// as soon as the current reconnection sleep or event loop iteration finishes.
    pub fn shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
        self.shutdown.notify_waiters();
    }

    async fn run_once(&self) -> Result<(), FlowError> {
        let (write, read) = self.connect().await?;

        // Connection established successfully, reset attempt counter
        self.reconnect_attempt.store(0, Ordering::Relaxed);

        self.set_sink(write).await;
        self.init_services().await;
        self.run_msg_loop(read).await?;

        Ok(())
    }

    async fn run_with_infinite_reconnect(
        &self,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<(), FlowError> {
        loop {
            if self.shutdown_requested.load(Ordering::Relaxed) {
                return Ok(());
            }

            let attempt = self.reconnect_attempt.load(Ordering::Relaxed);
            let current_delay =
                (initial_delay_ms * 2_u64.saturating_pow(attempt)).min(max_delay_ms);

            match self.run_once().await {
                Ok(()) => {
                    if self.shutdown_requested.load(Ordering::Relaxed) {
                        return Ok(());
                    }
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

            if self.shutdown_requested.load(Ordering::Relaxed) {
                return Ok(());
            }
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(current_delay)) => {}
                _ = self.shutdown.notified() => {
                    return Ok(());
                }
            }
        }
    }

    async fn run_with_limited_reconnect(
        &self,
        max_attempts: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<(), FlowError> {
        loop {
            if self.shutdown_requested.load(Ordering::Relaxed) {
                return Ok(());
            }

            let attempt = self.reconnect_attempt.load(Ordering::Relaxed);

            if attempt >= max_attempts {
                return Err(FlowError::ReconnectionFailed(max_attempts));
            }

            let current_delay =
                (initial_delay_ms * 2_u64.saturating_pow(attempt)).min(max_delay_ms);

            match self.run_once().await {
                Ok(()) => {
                    if self.shutdown_requested.load(Ordering::Relaxed) {
                        return Ok(());
                    }
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

            if self.shutdown_requested.load(Ordering::Relaxed) {
                return Ok(());
            }
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(current_delay)) => {}
                _ = self.shutdown.notified() => {
                    return Ok(());
                }
            }
        }
    }

    async fn connect(
        &self,
    ) -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        FlowError,
    > {
        let mut request = self.connection.target.clone().into_client_request()?;
        if let Some(auth) = &self.connection.auth {
            let value = auth.parse().map_err(|_| FlowError::InvalidAuth)?;
            request.headers_mut().append("Authorization", value);
        }

        let (ws_stream, _) = connect_async(request).await?;
        Ok(ws_stream.split())
    }

    async fn set_sink(
        &self,
        mut sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    ) {
        let (tx, mut rx) = tokio::sync::mpsc::channel(128);

        {
            let mut ws_sink = self.context.sink.lock().await;
            *ws_sink = Some(tx);
        }

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        });
    }

    async fn run_msg_loop(
        &self,
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> Result<(), FlowError> {
        loop {
            if self.shutdown_requested.load(Ordering::Relaxed) {
                return Ok(());
            }

            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(msg) => {
                            let msg = msg?;
                            if let Message::Text(text) = msg {
                                let text = serde_json::from_slice::<Value>(text.as_bytes())?;
                                if let Some(echo) = Self::check_is_echo(&text) {
                                    self.context.on_recv_echo(echo, text.to_string());
                                    continue;
                                }
                                self.handle_event(text)?;
                            }
                        }
                        None => break Ok(()),
                    }
                }
                _ = self.shutdown.notified() => {
                    return Ok(());
                }
            }
        }
    }

    async fn init_services(&self) {
        for processor in self.processors.iter() {
            processor.init(self.context.clone()).await;
        }
    }

    fn handle_event(&self, text: Value) -> Result<(), FlowError> {
        let event: Event = serde_json::from_value(text)?;
        let event = Arc::new(event);
        let context = self.context.clone();
        let processors = self.processors.clone();
        let concurrent_limit = self.concurrent_limit.clone();
        tokio::spawn(async move {
            let _permit = concurrent_limit
                .acquire()
                .await
                .expect("semaphore should not be closed");
            for processor in processors.iter() {
                match processor.process(context.clone(), event.clone()).await {
                    Ok(HandlerControl::Block) => break,
                    Ok(HandlerControl::Continue) => continue,
                    Err(e) => {
                        tracing::debug!("{}", e);
                        continue;
                    }
                }
            }
        });
        Ok(())
    }

    fn check_is_echo(msg: &Value) -> Option<String> {
        if let serde_json::Value::Object(obj) = msg
            && let Some(serde_json::Value::String(echo)) = obj.get("echo")
        {
            return Some(echo.clone());
        }
        None
    }
}
