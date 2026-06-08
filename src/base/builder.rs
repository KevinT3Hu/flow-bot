use std::{
    any::Any,
    sync::{Arc, atomic::AtomicU32},
};

use crate::base::{
    bot::{FlowBot, HandlerOrService},
    connect::ReverseConnectionConfig,
    context::{BotContext, Context, StateMap},
    handler::{HWrapped, Handler},
    service::Service,
};

pub struct FlowBotBuilder {
    handlers: Vec<HandlerOrService>,
    connection: ReverseConnectionConfig,
    states: StateMap,
}

impl FlowBotBuilder {
    /// Create a new FlowBotBuilder with the given connection configuration.
    pub fn new(connection: ReverseConnectionConfig) -> Self {
        Self {
            handlers: Vec::new(),
            connection,
            states: StateMap::new(),
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
        self.handlers
            .push(HandlerOrService::Handler(Box::new(wrapped)));
        self
    }

    /// Add a service to the bot.
    pub fn with_service<Svc>(mut self, service: Svc) -> Self
    where
        Svc: Service + Send + Sync + 'static,
    {
        self.handlers
            .push(HandlerOrService::Service(Box::new(service)));
        self
    }

    /// Build the FlowBot.
    pub fn build(self) -> FlowBot {
        FlowBot {
            handlers: Arc::new(self.handlers),
            context: BotContext::new(Context::new(self.states)),
            connection: self.connection,
            reconnect_attempt: AtomicU32::new(0),
        }
    }
}
