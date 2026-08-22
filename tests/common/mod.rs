//! Shared harness: fake OneBot implementations and bot scaffolding.

#![allow(dead_code)]

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::mpsc,
    task::JoinHandle,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, accept_async, connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use flow_bot::{ConnectionConfig, FlowBot, FlowBotBuilder, HandlerControl};

pub const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// A connection accepted by the fake implementation's WS server.
pub type ServerWs = WebSocketStream<TcpStream>;
/// A client-side connection (fake implementation dialing the SDK's server).
pub type ClientWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub type Ws = ServerWs;

/// Reserve an ephemeral port for a server the bot itself will bind (the
/// listener is dropped immediately; tokio sets `SO_REUSEADDR`).
pub async fn reserve_addr() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap()
}

/// A fake OneBot implementation WebSocket *server* (for forward-WS tests).
/// Every accepted connection is handed to the test through the channel.
pub async fn spawn_ws_server() -> (SocketAddr, mpsc::Receiver<ServerWs>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel(4);
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let Ok(ws) = accept_async(stream).await else {
                continue;
            };
            if tx.send(ws).await.is_err() {
                break;
            }
        }
    });
    (addr, rx)
}

/// A fake OneBot implementation *client* (for reverse-WS tests): dials the
/// SDK's WebSocket server with the spec's handshake headers.
pub async fn impl_dial(
    addr: SocketAddr,
    role: &str,
    token: Option<&str>,
) -> Result<ClientWs, tokio_tungstenite::tungstenite::Error> {
    let mut request = format!("ws://{addr}/ws").into_client_request()?;
    let headers = request.headers_mut();
    headers.insert("X-Self-ID", "10000".parse().unwrap());
    headers.insert("X-Client-Role", role.parse().unwrap());
    if let Some(token) = token {
        headers.insert("Authorization", format!("Bearer {token}").parse().unwrap());
    }
    Ok(connect_async(request).await?.0)
}

/// A fake OneBot HTTP API server answering every `/{action}` with a canned
/// status and JSON body, recording the `Authorization` headers it saw.
pub async fn spawn_http_api(
    status: u16,
    body: Value,
) -> (SocketAddr, Arc<Mutex<Vec<Option<String>>>>) {
    let auth_seen: Arc<Mutex<Vec<Option<String>>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = auth_seen.clone();
    let app = axum::Router::new().fallback(move |headers: axum::http::HeaderMap| {
        let body = body.clone();
        let recorder = recorder.clone();
        async move {
            recorder.lock().unwrap().push(
                headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned),
            );
            (
                axum::http::StatusCode::from_u16(status).unwrap(),
                axum::Json(body),
            )
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (addr, auth_seen)
}

/// Read the next text frame from a WebSocket.
pub async fn next_text<S>(ws: &mut WebSocketStream<S>) -> String
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    while let Some(msg) = ws.next().await {
        if let Message::Text(text) = msg.unwrap() {
            return text.to_string();
        }
    }
    panic!("WebSocket closed before a text frame arrived");
}

/// Read one `{action, params, echo}` API request sent by the SDK.
pub async fn read_api_request<S>(ws: &mut WebSocketStream<S>) -> (String, Value, Value)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let value: Value = serde_json::from_str(&next_text(ws).await).unwrap();
    (
        value["action"].as_str().unwrap().to_owned(),
        value["params"].clone(),
        value["echo"].clone(),
    )
}

/// Answer an API request by echoing its `echo` back, per the spec.
pub async fn respond<S>(
    ws: &mut WebSocketStream<S>,
    echo: Value,
    status: &str,
    retcode: i32,
    data: Value,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    respond_raw(
        ws,
        json!({"status": status, "retcode": retcode, "data": data, "echo": echo}),
    )
    .await;
}

/// Send an arbitrary JSON object as a text frame.
pub async fn respond_raw<S>(ws: &mut WebSocketStream<S>, value: Value)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    ws.send(Message::Text(value.to_string().into()))
        .await
        .unwrap();
}

/// Push an event to the SDK over the WebSocket.
pub async fn send_event<S>(ws: &mut WebSocketStream<S>, event: &Value)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    respond_raw(ws, event.clone()).await;
}

/// A well-formed private message event.
pub fn private_message_event(text: &str) -> Value {
    json!({
        "time": 123,
        "self_id": 10000,
        "post_type": "message",
        "message_type": "private",
        "sub_type": "friend",
        "message_id": 15,
        "user_id": 20000,
        "message": [{"type": "text", "data": {"text": text}}],
        "raw_message": text,
        "font": 0,
        "sender": {"nickname": "tester"},
    })
}

/// Poll `pred` every 10ms, panicking if it does not hold within `TEST_TIMEOUT`.
pub async fn wait_until<F: Fn() -> bool>(pred: F) {
    let deadline = tokio::time::Instant::now() + TEST_TIMEOUT;
    while !pred() {
        if tokio::time::Instant::now() > deadline {
            panic!("condition not met within {TEST_TIMEOUT:?}");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A bot recording message events, running in the background.
pub type RecordingBot = (
    Arc<FlowBot>,
    Arc<Mutex<Vec<String>>>,
    JoinHandle<Result<(), flow_bot::FlowError>>,
);

/// Build a bot whose handler records `raw_message` of every message event,
/// and run it in the background. The returned task resolves to `run()`'s
/// outcome, so tests can assert graceful shutdown returns `Ok(())`.
pub fn spawn_recording_bot(cfg: ConnectionConfig) -> RecordingBot {
    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let handler_events = events.clone();
    let bot = FlowBotBuilder::new(cfg)
        .with_state(())
        .with_handler(move |msg: flow_bot::extract::Message| {
            let events = handler_events.clone();
            async move {
                events.lock().unwrap().push(msg.raw_message.clone());
                Ok(HandlerControl::Continue)
            }
        })
        .build();
    let bot = Arc::new(bot);
    let run_bot = bot.clone();
    let handle = tokio::spawn(async move { run_bot.run().await });
    (bot, events, handle)
}
