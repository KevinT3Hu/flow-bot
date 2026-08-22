//! Reverse WebSocket connection tests with a fake OneBot implementation
//! dialing the SDK's server.

mod common;

use std::{net::SocketAddr, time::Duration};

use common::*;
use flow_bot::{
    FlowError,
    api::api_ext::ApiExt,
    base::transport::{ConnectionConfig, ReverseWebSocketConfig},
};
use serde_json::json;

fn reverse_cfg(bind: SocketAddr, access_token: Option<String>) -> ConnectionConfig {
    ConnectionConfig::ReverseWebSocket(ReverseWebSocketConfig {
        bind,
        path: None,
        access_token,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn universal_client_carries_events_and_api_calls() {
    let addr = reserve_addr().await;
    let (bot, events, run) = spawn_recording_bot(reverse_cfg(addr, None));

    let mut ws = impl_dial(addr, "Universal", None).await.unwrap();
    send_event(&mut ws, &private_message_event("reverse hello")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;
    assert_eq!(events.lock().unwrap()[0], "reverse hello");

    // API calls from the SDK travel over the implementation's connection.
    let ctx = bot.context();
    let call = tokio::spawn(async move { ctx.get_login_info().await });
    let (action, _, echo) = read_api_request(&mut ws).await;
    assert_eq!(action, "get_login_info");
    respond(
        &mut ws,
        echo,
        "ok",
        0,
        json!({"user_id": 10000, "nickname": "bot"}),
    )
    .await;
    assert_eq!(call.await.unwrap().unwrap().user_id, 10000);

    bot.shutdown();
    let result = tokio::time::timeout(TEST_TIMEOUT, run)
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.is_ok(),
        "run() should return Ok on shutdown: {result:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_client_receives_events_but_no_api_calls() {
    let addr = reserve_addr().await;
    let (bot, events, _run) = spawn_recording_bot(reverse_cfg(addr, None));

    let mut ws = impl_dial(addr, "Event", None).await.unwrap();
    send_event(&mut ws, &private_message_event("event only")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;

    // No API-capable client connected: API calls fail instead of hanging.
    let err = tokio::time::timeout(TEST_TIMEOUT, bot.context().get_login_info())
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(err, FlowError::NoConnection));
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrong_access_token_is_rejected() {
    let addr = reserve_addr().await;
    let (bot, events, _run) = spawn_recording_bot(reverse_cfg(addr, Some("secret".into())));

    let result = impl_dial(addr, "Universal", Some("wrong")).await;
    assert!(result.is_err(), "handshake with a bad token must fail");

    // With the right token the connection is accepted.
    let mut ws = impl_dial(addr, "Universal", Some("secret")).await.unwrap();
    send_event(&mut ws, &private_message_event("authorized")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;
    assert_eq!(events.lock().unwrap()[0], "authorized");
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_connection_takeover_after_disconnect() {
    let addr = reserve_addr().await;
    let (bot, _events, _run) = spawn_recording_bot(reverse_cfg(addr, None));

    // First API client connects, then disappears.
    let ws1 = impl_dial(addr, "API", None).await.unwrap();
    drop(ws1);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // A second client takes over and API calls work again.
    let mut ws2 = impl_dial(addr, "API", None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let ctx = bot.context();
    let call = tokio::spawn(async move { ctx.get_login_info().await });
    let (action, _, echo) = read_api_request(&mut ws2).await;
    assert_eq!(action, "get_login_info");
    respond(
        &mut ws2,
        echo,
        "ok",
        0,
        json!({"user_id": 10000, "nickname": "bot"}),
    )
    .await;
    assert_eq!(call.await.unwrap().unwrap().user_id, 10000);
    bot.shutdown();
}
