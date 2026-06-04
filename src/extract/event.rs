use async_trait::async_trait;

use crate::{
    base::context::BotContext,
    event::{
        BotEvent, Event, TypedEvent,
        message::{GroupMessageInfo, Message, PrivateMessageInfo, TypedMessageInfo},
        meta_event::{Heartbeat, Lifecycle, MetaEvent},
        notice::{
            FriendAdd, FriendRecall, GroupAdmin, GroupBan, GroupDecrease, GroupIncrease,
            GroupRecall, GroupUpload, Notice,
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

crate::impl_from_event!(Request);
crate::impl_from_event!(Request, Friend, FriendRequest);
crate::impl_from_event!(Request, Group, GroupRequest);

crate::impl_from_event!(MetaEvent);
crate::impl_from_event!(MetaEvent, Lifecycle);
crate::impl_from_event!(MetaEvent, Heartbeat);
