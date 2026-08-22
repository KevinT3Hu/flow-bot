use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use serde_json::Value;
use tokio::sync::{Notify, Semaphore, mpsc};

use crate::{
    base::{
        context::BotContext, control::HandlerControl, middleware::EventProcessor,
        transport::ConnectionConfig,
    },
    error::FlowError,
    event::{BotEvent, Event, TypedEvent},
};

/// How events arriving from the connection are dispatched to handlers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DispatchMode {
    /// Events are processed strictly in arrival order: the handler chain of
    /// one event completes before the next event is dispatched. (Default.)
    #[default]
    Ordered,
    /// Events are processed concurrently, bounded by the builder's
    /// [`concurrent_limit`](crate::FlowBotBuilder::concurrent_limit);
    /// ordering between events is not guaranteed.
    Concurrent,
}

/// State shared between the run loop, the transports and the dispatcher task.
pub(crate) struct BotShared {
    shutdown: Notify,
    shutdown_requested: AtomicBool,
    services_initialized: AtomicBool,
}

impl BotShared {
    pub(crate) fn new() -> Self {
        Self {
            shutdown: Notify::new(),
            shutdown_requested: AtomicBool::new(false),
            services_initialized: AtomicBool::new(false),
        }
    }

    pub(crate) fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
        self.shutdown.notify_waiters();
    }

    pub(crate) fn shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Relaxed)
    }

    /// Wait until shutdown is requested. The `notified()` future is created
    /// *before* re-checking the flag, so a shutdown that races with this call
    /// cannot be missed.
    pub(crate) async fn wait_shutdown(&self) {
        let notified = self.shutdown.notified();
        if self.shutdown_requested() {
            return;
        }
        notified.await;
    }

    /// Run `Service::init` for the processor chain exactly once per bot, no
    /// matter how many (re)connections happen.
    pub(crate) async fn init_services_once(
        &self,
        processors: &Arc<Vec<Arc<dyn EventProcessor>>>,
        context: BotContext,
    ) {
        if self.services_initialized.swap(true, Ordering::Relaxed) {
            return;
        }
        for processor in processors.iter() {
            processor.init(context.clone()).await;
        }
    }
}

pub struct FlowBot {
    pub(crate) processors: Arc<Vec<Arc<dyn EventProcessor>>>,
    pub(crate) context: BotContext,
    pub(crate) connection: ConnectionConfig,
    pub(crate) concurrent_limit: Arc<Semaphore>,
    pub(crate) dispatch_mode: DispatchMode,
    pub(crate) event_queue_capacity: usize,
    pub(crate) shared: Arc<BotShared>,
}

impl FlowBot {
    /// Run the bot.
    ///
    /// This starts the configured connection (see [`ConnectionConfig`]),
    /// dispatches incoming events to the handlers, and returns:
    /// - `Ok(())` when graceful shutdown is requested via [`shutdown`](Self::shutdown),
    ///   or when the connection closes and reconnection is not configured;
    /// - `Err` on fatal errors (invalid configuration, bind failures, or
    ///   exhausted reconnection attempts).
    pub async fn run(&self) -> Result<(), FlowError> {
        self.shared
            .shutdown_requested
            .store(false, Ordering::Relaxed);
        self.connection
            .validate()
            .map_err(FlowError::InvalidConfig)?;

        // Bounded event queue: connection frames wait here when handlers are
        // slow, giving backpressure instead of unbounded task growth.
        let (tx, mut rx) = mpsc::channel::<Value>(self.event_queue_capacity);
        let dispatcher = {
            let processors = self.processors.clone();
            let context = self.context.clone();
            let semaphore = self.concurrent_limit.clone();
            let mode = self.dispatch_mode;
            tokio::spawn(async move {
                while let Some(value) = rx.recv().await {
                    let event: BotEvent = match serde_json::from_value::<Event>(value) {
                        Ok(event) => Arc::new(event),
                        Err(e) => {
                            tracing::warn!("dropping event that failed to deserialize: {e}");
                            continue;
                        }
                    };
                    if matches!(event.event, TypedEvent::Unknown(_)) {
                        tracing::debug!(
                            "event of unrecognized shape degraded to the Unknown variant"
                        );
                    }
                    match mode {
                        DispatchMode::Ordered => run_processors(&processors, &context, event).await,
                        DispatchMode::Concurrent => {
                            let permit = semaphore
                                .clone()
                                .acquire_owned()
                                .await
                                .expect("semaphore should not be closed");
                            let processors = processors.clone();
                            let context = context.clone();
                            tokio::spawn(async move {
                                let _permit = permit;
                                run_processors(&processors, &context, event).await;
                            });
                        }
                    }
                }
            })
        };

        let result = match &self.connection {
            ConnectionConfig::ForwardWebSocket(cfg) => {
                crate::base::transport::forward_ws::run(self, cfg, tx.clone()).await
            }
            ConnectionConfig::ReverseWebSocket(cfg) => {
                crate::base::transport::reverse_ws::run(self, cfg, tx.clone()).await
            }
            ConnectionConfig::Http(cfg) => {
                crate::base::transport::http::run(self, cfg, tx.clone()).await
            }
            ConnectionConfig::HttpPost(cfg) => {
                crate::base::transport::http_post::run(self, cfg, tx.clone()).await
            }
        };

        // Close the queue so the dispatcher drains the remaining events and
        // exits; a handler error never aborts the run loop itself.
        drop(tx);
        let _ = dispatcher.await;
        self.context.clear_transport();
        result
    }

    /// Request graceful shutdown.
    ///
    /// The bot stops accepting new events and returns from [`run`](Self::run)
    /// as soon as the current reconnection sleep or event loop iteration
    /// finishes.
    pub fn shutdown(&self) {
        self.shared.request_shutdown();
    }

    /// The shared context of this bot, for calling OneBot APIs (via
    /// [`ApiExt`](crate::api::api_ext::ApiExt)) from outside the run loop.
    pub fn context(&self) -> BotContext {
        self.context.clone()
    }

    pub(crate) fn shutdown_requested(&self) -> bool {
        self.shared.shutdown_requested()
    }

    pub(crate) async fn wait_shutdown(&self) {
        self.shared.wait_shutdown().await
    }

    /// Owned shutdown future for server transports (graceful shutdown).
    pub(crate) fn shutdown_signal(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let shared = self.shared.clone();
        Box::pin(async move {
            shared.wait_shutdown().await;
        })
    }

    pub(crate) async fn init_services_once(&self) {
        self.shared
            .init_services_once(&self.processors, self.context.clone())
            .await;
    }
}

/// Run one event through the processor chain. Handler errors are logged and
/// contained; they never terminate the bot.
async fn run_processors(
    processors: &[Arc<dyn EventProcessor>],
    context: &BotContext,
    event: BotEvent,
) {
    for processor in processors {
        match processor.process(context.clone(), event.clone()).await {
            Ok(HandlerControl::Block) => break,
            Ok(HandlerControl::Continue) => continue,
            Err(e) => {
                tracing::debug!("handler error: {e}");
                continue;
            }
        }
    }
}
