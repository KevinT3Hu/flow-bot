use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    api::api_ext::ApiExt,
    event::{
        BotEvent, TypedEvent,
        message::{GroupSenderInfo, PrivateSenderInfo, SenderSex},
    },
    message::{self, segments::Segment},
};

use super::context::BotContext;

#[async_trait]
/// Extractor trait for extracting information from BotEvent and BotContext.
pub trait FromEvent {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized;
}

/// State extractor, extract the state from BotContext.
/// If the required state is not found, the handler will be skipped.
pub struct State<S> {
    pub state: Arc<S>,
}

#[async_trait]
impl<S> FromEvent for State<S>
where
    S: 'static + Send + Sync,
{
    async fn from_event(context: BotContext, _: BotEvent) -> Option<Self> {
        let state = context.state.get::<S>()?;
        Some(Self { state })
    }
}

/// Extractor for message event.
pub struct Message {
    pub message: message::Message,
}

#[async_trait]
impl FromEvent for Message {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Message> {
        match event.event {
            crate::event::TypedEvent::Message(ref msg) => Some(Self {
                message: msg.message.clone(),
            }),
            _ => None,
        }
    }
}

unsafe impl Send for Message {}
unsafe impl Sync for Message {}

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

pub struct Sender {
    pub info: BasicSenderInfo,
}

#[async_trait]
impl FromEvent for Sender {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        match event.event {
            TypedEvent::Message(ref msg) => {
                let info = match &msg.info {
                    crate::event::message::TypedMessageInfo::Private(info) => {
                        info.sender.clone().into()
                    }
                    crate::event::message::TypedMessageInfo::Group(info) => {
                        info.sender.clone().into()
                    }
                };
                Some(Self { info })
            }
            _ => None,
        }
    }
}

unsafe impl Send for Sender {}
unsafe impl Sync for Sender {}

pub struct At {
    pub user_id: String,
}

#[async_trait]
impl FromEvent for At {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::At(at) => Some(Self {
                    user_id: at.qq.clone(),
                }),
                _ => None,
            })
        } else {
            None
        }
    }
}

unsafe impl Send for At {}
unsafe impl Sync for At {}

pub struct GroupId {
    pub group_id: i64,
}

#[async_trait]
impl FromEvent for GroupId {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        if let TypedEvent::Message(ref msg) = event.event {
            match &msg.info {
                crate::event::message::TypedMessageInfo::Group(info) => Some(Self {
                    group_id: info.group_id,
                }),
                _ => None,
            }
        } else {
            None
        }
    }
}

unsafe impl Send for GroupId {}
unsafe impl Sync for GroupId {}

pub struct SenderId {
    pub user_id: i64,
}

#[async_trait]
impl FromEvent for SenderId {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        let sender_info = Sender::from_event(context, event).await?;
        Some(Self {
            user_id: sender_info.info.user_id?,
        })
    }
}

unsafe impl Send for SenderId {}
unsafe impl Sync for SenderId {}

pub struct MatchGroupId<const ID: i64>;

#[async_trait]
impl<const ID: i64> FromEvent for MatchGroupId<ID> {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        let group_id = GroupId::from_event(context, event).await?.group_id;
        if group_id == ID { Some(Self) } else { None }
    }
}

unsafe impl<const ID: i64> Send for MatchGroupId<ID> {}
unsafe impl<const ID: i64> Sync for MatchGroupId<ID> {}

pub struct Reply {
    pub reply: message::Message,
}

#[async_trait]
impl FromEvent for Reply {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        if let TypedEvent::Message(ref msg) = event.event {
            for segment in msg.message.iter() {
                if let Segment::Reply(reply) = segment {
                    let id = reply.id.parse::<i64>().ok()?;
                    let message = context
                        .get_message(id)
                        .await
                        .map(|msg| msg.message.clone())
                        .ok()?;
                    return Some(Self { reply: message });
                }
            }
        }
        None
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

/// A helper macro for matching one of the variants.
/// The macro will generate an enum with the given name and variants.
/// The enum will implement the FromEvent trait, and will try to match the event with the given matchers.
///
/// # Example
/// ```no_run
/// match_one!(MatchOne, A: AMatcher, B: BMatcher);
/// ```
/// The above code will generate an enum like this:
/// ```no_run
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
        impl $crate::base::extract::FromEvent for $name {
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
