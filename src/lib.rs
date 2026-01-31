#![feature(try_trait_v2)]
#![feature(adt_const_params)]
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
//! # Example
//! ```no_run
//! use flow_bot::{
//!     FlowBotBuilder,
//!     base::{connect::ReverseConnectionConfig, extract::Message, handler::HandlerControl},
//! };
//!
//! async fn on_message(msg: Message) -> HandlerControl {
//!     println!("{:?}", msg.message);
//!     HandlerControl::Continue
//! }
//!
//! async fn main() {
//!     let bot = FlowBotBuilder::new(ReverseConnectionConfig {
//!         target: "ws://localhost:19999".to_string(),
//!         auth: None,
//!     })
//!     .with_state(())
//!     .with_handler(on_message)
//!     .build();
//!
//!     bot.run().await.unwrap();
//! }
//! ```
//!
//! # Handlers
//!
//! Handlers are functions that can be registered to process events. They can be registered using the [`with_handler`] method.
//! Commonly, a handler responds to a event by calling methods in [`ApiExt`] which is implemented by [`BotContext`] to control the bot.
//!
//! [`with_handler`]: crate::FlowBotBuilder::with_handler
//! [`ApiExt`]: crate::api::api_ext::ApiExt
//! [`BotContext`]: crate::base::context::BotContext
//!
//! The returned value of a handler is a [`HandlerControl`] which determines the flow of the event processing.
//! [`HandlerControl::Continue`] means the event will be passed to the next handler, [`HandlerControl::Block`] means the event will not be passed to the next handler.
//! [`HandlerControl::Skip`] means the event will be passed to the next handler but the event will not be processed by the current handler, used in the case where the event criteria is not met within the handler.
//! It is a crucial difference from many other bot SDKs that we do not provide a matcher machenism to match the event, so that you need to implement the logic in the handler. However, a similar way is mimiced by the extractor mechanism. See the [Extractors] section below.
//!
//! [`HandlerControl`]: crate::base::handler::HandlerControl
//! [`HandlerControl::Continue`]: crate::base::handler::HandlerControl::Continue
//! [`HandlerControl::Block`]: crate::base::handler::HandlerControl::Block
//! [`HandlerControl::Skip`]: crate::base::handler::HandlerControl::Skip
//! [Extractors]: #extractors
//!
//! # Extractors
//! Extractors work similarly to the extractors in axum. They are functions that can be registered to extract data from the event. They are to extract data from the context and event for the handler to use.
//! To see a full list of predefined extractors, see the [`extract`] module.
//!
//! [`extract`]: crate::base::extract
//!
//! ## Using Extractors
//!
//! It is already shown in the example above how to use the predefined [`Message`] extractor which extracts the message from the event. It is also possible to use extractors to match event criteria.
//!
//! [`Message`]: crate::base::extract::Message
//!
//! ```no_run
//! use flow_bot::{
//!    base::extract::MatchGroupId,handler::HandlerControl
//! };
//!
//! async fn on_group_msg(_: MatchGroupId<123>) -> HandlerControl {
//!    // This handler will only be called when the event is a group message in group 123, otherwise it will be skipped.
//!    println!("Received message in group 123");
//!    HandlerControl::Continue
//! }
//! ```
//!
//! ## Optional Extraction
//!
//! Extractors can be optional by using the [`Option`] type. This is useful when the data is not always present in the event.
//!
//! ## Custom Extractors
//!
//! It is also possible to create custom extractors by implementing the [`FromEvent`] trait.
//! This is an async trait that takes the context and event as arguments and returns a result of the extracted data.
//!
//! [`FromEvent`]: crate::base::extract::FromEvent
//!
//! # States
//!
//! States are data that can be shared between handlers. They are stored in the context and can be accessed by any handler.
//! States can be added to the bot using the [`with_state`] method.
//! States can be any type that implements [`std::any::Any`], [`Send`], and [`Sync`].
//!
//! [`with_state`]: crate::FlowBotBuilder::with_state
//!
//! In a handler, a state is accessed by using the [`State`] extractor.
//!
//! [`State`]: crate::base::extract::State
//!
//! There can be multiple states in the bot, each with a unique type.
//! If the required state is not present in the context, the handler will be skipped.
//!
//! # Services
//!
//! Services provide a way to make the bot extendable. They are similar to handlers but take the shape of a struct that implements the [`Service`] trait and have their own state.
//! It is made so that the bot can be extended to use services from other crates with ease.
//! Services can be added to the bot using the [`with_service`] method.
//!
//! [`Service`]: crate::base::service::Service
//! [`with_service`]: crate::FlowBotBuilder::with_service
use std::{any::Any, ops::Deref, sync::Arc};

use base::{
    connect::ReverseConnectionConfig,
    context::{BotContext, Context, StateMap},
    handler::{ErasedHandler, HWrapped, Handler, HandlerControl},
    service::Service,
};
use error::FlowError;
use event::Event;
use futures::{
    StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, Utf8Bytes, client::IntoClientRequest},
};

pub mod api;
pub mod base;
pub mod error;
pub mod event;
pub mod message;

enum HandlerOrService {
    Handler(Box<dyn ErasedHandler>),
    Service(Box<dyn Service>),
}

pub struct FlowBot {
    handlers: Arc<Vec<HandlerOrService>>,
    context: BotContext,
    connection: ReverseConnectionConfig,
}

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
        }
    }
}

impl FlowBot {
    /// Run the bot.
    /// This will connect to the server and start processing events.
    /// This method will never return unless an error occurs.
    pub async fn run(&self) -> Result<(), FlowError> {
        let (write, read) = self.connect().await?;

        self.set_sink(write).await;
        self.run_msg_loop(read).await?;
        self.init_services().await;

        Ok(())
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
            request
                .headers_mut()
                .append("Authorization", auth.parse().unwrap());
        }

        let (ws_stream, _) = connect_async(request).await?;
        Ok(ws_stream.split())
    }

    async fn set_sink(&self, sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) {
        let mut ws_sink = self.context.sink.lock().await;
        *ws_sink = Some(sink);
    }

    async fn run_msg_loop(
        &self,
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> Result<(), FlowError> {
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                if let Some(echo) = Self::check_is_echo(&text) {
                    self.context.on_recv_echo(echo, text.to_string());
                    continue;
                }
                self.handle_event(text)?;
            }
        }
        Ok(())
    }

    async fn init_services(&self) {
        for handler in self.handlers.deref() {
            if let HandlerOrService::Service(service) = handler {
                service.init(self.context.clone()).await;
            }
        }
    }

    fn handle_event(&self, text: Utf8Bytes) -> Result<(), FlowError> {
        let event: Event = serde_json::from_slice(text.as_bytes())?;
        let event = Arc::new(event);
        let context = self.context.clone();
        let handlers = self.handlers.clone();
        tokio::spawn(async move {
            for handler in handlers.deref() {
                let control = match handler {
                    HandlerOrService::Handler(handler) => {
                        handler.call(context.clone(), event.clone()).await
                    }
                    HandlerOrService::Service(service) => {
                        service.serve(context.clone(), event.clone()).await
                    }
                };

                if let HandlerControl::Block = control {
                    break;
                }
            }
        });
        Ok(())
    }

    fn check_is_echo(msg: &str) -> Option<String> {
        let msg = serde_json::from_str::<serde_json::Value>(msg).unwrap();
        if let serde_json::Value::Object(obj) = msg
            && let Some(serde_json::Value::String(echo)) = obj.get("echo")
        {
            return Some(echo.clone());
        }
        None
    }
}
