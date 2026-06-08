pub mod command;
pub mod event;
pub mod filters;
pub mod message;
pub mod segment;

use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;

use crate::{base::context::BotContext, event::BotEvent};

#[async_trait]
/// Extractor trait for extracting information from BotEvent and BotContext.
pub trait FromEvent {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized;
}

/// Extract `time` field from the event.
pub struct EventTime(pub i64);

#[async_trait]
impl FromEvent for EventTime {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        Some(Self(event.time))
    }
}

/// Extract `self_id` field from the event.
pub struct SelfId(pub i64);

#[async_trait]
impl FromEvent for SelfId {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        Some(Self(event.self_id))
    }
}

/// State extractor, extract the state from BotContext.
/// If the required state is not found, the handler will be skipped.
pub struct State<S>(pub Arc<S>);

#[async_trait]
impl<S> FromEvent for State<S>
where
    S: 'static + Send + Sync,
{
    async fn from_event(context: BotContext, _: BotEvent) -> Option<Self> {
        let state = context.state.get::<S>()?;
        Some(Self(state))
    }
}

impl<S> Deref for State<S> {
    type Target = Arc<S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl FromEvent for BotContext {
    async fn from_event(context: BotContext, _: BotEvent) -> Option<Self> {
        Some(context)
    }
}

#[async_trait]
impl<T> FromEvent for Option<T>
where
    T: FromEvent,
{
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        Some(T::from_event(context, event).await)
    }
}

/// A helper macro for implementing `FromEvent` for event types and their variants.
#[macro_export]
macro_rules! impl_from_event {
    ($event_type:ident) => {
        #[::async_trait::async_trait]
        impl $crate::extract::FromEvent for $event_type {
            async fn from_event(
                _: $crate::base::context::BotContext,
                event: $crate::event::BotEvent,
            ) -> Option<Self> {
                match &event.event {
                    $crate::event::TypedEvent::$event_type(inner) => Some(inner.clone()),
                    _ => None,
                }
            }
        }
    };

    ($event_type:ident,$variant:ident, $variant_type:ident) => {
        #[::async_trait::async_trait]
        impl $crate::extract::FromEvent for $variant_type {
            async fn from_event(
                _: $crate::base::context::BotContext,
                event: $crate::event::BotEvent,
            ) -> Option<Self> {
                match &event.event {
                    $crate::event::TypedEvent::$event_type($event_type::$variant(inner)) => {
                        Some(inner.clone())
                    }
                    _ => None,
                }
            }
        }
    };

    ($event_type:ident, $variant:ident) => {
        $crate::impl_from_event!($event_type, $variant, $variant);
    };
}

/// A helper macro for matching one of the variants.
/// The macro will generate an enum with the given name and variants.
/// The enum will implement the FromEvent trait, and will try to match the event with the given matchers.
///
/// # Example
/// ```ignore
/// match_one!(MatchOne, A: AMatcher, B: BMatcher);
/// ```
/// The above code will generate an enum like this:
/// ```ignore
/// pub enum MatchOne {
///    A(AMatcher),
///    B(BMatcher),
/// }
/// ```
#[macro_export]
macro_rules! match_one {
    ($name:ident,$($variant:ident : $matcher:ty),*) => {
        pub enum $name {
            $(
                $variant($matcher),
            )*
        }

        #[async_trait::async_trait]
        impl $crate::extract::FromEvent for $name {
            async fn from_event(context: $crate::base::context::BotContext, event: $crate::event::BotEvent) -> Option<Self> {
                $(
                    if let Some(matcher) = <$matcher>::from_event(context.clone(), event.clone()).await {
                        return Some(Self::$variant(matcher));
                    }
                )*
                None
            }
        }
    };
}

// Re-exports for convenience
pub use self::{
    command::Command,
    message::{
        BasicSenderInfo, Font, GroupId, MatchGroupId, MessageBody, MessageId, RawMessage, Sender,
        SenderId, UserId,
    },
    segment::{
        At, Dice, Face, Forward, Image, Json, Location, Music, Node, PlainText, Poke, Record,
        Reply, Shake, Share, Text, Video, Xml,
    },
};
pub use crate::event::message::Message;
