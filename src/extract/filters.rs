use async_trait::async_trait;

use crate::{base::context::{BotContext, BotContextExt}, event::{BotEvent, message::{GroupMessageInfo, PrivateMessageInfo}}, extract::{FromEvent, Message}};

pub struct IsGroupMessage;

#[async_trait]
impl FromEvent for IsGroupMessage {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self> {
        context.extract::<GroupMessageInfo>(event).await.map(|_| Self)
    }
}

pub struct IsPrivateMessage;

#[async_trait]
impl FromEvent for IsPrivateMessage {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self> {
        context.extract::<PrivateMessageInfo>(event).await.map(|_| Self)
    }
}
