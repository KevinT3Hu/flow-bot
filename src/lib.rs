#![cfg_attr(feature = "nightly", feature(adt_const_params))]
#![cfg_attr(feature = "nightly", feature(unsized_const_params))]

//! An onebot-11 SDK that simplifies bot creation.
//!
//! Flow-bot is carefully crafted to provide a mechanism similar to that of axum so if you are familiar with axum, you will find it easy to use.
//!
//! The basic unit of event processing in flow-bot is a handler. A handler is a function that optionally takes [`BotContext`] and a [`BotEvent`] or any of the extractors as arguments and returns a [`Result<HandlerControl, HandlerError>`].
//! Handlers can parse the incoming event and respond to it. The returned value serves as a control flow signal to determine the flow of the event processing which is where the name comes from.
//!
//! [`BotContext`]: crate::base::context::BotContext
//! [`BotEvent`]: crate::event::BotEvent
//!
//! # Example
//! ```no_run
//! use flow_bot::{
//!     FlowBotBuilder,
//!     base::transport::{ConnectionConfig, ForwardWebSocketConfig, ReconnectionStrategy},
//!     HandlerControl, HandlerError,
//!     extract::Message,
//! };
//!
//! async fn on_message(msg: Message) -> Result<HandlerControl, HandlerError> {
//!     println!("{:?}", msg.message);
//!     Ok(HandlerControl::Continue)
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() {
//!     let bot = FlowBotBuilder::new(ConnectionConfig::ForwardWebSocket(ForwardWebSocketConfig {
//!         url: "ws://localhost:19999".to_string(),
//!         access_token: None,
//!         reconnection: ReconnectionStrategy::None,
//!     }))
//!     .with_state(())
//!     .with_handler(on_message)
//!     .build();
//!
//!     bot.run().await.unwrap();
//! }
//! ```
//!
//! # Connections
//!
//! All four OneBot 11 communication types are supported through
//! [`ConnectionConfig`], behind one unified surface: handlers, extractors,
//! services and [`ApiExt`] API calls work identically regardless of the
//! transport.
//!
//! - [`ConnectionConfig::ForwardWebSocket`] — flow-bot connects, as a client,
//!   to the implementation's WebSocket server (`/`, `/api` or `/event`
//!   endpoint). Reconnection is configured per connection.
//! - [`ConnectionConfig::ReverseWebSocket`] — flow-bot runs a WebSocket
//!   server and the implementation connects to it (announcing itself with
//!   `X-Self-ID`/`X-Client-Role` headers). The implementation is responsible
//!   for reconnecting.
//! - [`ConnectionConfig::Http`] — flow-bot calls the implementation's HTTP
//!   API. This type receives no events.
//! - [`ConnectionConfig::HttpPost`] — flow-bot runs an HTTP server receiving
//!   event POSTs (with optional `X-Signature` verification); outbound API
//!   calls optionally use a separate [`HttpConfig`] endpoint.
//!
//! [`ConnectionConfig`]: crate::base::transport::ConnectionConfig
//! [`ApiExt`]: crate::api::api_ext::ApiExt
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
//! The returned value of a handler is a [`Result<HandlerControl, HandlerError>`] which determines the flow of the event processing.
//! [`HandlerControl::Continue`] means the event will be passed to the next handler, [`HandlerControl::Block`] means the event will not be passed to the next handler.
//! [`HandlerError`] means the event will be passed to the next handler but the current handler will not process it, used in the case where the event criteria is not met within the handler.
//! It is a crucial difference from many other bot SDKs that we do not provide a matcher machenism to match the event, so that you need to implement the logic in the handler. However, a similar way is mimiced by the extractor mechanism. See the [Extractors] section below.
//!
//! [`HandlerControl`]: crate::base::control::HandlerControl
//! [`HandlerControl::Continue`]: crate::base::control::HandlerControl::Continue
//! [`HandlerControl::Block`]: crate::base::control::HandlerControl::Block
//! [`HandlerError`]: crate::base::control::HandlerError
//! [Extractors]: #extractors
//!
//! By default events are dispatched to handlers strictly in arrival order; see
//! [`DispatchMode`] if you need bounded concurrent processing instead.
//!
//! [`DispatchMode`]: crate::base::bot::DispatchMode
//!
//! # Messages
//!
//! A [`Message`] is a sequence of message segments. On the wire the OneBot 11
//! spec allows `message`-typed fields as a CQ-code string, a segment array,
//! or a single segment object; [`Message`] deserializes from all three (so
//! implementations configured with `event.message_format: string` work) and
//! always serializes to the array form. `Display` renders the CQ-code string
//! form (also available as [`message::cq::to_cq_string`]), and
//! [`message::cq::parse_cq`] parses one by hand.
//!
//! Segment types not in the OneBot 11 standard set deserialize into
//! [`Segment::Unknown`](message::segments::Segment::Unknown) instead of
//! failing the surrounding message, and message events tolerate omitted
//! optional fields (e.g. `font`).
//!
//! # Extractors
//! Extractors work similarly to the extractors in axum. They are functions that can be registered to extract data from the event. They are to extract data from the context and event for the handler to use.
//! To see a full list of predefined extractors, see the [`extract`] module.
//!
//! [`extract`]: crate::extract
//!
//! ## Using Extractors
//! It is already shown in the example above how to use the predefined [`Message`] extractor which extracts the message from the event. It is also possible to use extractors to match event criteria.
//!
//! [`Message`]: crate::extract::Message
//!
//! ```no_run
//! use flow_bot::{
//!    extract::MatchGroupId,
//!    HandlerControl, HandlerError,
//! };
//!
//! async fn on_group_msg(_: MatchGroupId<123>) -> Result<HandlerControl, HandlerError> {
//!    // This handler will only be called when the event is a group message in group 123, otherwise it will be skipped.
//!    println!("Received message in group 123");
//!    Ok(HandlerControl::Continue)
//! }
//! ```
//!
//! ## Optional Extraction
//! Extractors can be optional by using the [`Option`] type. This is useful when the data is not always present in the event.
//!
//! ## Custom Extractors
//! It is also possible to create custom extractors by implementing the [`FromEvent`] trait.
//! This is an async trait that takes the context and event as arguments and returns a result of the extracted data.
//!
//! [`FromEvent`]: crate::extract::FromEvent
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
//! [`State`]: crate::extract::State
//!
//! There can be multiple states in the bot, each with a unique type.
//! If the required state is not present in the context, the handler will be skipped.
//!
//! # Services
//!
//! Services provide a way to make the bot extendable. They are similar to handlers but take the shape of a struct that implements the [`Service`] trait and have their own state.
//! It is made so that the bot can be extended to use services from other crates with ease.
//! Services can be added to the bot using the [`with_service`] method.
//! `Service::init` runs exactly once per bot, no matter how often the connection is re-established.
//!
//! [`Service`]: crate::base::handler::Service
//! [`with_service`]: crate::FlowBotBuilder::with_service
pub mod api;
pub mod base;
pub mod error;
pub mod event;
pub mod extensions;
pub mod extract;
pub mod message;

#[cfg(feature = "macros")]
pub use flow_bot_macros::{flow_filter, flow_service, group_message, private_message};

// Re-exports of commonly used types for convenience.
pub use api::{MessageTarget, QuickOperation};
pub use base::{
    bot::{DispatchMode, FlowBot},
    builder::FlowBotBuilder,
    context::{BotContext, BotContextExt},
    control::{HandlerControl, HandlerError, IntoHandlerResult},
    handler::{Handler, Service},
    middleware::{Middleware, Next, from_fn},
    transport::{
        ConnectionConfig, ForwardWebSocketConfig, HttpConfig, HttpPostConfig, ReconnectionStrategy,
        ReverseWebSocketConfig,
    },
};
pub use error::FlowError;
pub use event::{BotEvent, Event, TypedEvent};
pub use extract::{FromEvent, State};
pub use message::{IntoMessage, Message};
