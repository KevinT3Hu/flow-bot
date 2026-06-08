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
//!     base::{
//!         connect::{ReconnectionStrategy, ReverseConnectionConfig},
//!         handler::{HandlerControl, HandlerError},
//!     },
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
//!     let bot = FlowBotBuilder::new(ReverseConnectionConfig {
//!         target: "ws://localhost:19999".to_string(),
//!         auth: None,
//!         reconnection: ReconnectionStrategy::None,
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
//! # Extractors
//! Extractors work similarly to the extractors in axum. They are functions that can be registered to extract data from the event. They are to extract data from the context and event for the handler to use.
//! To see a full list of predefined extractors, see the [`extract`] module.
//!
//! [`extract`]: crate::extract
//!
//! ## Using Extractors
//!
//! It is already shown in the example above how to use the predefined [`Message`] extractor which extracts the message from the event. It is also possible to use extractors to match event criteria.
//!
//! [`Message`]: crate::extract::Message
//!
//! ```no_run
//! use flow_bot::{
//!    extract::MatchGroupId,
//!    base::handler::{HandlerControl, HandlerError},
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
//!
//! Extractors can be optional by using the [`Option`] type. This is useful when the data is not always present in the event.
//!
//! ## Custom Extractors
//!
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
//!
//! [`Service`]: crate::base::service::Service
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
pub use base::{
    bot::FlowBot,
    builder::FlowBotBuilder,
    connect::{ReconnectionStrategy, ReverseConnectionConfig},
    context::{BotContext, BotContextExt},
    control::{HandlerControl, HandlerError, IntoHandlerResult},
    handler::Handler,
    service::Service,
};
pub use error::FlowError;
pub use event::{BotEvent, Event, TypedEvent};
pub use extract::{FromEvent, State};
pub use message::{IntoMessage, Message};
