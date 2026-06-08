use std::{
    any::Any,
    sync::{Arc, atomic::AtomicBool, atomic::AtomicU32},
};

use tokio::sync::{Notify, Semaphore};

use crate::base::{
    bot::FlowBot,
    connect::ReverseConnectionConfig,
    context::{BotContext, Context, StateMap},
    handler::{HWrapped, Handler, HandlerOrService, Service},
    middleware::{EventProcessor, Leaf, Middleware, Node},
};

pub struct FlowBotBuilder {
    processors: Vec<Arc<dyn EventProcessor>>,
    connection: ReverseConnectionConfig,
    states: StateMap,
    concurrent_limit: usize,
}

impl FlowBotBuilder {
    /// Create a new FlowBotBuilder with the given connection configuration.
    pub fn new(connection: ReverseConnectionConfig) -> Self {
        Self {
            processors: Vec::new(),
            connection,
            states: StateMap::new(),
            concurrent_limit: 8, // default concurrent limit
        }
    }

    /// Add a state to the bot.
    /// If the state of the same type is already present, it will be replaced.
    pub fn with_state<S: 'static + Any + Send + Sync>(mut self, state: S) -> Self {
        self.states.insert(state);
        self
    }

    /// Add a handler to the bot.
    /// The order of the handlers added is the order in which they will be called.
    pub fn with_handler<T, H>(mut self, handler: H) -> Self
    where
        T: Send + Sync + 'static,
        H: Handler<T> + Send + Sync + 'static,
    {
        let wrapped = HWrapped {
            handler,
            _phantom: std::marker::PhantomData,
        };
        self.processors.push(Arc::new(Leaf {
            inner: HandlerOrService::Handler(Box::new(wrapped)),
        }));
        self
    }

    /// Add a service to the bot.
    pub fn with_service<Svc>(mut self, service: Svc) -> Self
    where
        Svc: Service + Send + Sync + 'static,
    {
        self.processors.push(Arc::new(Leaf {
            inner: HandlerOrService::Service(Box::new(service)),
        }));
        self
    }

    /// Apply a middleware layer to **all handlers already added**.
    ///
    /// This works like axum's `.layer()`: it retroactively wraps every
    /// existing handler/service with the given middleware.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// FlowBotBuilder::new(...)
    ///     .with_handler(help_cmd)       // bare help_cmd
    ///     .layer(from_fn(log_all))      // help_cmd is now wrapped with log_all
    ///     .with_handler(ban_cmd)        // bare ban_cmd
    ///     .layer(from_fn(require_admin)) // ban_cmd is wrapped with require_admin;
    ///                                   // help_cmd is wrapped with log_all → require_admin
    ///     .build();
    /// ```
    pub fn layer<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        let middleware = Arc::new(middleware);
        self.processors = self
            .processors
            .into_iter()
            .map(|processor| {
                Arc::new(Node {
                    middleware: middleware.clone(),
                    inner: processor,
                }) as Arc<dyn EventProcessor>
            })
            .collect();
        self
    }

    pub fn concurrent_limit(mut self, limit: usize) -> Self {
        self.concurrent_limit = limit;
        self
    }

    /// Build the FlowBot.
    pub fn build(self) -> FlowBot {
        FlowBot {
            processors: Arc::new(self.processors),
            context: BotContext::new(Context::new(self.states)),
            connection: self.connection,
            reconnect_attempt: AtomicU32::new(0),
            concurrent_limit: Arc::new(Semaphore::new(self.concurrent_limit)),
            shutdown: Notify::new(),
            shutdown_requested: AtomicBool::new(false),
        }
    }
}
