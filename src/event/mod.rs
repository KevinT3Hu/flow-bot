use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use crate::base::{context::BotContext, extract::FromEvent};

pub mod message;
pub mod meta_event;
pub mod notice;
pub mod request;

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "post_type")]
#[serde(rename_all = "snake_case")]
pub enum TypedEvent {
    // use Box to avoid large size differences between variants
    Message(Box<message::Message>),
    Notice(notice::Notice),
    Request(request::Request),
    MetaEvent(meta_event::MetaEvent),
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

impl TypedEvent {
    pub fn get_type(&self) -> &str {
        match self {
            TypedEvent::Message(..) => "message",
            TypedEvent::Notice(..) => "notice",
            TypedEvent::Request(..) => "request",
            TypedEvent::MetaEvent(..) => "meta_event",
            TypedEvent::Unknown(..) => "unknown",
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Event {
    pub time: i64,
    pub self_id: i64,
    #[serde(flatten)]
    pub event: TypedEvent,
}

pub type BotEvent = Arc<Event>;

#[async_trait]
impl FromEvent for BotEvent {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        Some(event)
    }
}

#[macro_export]
macro_rules! impl_from_event {
    ($event_type:ident) => {
        #[::async_trait::async_trait]
        impl $crate::base::extract::FromEvent for $event_type {
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
        impl $crate::base::extract::FromEvent for $variant_type {
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
        impl_from_event!($event_type, $variant, $variant);
    };
}
