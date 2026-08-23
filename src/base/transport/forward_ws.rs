//! Forward WebSocket (正向): flow-bot dials the OneBot implementation's
//! WebSocket server, with reconnection.

use std::sync::Arc;

use crate::event::BotEvent;
use futures::{StreamExt, stream::SplitSink, stream::SplitStream};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use crate::{
    base::{
        bot::FlowBot,
        transport::{
            ApiTransport, ForwardWebSocketConfig, ReconnectionStrategy, validate_ws_url,
            ws::WsSession,
        },
    },
    error::FlowError,
};

pub(crate) async fn run(
    bot: &FlowBot,
    cfg: &ForwardWebSocketConfig,
    events: mpsc::Sender<BotEvent>,
) -> Result<(), FlowError> {
    let url = validate_ws_url(&cfg.url).map_err(FlowError::InvalidConfig)?;

    match cfg.reconnection.clone() {
        ReconnectionStrategy::None => cycle(bot, cfg, &url, &events).await,
        ReconnectionStrategy::Infinite {
            initial_delay_ms,
            max_delay_ms,
        } => {
            run_with_reconnect(
                bot,
                cfg,
                &url,
                &events,
                None,
                initial_delay_ms,
                max_delay_ms,
            )
            .await
        }
        ReconnectionStrategy::Limited {
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
        } => {
            run_with_reconnect(
                bot,
                cfg,
                &url,
                &events,
                Some(max_attempts),
                initial_delay_ms,
                max_delay_ms,
            )
            .await
        }
    }
}

/// One full connection lifecycle: connect, serve until the socket closes or
/// shutdown is requested, then hand the outcome back to the caller.
async fn cycle(
    bot: &FlowBot,
    cfg: &ForwardWebSocketConfig,
    url: &reqwest::Url,
    events: &mpsc::Sender<BotEvent>,
) -> Result<(), FlowError> {
    let (write, read) = connect(url, cfg.access_token.as_deref()).await?;

    let session = WsSession::spawn(write);
    let as_transport: Arc<dyn ApiTransport> = session.clone();
    bot.context().set_transport(as_transport.clone());
    bot.init_services_once().await;

    let outcome = read_loop(bot, &session, read, events).await;

    bot.context().clear_transport_if(&as_transport);
    session.fail_pending();
    outcome
}

/// Reconnection loop shared by the `Infinite` and `Limited` strategies.
/// `max_attempts == Some(n)` limits the number of (re)connection cycles.
async fn run_with_reconnect(
    bot: &FlowBot,
    cfg: &ForwardWebSocketConfig,
    url: &reqwest::Url,
    events: &mpsc::Sender<BotEvent>,
    max_attempts: Option<u32>,
    initial_delay_ms: u64,
    max_delay_ms: u64,
) -> Result<(), FlowError> {
    let mut attempt: u32 = 0;
    loop {
        if bot.shutdown_requested() {
            return Ok(());
        }

        if let Some(max_attempts) = max_attempts
            && attempt >= max_attempts
        {
            return Err(FlowError::ReconnectionFailed(max_attempts));
        }

        let delay_ms =
            (initial_delay_ms.saturating_mul(2_u64.saturating_pow(attempt))).min(max_delay_ms);

        match cycle(bot, cfg, url, events).await {
            Ok(()) => {
                if bot.shutdown_requested() {
                    return Ok(());
                }
                tracing::info!("connection closed; reconnecting in {delay_ms}ms");
            }
            Err(e) => {
                if bot.shutdown_requested() {
                    return Ok(());
                }
                tracing::warn!("connection error: {e}; reconnecting in {delay_ms}ms");
            }
        }

        attempt += 1;

        if bot.shutdown_requested() {
            return Ok(());
        }
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)) => {}
            _ = bot.wait_shutdown() => return Ok(()),
        }
    }
}

async fn connect(
    url: &reqwest::Url,
    access_token: Option<&str>,
) -> Result<
    (
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ),
    FlowError,
> {
    let mut request = url.as_str().into_client_request()?;
    if let Some(token) = access_token {
        let value = format!("Bearer {token}")
            .parse()
            .map_err(|_| FlowError::InvalidAuth)?;
        request.headers_mut().insert("Authorization", value);
    }
    let (stream, _) = connect_async(request).await?;
    Ok(stream.split())
}

async fn read_loop(
    bot: &FlowBot,
    session: &WsSession,
    mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    events: &mpsc::Sender<BotEvent>,
) -> Result<(), FlowError> {
    loop {
        tokio::select! {
            msg = read.next() => match msg {
                Some(Ok(Message::Text(text))) => session.handle_frame(&text, events).await,
                // Ping/Pong/Close are handled by tungstenite itself.
                Some(Ok(_)) => {}
                Some(Err(e)) => return Err(e.into()),
                None => return Ok(()),
            },
            _ = bot.wait_shutdown() => return Ok(()),
        }
    }
}
