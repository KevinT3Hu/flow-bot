use async_trait::async_trait;

use crate::{
    base::context::BotContext,
    event::BotEvent,
    extract::{FromEvent, MessageBody},
    message::message_ext::MessageExt,
};

pub struct Command<const PREFIX: &'static str, Cmd> {
    pub command: Cmd,
}

#[async_trait]
impl<const PREFIX: &'static str, Cmd> FromEvent for Command<PREFIX, Cmd>
where
    Cmd: clap::Parser,
{
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        let message_body = MessageBody::from_event(context, event).await?;
        let plain_text = message_body.0.extract_if_plain_text()?;
        let trimmed = plain_text.trim();

        if trimmed.split_whitespace().next() != Some(PREFIX) {
            return None;
        }

        let cmd = Cmd::try_parse_from(trimmed.split_whitespace()).ok()?;
        Some(Self { command: cmd })
    }
}
