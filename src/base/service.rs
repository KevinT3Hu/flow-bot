use async_trait::async_trait;

use crate::event::BotEvent;

use super::{context::BotContext, handler::HandlerControl};

#[async_trait]
pub trait Service: Send + Sync {
    /// Extractors are not possible to be used in services but you can call [`FromEvent::from_event`] manually.
    ///
    /// [`FromEvent::from_event`]: crate::base::extract::FromEvent::from_event
    async fn serve(&self, context: BotContext, event: BotEvent) -> HandlerControl;
}
