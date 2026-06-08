use std::{future::Future, sync::Arc};

use async_trait::async_trait;

use crate::{
    base::{
        context::BotContext,
        control::{HandlerControl, HandlerError},
        handler::HandlerOrService,
    },
    event::BotEvent,
};

/// A middleware that can intercept and transform events before they reach handlers.
///
/// Middlewares are composed in an onion pattern: the outermost middleware runs first,
/// then calls [`Next::run`] to proceed to the next middleware or handler.
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn handle(
        &self,
        ctx: BotContext,
        event: BotEvent,
        next: Next,
    ) -> Result<HandlerControl, HandlerError>;
}

/// A handle to the rest of the middleware chain.
///
/// Cloning `Next` is cheap (it just clones an `Arc`).
#[derive(Clone)]
pub struct Next {
    pub(crate) inner: Arc<dyn EventProcessor>,
}

impl Next {
    /// Proceed to the next middleware or handler in the chain.
    pub async fn run(
        &self,
        ctx: BotContext,
        event: BotEvent,
    ) -> Result<HandlerControl, HandlerError> {
        self.inner.process(ctx, event).await
    }
}

/// Internal trait for a node in the middleware/handler chain.
#[async_trait]
pub(crate) trait EventProcessor: Send + Sync {
    async fn process(
        &self,
        ctx: BotContext,
        event: BotEvent,
    ) -> Result<HandlerControl, HandlerError>;

    #[allow(unused_variables)]
    async fn init(&self, ctx: BotContext) {}
}

/// A middleware node that wraps the rest of the chain.
pub(crate) struct Node {
    pub(crate) middleware: Arc<dyn Middleware>,
    pub(crate) inner: Arc<dyn EventProcessor>,
}

#[async_trait]
impl EventProcessor for Node {
    async fn process(
        &self,
        ctx: BotContext,
        event: BotEvent,
    ) -> Result<HandlerControl, HandlerError> {
        let next = Next {
            inner: self.inner.clone(),
        };
        self.middleware.handle(ctx, event, next).await
    }

    async fn init(&self, ctx: BotContext) {
        self.inner.init(ctx).await;
    }
}

/// A leaf node that directly invokes a handler or service.
pub(crate) struct Leaf {
    pub(crate) inner: HandlerOrService,
}

#[async_trait]
impl EventProcessor for Leaf {
    async fn process(
        &self,
        ctx: BotContext,
        event: BotEvent,
    ) -> Result<HandlerControl, HandlerError> {
        match &self.inner {
            HandlerOrService::Handler(h) => h.call(ctx, event).await,
            HandlerOrService::Service(s) => s.serve(ctx, event).await,
        }
    }

    async fn init(&self, ctx: BotContext) {
        if let HandlerOrService::Service(s) = &self.inner {
            s.init(ctx).await;
        }
    }
}

/// Create a [`Middleware`] from a function.
///
/// # Example
///
/// ```rust,ignore
/// FlowBotBuilder::new(...)
///     .with_handler(help_cmd)
///     .layer(from_fn(|ctx, event, next| async move {
///         println!("before");
///         let res = next.run(ctx, event).await;
///         println!("after");
///         res
///     }))
///     .build();
/// ```
pub fn from_fn<F, Fut>(f: F) -> FromFn<F>
where
    F: Fn(BotContext, BotEvent, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<HandlerControl, HandlerError>> + Send + 'static,
{
    FromFn { f }
}

/// A middleware created by [`from_fn`].
pub struct FromFn<F> {
    f: F,
}

#[async_trait]
impl<F, Fut> Middleware for FromFn<F>
where
    F: Fn(BotContext, BotEvent, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<HandlerControl, HandlerError>> + Send + 'static,
{
    async fn handle(
        &self,
        ctx: BotContext,
        event: BotEvent,
        next: Next,
    ) -> Result<HandlerControl, HandlerError> {
        (self.f)(ctx, event, next).await
    }
}
