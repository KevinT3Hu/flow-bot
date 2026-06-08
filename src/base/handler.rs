use crate::{event::BotEvent, extract::FromEvent};
use async_trait::async_trait;
use std::future::Future;

use super::{
    context::BotContext,
    control::{HandlerControl, HandlerError},
};

#[async_trait]
pub trait Handler<T> {
    async fn handle(
        &self,
        context: BotContext,
        event: BotEvent,
    ) -> Result<HandlerControl, HandlerError>;
}

macro_rules! impl_handler {
    ([$($ty:ident),*]) => {
        #[allow(unused_variables, unused_mut,unused_parens,non_snake_case)]
        #[async_trait]
        impl<F,Fut, $($ty),*> Handler<($($ty),*)> for F
        where
            F: Fn($($ty),*) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Result<HandlerControl, HandlerError>> + Send + 'static,
            $($ty: FromEvent+Send),*
        {
            async fn handle(&self, context: BotContext, event: BotEvent) -> Result<HandlerControl, HandlerError> {
                match ($($ty::from_event(context.clone(), event.clone()).await,)*) {
                    ($(Some($ty),)*) => self($($ty),*).await,
                    _ => Err(HandlerError::skip()),
                }
            }
        }
    };
}

#[async_trait]
pub(crate) trait ErasedHandler: Send + Sync {
    async fn call(
        &self,
        context: BotContext,
        event: BotEvent,
    ) -> Result<HandlerControl, HandlerError>;
}

pub(crate) struct HWrapped<T, H> {
    pub handler: H,
    pub _phantom: std::marker::PhantomData<T>,
}

#[async_trait]
impl<H, T> ErasedHandler for HWrapped<T, H>
where
    H: Handler<T> + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    async fn call(
        &self,
        context: BotContext,
        event: BotEvent,
    ) -> Result<HandlerControl, HandlerError> {
        self.handler.handle(context, event).await
    }
}

macro_rules! all_tuples {
    ($macro:ident) => {
        $macro!([T1]);
        $macro!([T1, T2]);
        $macro!([T1, T2, T3]);
        $macro!([T1, T2, T3, T4]);
        $macro!([T1, T2, T3, T4, T5]);
        $macro!([T1, T2, T3, T4, T5, T6]);
        $macro!([T1, T2, T3, T4, T5, T6, T7]);
        $macro!([T1, T2, T3, T4, T5, T6, T7, T8]);
        $macro!([T1, T2, T3, T4, T5, T6, T7, T8, T9]);
        $macro!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10]);
        $macro!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11]);
        $macro!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12]);
        $macro!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13]);
        $macro!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14]);
        $macro!([
            T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
        ]);
    };
}

all_tuples!(impl_handler);

#[async_trait]
pub trait Service: Send + Sync {
    /// Extractors are not possible to be used in services but you can call [`FromEvent::from_event`] manually.
    ///
    /// [`FromEvent::from_event`]: crate::extract::FromEvent::from_event
    async fn serve(
        &self,
        context: BotContext,
        event: BotEvent,
    ) -> Result<HandlerControl, HandlerError>;

    #[allow(unused_variables)]
    async fn init(&self, bot: BotContext) {}
}

pub(crate) enum HandlerOrService {
    Handler(Box<dyn ErasedHandler>),
    Service(Box<dyn Service>),
}
