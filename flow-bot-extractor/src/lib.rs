//! Event extraction traits for flow-bot
//!
//! This crate provides the `FromEvent` trait for extracting data from OneBot-11 events.

use async_trait::async_trait;
use flow_bot_onebot11::event::{
    Event, TypedEvent,
    message::{
        GroupMessageInfo, GroupSenderInfo, GroupSenderRole, Message, PrivateMessageInfo,
        PrivateSenderInfo, SenderSex, TypedMessageInfo,
    },
    meta_event::{Heartbeat, Lifecycle, MetaEvent},
    notice::{
        FriendAdd, FriendRecall, GroupAdmin, GroupBan, GroupDecrease, GroupIncrease, GroupRecall,
        GroupUpload, Notice,
    },
    request::{FriendRequest, GroupRequest, Request},
};
use flow_bot_onebot11::message::{Message as OBMessage, segments::Segment};
use std::sync::Arc;

pub type BotEvent = Arc<Event>;

/// Trait for extracting data from events.
///
/// This trait allows types to be extracted from events in a type-safe manner.
/// Implementations should return `Some(Self)` if the extraction is successful,
/// or `None` if the event doesn't match the expected type.
#[async_trait]
pub trait FromEvent {
    /// Extract this type from an event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to extract from
    ///
    /// # Returns
    ///
    /// `Some(Self)` if extraction succeeds, `None` otherwise
    async fn from_event(event: BotEvent) -> Option<Self>
    where
        Self: Sized;
}

#[async_trait]
impl FromEvent for BotEvent {
    async fn from_event(event: BotEvent) -> Option<Self> {
        Some(event)
    }
}

// Message event implementations
#[async_trait]
impl FromEvent for Message {
    async fn from_event(event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(message) = &event.event {
            Some((**message).clone())
        } else {
            None
        }
    }
}

#[async_trait]
impl FromEvent for PrivateMessageInfo {
    async fn from_event(event: BotEvent) -> Option<Self> {
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
    async fn from_event(event: BotEvent) -> Option<Self> {
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

// Meta event implementations
#[async_trait]
impl FromEvent for MetaEvent {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::MetaEvent(inner) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for Lifecycle {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::MetaEvent(MetaEvent::Lifecycle(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for Heartbeat {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::MetaEvent(MetaEvent::Heartbeat(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

// Notice event implementations
#[async_trait]
impl FromEvent for Notice {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(inner) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupUpload {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::GroupUpload(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupAdmin {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::GroupAdmin(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupDecrease {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::GroupDecrease(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupIncrease {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::GroupIncrease(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupBan {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::GroupBan(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for FriendAdd {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::FriendAdd(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupRecall {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::GroupRecall(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for FriendRecall {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Notice(Notice::FriendRecall(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

// Request event implementations
#[async_trait]
impl FromEvent for Request {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Request(inner) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for FriendRequest {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Request(Request::Friend(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupRequest {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Request(Request::Group(inner)) => Some(inner.clone()),
            _ => None,
        }
    }
}

// Additional context-independent extractors

/// Extractor for message body (the actual message content).
pub struct MessageBody(pub OBMessage);

#[async_trait]
impl FromEvent for MessageBody {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Message(msg) => Some(Self(msg.message.clone())),
            _ => None,
        }
    }
}

/// Extractor for group sender role.
#[async_trait]
impl FromEvent for GroupSenderRole {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Message(msg) => match &msg.info {
                TypedMessageInfo::Group(info) => info.sender.role.clone(),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Extractor for group sender information.
#[async_trait]
impl FromEvent for GroupSenderInfo {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Message(msg) => match &msg.info {
                TypedMessageInfo::Group(info) => Some(info.sender.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Extractor for private sender information.
#[async_trait]
impl FromEvent for PrivateSenderInfo {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Message(msg) => match &msg.info {
                TypedMessageInfo::Private(info) => Some(info.sender.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Basic sender information that works for both private and group messages.
pub struct BasicSenderInfo {
    pub user_id: Option<i64>,
    pub nickname: Option<String>,
    pub sex: Option<SenderSex>,
    pub age: Option<i32>,
}

impl From<PrivateSenderInfo> for BasicSenderInfo {
    fn from(info: PrivateSenderInfo) -> Self {
        Self {
            user_id: info.user_id,
            nickname: info.nickname,
            sex: info.sex,
            age: info.age,
        }
    }
}

impl From<GroupSenderInfo> for BasicSenderInfo {
    fn from(info: GroupSenderInfo) -> Self {
        Self {
            user_id: info.user_id,
            nickname: info.nickname,
            sex: info.sex,
            age: info.age,
        }
    }
}

/// Extractor for sender information (works for both private and group messages).
pub struct Sender(pub BasicSenderInfo);

#[async_trait]
impl FromEvent for Sender {
    async fn from_event(event: BotEvent) -> Option<Self> {
        match &event.event {
            TypedEvent::Message(msg) => {
                let info = match &msg.info {
                    TypedMessageInfo::Private(info) => info.sender.clone().into(),
                    TypedMessageInfo::Group(info) => info.sender.clone().into(),
                };
                Some(Self(info))
            }
            _ => None,
        }
    }
}

/// Extractor for @mention in messages.
pub struct At(pub String);

#[async_trait]
impl FromEvent for At {
    async fn from_event(event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::At(at) => Some(Self(at.qq.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extractor for group ID.
pub struct GroupId(pub i64);

#[async_trait]
impl FromEvent for GroupId {
    async fn from_event(event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            match &msg.info {
                TypedMessageInfo::Group(info) => Some(Self(info.group_id)),
                _ => None,
            }
        } else {
            None
        }
    }
}

/// Extractor for sender user ID.
pub struct SenderId(pub i64);

#[async_trait]
impl FromEvent for SenderId {
    async fn from_event(event: BotEvent) -> Option<Self> {
        let sender_info = Sender::from_event(event).await?;
        Some(Self(sender_info.0.user_id?))
    }
}

/// Extractor that matches a specific group ID.
pub struct MatchGroupId<const ID: i64>;

#[async_trait]
impl<const ID: i64> FromEvent for MatchGroupId<ID> {
    async fn from_event(event: BotEvent) -> Option<Self> {
        let group_id = GroupId::from_event(event).await?.0;
        if group_id == ID { Some(Self) } else { None }
    }
}
