use async_trait::async_trait;

use crate::{
    base::context::BotContext,
    event::{
        BotEvent, TypedEvent,
        message::{
            GroupAnonymousInfo, GroupSenderInfo, GroupSenderRole, GroupSubType,
            PrivateSenderInfo, PrivateSubType, SenderSex, TypedMessageInfo,
        },
    },
    extract::FromEvent,
    message::{self},
};

/// Extractor for the message body (segments) from a message event.
pub struct MessageBody(pub message::Message);

#[async_trait]
impl FromEvent for MessageBody {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<MessageBody> {
        match event.event {
            TypedEvent::Message(ref msg) => Some(Self(msg.message.clone())),
            _ => None,
        }
    }
}

/// Extract the message id from a message event.
pub struct MessageId(pub i32);

#[async_trait]
impl FromEvent for MessageId {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => Some(Self(msg.message_id)),
            _ => None,
        }
    }
}

/// Extract the user id from a message event.
pub struct UserId(pub i64);

#[async_trait]
impl FromEvent for UserId {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => Some(Self(msg.user_id)),
            _ => None,
        }
    }
}

/// Extract the raw message string from a message event.
pub struct RawMessage(pub String);

#[async_trait]
impl FromEvent for RawMessage {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => Some(Self(msg.raw_message.clone())),
            _ => None,
        }
    }
}

/// Extract the font field from a message event.
pub struct Font(pub i32);

#[async_trait]
impl FromEvent for Font {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => Some(Self(msg.font)),
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for PrivateSubType {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => match &msg.info {
                TypedMessageInfo::Private(info) => Some(info.sub_type.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupSubType {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => match &msg.info {
                TypedMessageInfo::Group(info) => Some(info.sub_type.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for SenderSex {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => match &msg.info {
                TypedMessageInfo::Private(info) => info.sender.sex.clone(),
                TypedMessageInfo::Group(info) => info.sender.sex.clone(),
            },
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupSenderRole {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => match &msg.info {
                TypedMessageInfo::Group(info) => info.sender.role.clone(),
                _ => None,
            },
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for GroupSenderInfo {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => match &msg.info {
                TypedMessageInfo::Group(info) => Some(info.sender.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

#[async_trait]
impl FromEvent for PrivateSenderInfo {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => match &msg.info {
                TypedMessageInfo::Private(info) => Some(info.sender.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

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

pub struct Sender(pub BasicSenderInfo);

#[async_trait]
impl FromEvent for Sender {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => {
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

pub struct GroupId(pub i64);

#[async_trait]
impl FromEvent for GroupId {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
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

#[async_trait]
impl FromEvent for GroupAnonymousInfo {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            match &msg.info {
                TypedMessageInfo::Group(info) => info.anonymous.clone(),
                _ => None,
            }
        } else {
            None
        }
    }
}

pub struct SenderId(pub i64);

#[async_trait]
impl FromEvent for SenderId {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        let sender_info = Sender::from_event(context, event).await?;
        Some(Self(sender_info.0.user_id?))
    }
}

pub struct MatchGroupId<const ID: i64>;

#[async_trait]
impl<const ID: i64> FromEvent for MatchGroupId<ID> {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        let group_id = GroupId::from_event(context, event).await?.0;
        if group_id == ID { Some(Self) } else { None }
    }
}
