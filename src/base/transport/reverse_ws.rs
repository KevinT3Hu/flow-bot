//! Reverse WebSocket (反向): flow-bot runs a WebSocket server and the OneBot
//! implementation connects to it. The implementation is responsible for
//! reconnecting; this server keeps accepting connections until shutdown.

use std::sync::Arc;

use crate::event::BotEvent;
use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::{
    base::{
        bot::FlowBot,
        context::BotContext,
        middleware::EventProcessor,
        transport::{ApiTransport, ReverseWebSocketConfig, ws::WsSession},
    },
    error::FlowError,
};

#[derive(Clone)]
struct WsState {
    context: BotContext,
    access_token: Option<String>,
    path: Option<String>,
    events: mpsc::Sender<BotEvent>,
    processors: Arc<Vec<Arc<dyn EventProcessor>>>,
    shared: Arc<crate::base::bot::BotShared>,
}

/// The `X-Client-Role` handshake header values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClientRole {
    Api,
    Event,
    Universal,
}

pub(crate) async fn run(
    bot: &FlowBot,
    cfg: &ReverseWebSocketConfig,
    events: mpsc::Sender<BotEvent>,
) -> Result<(), FlowError> {
    let listener = tokio::net::TcpListener::bind(cfg.bind)
        .await
        .map_err(FlowError::ServerError)?;
    tracing::info!("reverse WebSocket server listening on {}", cfg.bind);

    let state = WsState {
        context: bot.context().clone(),
        access_token: cfg.access_token.clone(),
        path: cfg.path.clone(),
        events,
        processors: bot.processors.clone(),
        shared: bot.shared.clone(),
    };
    let app = Router::new().fallback(ws_route).with_state(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(bot.shutdown_signal())
        .await
        .map_err(FlowError::ServerError)?;
    Ok(())
}

async fn ws_route(
    State(state): State<WsState>,
    uri: Uri,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Some(expected) = &state.path
        && uri.path() != expected
    {
        return StatusCode::NOT_FOUND.into_response();
    }

    if let Some(token) = &state.access_token {
        let expected = format!("Bearer {token}");
        let authorized = headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value == expected);
        if !authorized {
            tracing::warn!("rejected reverse WebSocket client: invalid access token");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let role = match headers
        .get("x-client-role")
        .and_then(|value| value.to_str().ok())
    {
        Some("API") => ClientRole::Api,
        Some("Event") => ClientRole::Event,
        // "Universal" and a missing header both speak both directions.
        _ => ClientRole::Universal,
    };

    if let Some(self_id) = headers
        .get("x-self-id")
        .and_then(|value| value.to_str().ok())
    {
        tracing::info!(self_id, ?role, "reverse WebSocket client connected");
    } else {
        tracing::info!(?role, "reverse WebSocket client connected");
    }

    ws.on_upgrade(move |socket| handle_connection(state, socket, role))
}

async fn handle_connection(state: WsState, socket: WebSocket, role: ClientRole) {
    let (sink, mut stream) = socket.split();
    let session = WsSession::spawn(sink);
    let as_transport: Arc<dyn ApiTransport> = session.clone();

    let carries_api = role != ClientRole::Event;
    if carries_api {
        state.context.set_transport(as_transport.clone());
    }
    state
        .shared
        .init_services_once(&state.processors, state.context.clone())
        .await;

    loop {
        tokio::select! {
            msg = stream.next() => match msg {
                Some(Ok(Message::Text(text))) => session.handle_frame(&text, &state.events).await,
                // Ping/Pong are answered automatically by the WS layer.
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    tracing::warn!("reverse WebSocket connection error: {e}");
                    break;
                }
                None => break,
            },
            _ = state.shared.wait_shutdown() => break,
        }
    }

    tracing::info!(?role, "reverse WebSocket client disconnected");
    if carries_api {
        state.context.clear_transport_if(&as_transport);
    }
    session.fail_pending();
}
