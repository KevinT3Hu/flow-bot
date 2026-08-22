use std::{any::Any, sync::Arc, time::Duration};

use tokio::sync::Semaphore;

use crate::base::{
    bot::{BotShared, DispatchMode, FlowBot},
    context::{BotContext, Context, StateMap},
    handler::{HWrapped, Handler, HandlerOrService, Service},
    middleware::{EventProcessor, Leaf, Middleware, Node},
    transport::ConnectionConfig,
};

pub struct FlowBotBuilder {
    processors: Vec<Arc<dyn EventProcessor>>,
    connection: ConnectionConfig,
    states: StateMap,
    concurrent_limit: usize,
    dispatch_mode: DispatchMode,
    event_queue_capacity: usize,
    api_timeout: Duration,
}

impl FlowBotBuilder {
    /// Create a new FlowBotBuilder with the given connection configuration.
    ///
    /// # Panics
    /// Panics in [`build`](Self::build) if the configuration is invalid
    /// (malformed URL, wrong scheme, unusable path, TLS URL without the `tls`
    /// feature, ...). Use [`ConnectionConfig::validate`] to check ahead of time.
    pub fn new(connection: ConnectionConfig) -> Self {
        Self {
            processors: Vec::new(),
            connection,
            states: StateMap::new(),
            concurrent_limit: 8,
            dispatch_mode: DispatchMode::default(),
            event_queue_capacity: 256,
            api_timeout: Duration::from_secs(30),
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

    /// Set how events are dispatched to handlers (see [`DispatchMode`]).
    ///
    /// Defaults to [`DispatchMode::Ordered`]: events are processed strictly in
    /// arrival order.
    pub fn dispatch_mode(mut self, mode: DispatchMode) -> Self {
        self.dispatch_mode = mode;
        self
    }

    /// Set the maximum number of events processed concurrently.
    ///
    /// Only takes effect with [`DispatchMode::Concurrent`].
    ///
    /// # Panics
    /// Panics in [`build`](Self::build) if `limit` is 0 (a zero-permit
    /// semaphore would stall every event forever).
    pub fn concurrent_limit(mut self, limit: usize) -> Self {
        self.concurrent_limit = limit;
        self
    }

    /// Set the capacity of the bounded event queue connecting the connection
    /// to the dispatcher. When handlers are slower than the incoming event
    /// rate, the queue fills up and the connection experiences backpressure
    /// instead of unbounded memory growth.
    ///
    /// # Panics
    /// Panics in [`build`](Self::build) if `capacity` is 0.
    pub fn event_queue_capacity(mut self, capacity: usize) -> Self {
        self.event_queue_capacity = capacity;
        self
    }

    /// Set the timeout for OneBot API calls (default: 30 seconds).
    pub fn api_timeout(mut self, timeout: Duration) -> Self {
        self.api_timeout = timeout;
        self
    }

    /// Build the FlowBot.
    ///
    /// # Panics
    /// Panics if the connection configuration is invalid or a capacity/limit
    /// was set to 0.
    pub fn build(self) -> FlowBot {
        if let Err(err) = self.connection.validate() {
            panic!("invalid connection configuration: {err}");
        }
        assert!(
            self.concurrent_limit > 0,
            "concurrent_limit must be greater than 0"
        );
        assert!(
            self.event_queue_capacity > 0,
            "event_queue_capacity must be greater than 0"
        );

        FlowBot {
            processors: Arc::new(self.processors),
            context: BotContext::new(Context::new(self.states, self.api_timeout)),
            connection: self.connection,
            concurrent_limit: Arc::new(Semaphore::new(self.concurrent_limit)),
            dispatch_mode: self.dispatch_mode,
            event_queue_capacity: self.event_queue_capacity,
            shared: Arc::new(BotShared::new()),
        }
    }
}
