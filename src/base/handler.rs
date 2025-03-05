use crate::{base::extract::FromEvent, event::BotEvent};
use async_trait::async_trait;
use std::{convert::Infallible, future::Future, ops::FromResidual};

use super::context::BotContext;

pub enum HandlerControl {
    Skip,
    Continue,
    Block,
}

impl<E> FromResidual<Result<Infallible, E>> for HandlerControl {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Err(_) => HandlerControl::Skip,
        }
    }
}

impl FromResidual<Option<Infallible>> for HandlerControl {
    fn from_residual(residual: Option<Infallible>) -> Self {
        match residual {
            None => HandlerControl::Skip,
        }
    }
}

#[async_trait]
pub trait Handler<T> {
    async fn handle(&self, context: BotContext, event: BotEvent) -> HandlerControl;
}

macro_rules! impl_handler {
    ([$($ty:ident),*]) => {
        #[allow(unused_variables, unused_mut,unused_parens,non_snake_case)]
        #[async_trait]
        impl<F,Fut, $($ty),*> Handler<($($ty),*)> for F
        where
            F: Fn($($ty),*) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = HandlerControl> + Send + 'static,
            $($ty: FromEvent+Send),*
        {
            async fn handle(&self, context: BotContext, event: BotEvent) -> HandlerControl {
                match ($($ty::from_event(context.clone(), event.clone()).await,)*) {
                    ($(Some($ty),)*) => self($($ty),*).await,
                    _ => HandlerControl::Skip,
                }
            }
        }
    };
}

#[async_trait]
pub(crate) trait ErasedHandler: Send + Sync {
    async fn call(&self, context: BotContext, event: BotEvent) -> HandlerControl;
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
    async fn call(&self, context: BotContext, event: BotEvent) -> HandlerControl {
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
