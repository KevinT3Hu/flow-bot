use async_trait::async_trait;

use crate::{
    base::context::BotContext,
    event::{
        BotEvent, Event, TypedEvent,
        message::{GroupMessageInfo, Message, PrivateMessageInfo, TypedMessageInfo},
        meta_event::{Heartbeat, Lifecycle, MetaEvent},
        notice::{
            EssenceEvent, FriendAdd, FriendRecall, GroupAdmin, GroupBan, GroupDecrease,
            GroupIncrease, GroupRecall, GroupUpload, HonorEvent, LuckyKingEvent, Notice,
            NotifyEvent, PokeEvent,
        },
        request::{FriendRequest, GroupRequest, Request},
    },
    extract::FromEvent,
};

#[async_trait]
impl FromEvent for BotEvent {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        Some(event)
    }
}

#[async_trait]
impl FromEvent for Event {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        Some((*event).clone())
    }
}

#[async_trait]
impl FromEvent for TypedEvent {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        Some(event.event.clone())
    }
}

#[async_trait]
impl FromEvent for Message {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(message) = &event.event {
            Some((**message).clone())
        } else {
            None
        }
    }
}

#[async_trait]
impl FromEvent for PrivateMessageInfo {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            match &msg.info {
                TypedMessageInfo::Private(info) => Some(info.clone()),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[async_trait]
impl FromEvent for GroupMessageInfo {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            match &msg.info {
                TypedMessageInfo::Group(info) => Some(info.clone()),
                _ => None,
            }
        } else {
            None
        }
    }
}

crate::impl_from_event!(Notice);
crate::impl_from_event!(Notice, GroupUpload);
crate::impl_from_event!(Notice, GroupAdmin);
crate::impl_from_event!(Notice, GroupDecrease);
crate::impl_from_event!(Notice, GroupIncrease);
crate::impl_from_event!(Notice, GroupBan);
crate::impl_from_event!(Notice, FriendAdd);
crate::impl_from_event!(Notice, GroupRecall);
crate::impl_from_event!(Notice, FriendRecall);
crate::impl_from_event!(Notice, Essence, EssenceEvent);

// The notify sub-events sit one level deeper than `impl_from_event!` can
// match (TypedEvent::Notice(Notice::Notify(NotifyEvent::..))), so they get
// hand-written impls.
#[async_trait]
impl FromEvent for NotifyEvent {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::Notify(notify)) => Some(notify.clone()),
            _ => None,
        }
    }
}

macro_rules! impl_notify_sub_event {
    ($($(#[$meta:meta])* $variant:ident, $ty:ident),* $(,)?) => {
        $(
            $(#[$meta])*
            #[async_trait]
            impl FromEvent for $ty {
                async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
                    match &event.event {
                        TypedEvent::Notice(Notice::Notify(NotifyEvent::$variant(inner))) => {
                            Some(inner.clone())
                        }
                        _ => None,
                    }
                }
            }
        )*
    };
}

impl_notify_sub_event! {
    #[doc = "Extract a poke notify sub-event (`sub_type: poke`)."]
    Poke, PokeEvent,
    #[doc = "Extract a lucky-king notify sub-event (`sub_type: lucky_king`)."]
    LuckyKing, LuckyKingEvent,
    #[doc = "Extract a honor-change notify sub-event (`sub_type: honor`)."]
    Honor, HonorEvent,
}

crate::impl_from_event!(Request);
crate::impl_from_event!(Request, Friend, FriendRequest);
crate::impl_from_event!(Request, Group, GroupRequest);

crate::impl_from_event!(MetaEvent);
crate::impl_from_event!(MetaEvent, Lifecycle);
crate::impl_from_event!(MetaEvent, Heartbeat);
