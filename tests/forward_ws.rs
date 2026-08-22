//! Forward WebSocket connection tests against a fake OneBot implementation.

mod common;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use common::*;
use flow_bot::{
    FlowError,
    api::api_ext::ApiExt,
    base::transport::{ConnectionConfig, ForwardWebSocketConfig, ReconnectionStrategy},
};
use serde_json::json;

fn forward_cfg(url: String, reconnection: ReconnectionStrategy) -> ConnectionConfig {
    ConnectionConfig::ForwardWebSocket(ForwardWebSocketConfig {
        url,
        access_token: None,
        reconnection,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delivers_events_and_calls_apis() {
    let (addr, mut conns) = spawn_ws_server().await;
    let (bot, events, run) = spawn_recording_bot(forward_cfg(
        format!("ws://{addr}/"),
        ReconnectionStrategy::None,
    ));

    let mut ws = conns.recv().await.unwrap();
    send_event(&mut ws, &private_message_event("hello")).await;
    wait_until(|| events.lock().unwrap().len() == 1).await;
    assert_eq!(events.lock().unwrap()[0], "hello");

    // API calls travel over the same socket and are matched by `echo`.
    let ctx = bot.context();
    let call = tokio::spawn(async move { ctx.get_login_info().await });
    let (action, params, echo) = read_api_request(&mut ws).await;
    assert_eq!(action, "get_login_info");
    assert_eq!(params, json!({}));
    respond(
        &mut ws,
        echo,
        "ok",
        0,
        json!({"user_id": 10000, "nickname": "bot"}),
    )
    .await;
    let info = call.await.unwrap().unwrap();
    assert_eq!(info.user_id, 10000);
    assert_eq!(info.nickname, "bot");

    // Graceful shutdown makes `run()` return Ok.
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
async fn api_error_carries_retcode_and_wording() {
    let (addr, mut conns) = spawn_ws_server().await;
    let (bot, events, _run) = spawn_recording_bot(forward_cfg(
        format!("ws://{addr}/"),
        ReconnectionStrategy::None,
    ));

    let mut ws = conns.recv().await.unwrap();
    send_event(&mut ws, &private_message_event("hi")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;

    let ctx = bot.context();
    let call = tokio::spawn(async move { ctx.get_login_info().await });
    let (_, _, echo) = read_api_request(&mut ws).await;
    respond_raw(
        &mut ws,
        json!({"status": "failed", "retcode": 1200, "data": null, "echo": echo, "wording": "nope"}),
    )
    .await;
    match call.await.unwrap().unwrap_err() {
        FlowError::ApiError {
            retcode, message, ..
        } => {
            assert_eq!(retcode, 1200);
            assert_eq!(message.as_deref(), Some("nope"));
        }
        other => panic!("expected ApiError, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_frame_does_not_tear_down_the_connection() {
    let (addr, mut conns) = spawn_ws_server().await;
    let (bot, events, _run) = spawn_recording_bot(forward_cfg(
        format!("ws://{addr}/"),
        ReconnectionStrategy::None,
    ));

    let mut ws = conns.recv().await.unwrap();
    respond_raw(&mut ws, json!("this is not an event")).await;
    respond_raw(
        &mut ws,
        serde_json::Value::String("{{{ definitely not json".into()),
    )
    .await;

    // The connection survived both malformed frames and still delivers events.
    send_event(&mut ws, &private_message_event("still alive")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;
    assert_eq!(events.lock().unwrap()[0], "still alive");
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn events_with_an_echo_field_are_not_swallowed_as_responses() {
    let (addr, mut conns) = spawn_ws_server().await;
    let (bot, events, _run) = spawn_recording_bot(forward_cfg(
        format!("ws://{addr}/"),
        ReconnectionStrategy::None,
    ));

    let mut ws = conns.recv().await.unwrap();
    // An event carrying a string `echo` field must still be dispatched as an
    // event (only frames whose echo matches a pending request are responses).
    let mut event = private_message_event("echoed event");
    event["echo"] = json!("not-a-pending-request");
    send_event(&mut ws, &event).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;
    assert_eq!(events.lock().unwrap()[0], "echoed event");
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_fails_pending_api_calls_immediately() {
    let (addr, mut conns) = spawn_ws_server().await;
    // Long API timeout so the test proves the failure comes from the
    // disconnect, not from a timeout.
    let bot = flow_bot::FlowBotBuilder::new(forward_cfg(
        format!("ws://{addr}/"),
        ReconnectionStrategy::None,
    ))
    .with_state(())
    .with_handler(|_msg: flow_bot::extract::Message| async {
        Ok(flow_bot::HandlerControl::Continue)
    })
    .api_timeout(Duration::from_secs(30))
    .build();
    let bot = Arc::new(bot);
    let run = {
        let bot = bot.clone();
        tokio::spawn(async move {
            let _ = bot.run().await;
        })
    };

    let mut ws = conns.recv().await.unwrap();
    // Prime the connection so the transport is installed for sure.
    send_event(&mut ws, &private_message_event("primer")).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Start an API call and never answer it, then kill the connection.
    let ctx = bot.context();
    let call = tokio::spawn(async move { ctx.get_login_info().await });
    let (_, _, _echo) = read_api_request(&mut ws).await;
    drop(ws);

    let started = Instant::now();
    let err = call.await.unwrap().unwrap_err();
    assert!(
        matches!(err, FlowError::NoConnection),
        "expected NoConnection, got {err:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "pending call should fail immediately on disconnect, took {:?}",
        started.elapsed()
    );
    let _ = tokio::time::timeout(TEST_TIMEOUT, run).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnects_after_disconnect() {
    let (addr, mut conns) = spawn_ws_server().await;
    let (bot, events, _run) = spawn_recording_bot(forward_cfg(
        format!("ws://{addr}/"),
        ReconnectionStrategy::Infinite {
            initial_delay_ms: 10,
            max_delay_ms: 50,
        },
    ));

    // First connection comes and goes.
    let ws1 = conns.recv().await.unwrap();
    drop(ws1);

    // The bot reconnects on its own.
    let mut ws2 = tokio::time::timeout(TEST_TIMEOUT, conns.recv())
        .await
        .expect("bot should reconnect")
        .unwrap();
    send_event(&mut ws2, &private_message_event("after reconnect")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;
    assert_eq!(events.lock().unwrap()[0], "after reconnect");
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_init_runs_only_once_across_reconnects() {
    use std::sync::atomic::{AtomicU32, Ordering};

    let (addr, mut conns) = spawn_ws_server().await;
    let inits = Arc::new(AtomicU32::new(0));

    struct CountingService {
        inits: Arc<AtomicU32>,
    }

    #[async_trait::async_trait]
    impl flow_bot::Service for CountingService {
        async fn serve(
            &self,
            _ctx: flow_bot::BotContext,
            _event: flow_bot::BotEvent,
        ) -> Result<flow_bot::HandlerControl, flow_bot::HandlerError> {
            Ok(flow_bot::HandlerControl::Continue)
        }

        async fn init(&self, _ctx: flow_bot::BotContext) {
            self.inits.fetch_add(1, Ordering::SeqCst);
        }
    }

    let bot = flow_bot::FlowBotBuilder::new(forward_cfg(
        format!("ws://{addr}/"),
        ReconnectionStrategy::Infinite {
            initial_delay_ms: 10,
            max_delay_ms: 50,
        },
    ))
    .with_state(())
    .with_service(CountingService {
        inits: inits.clone(),
    })
    .build();
    let bot = Arc::new(bot);
    let run = {
        let bot = bot.clone();
        tokio::spawn(async move {
            let _ = bot.run().await;
        })
    };

    let ws1 = conns.recv().await.unwrap();
    drop(ws1);
    let mut ws2 = tokio::time::timeout(TEST_TIMEOUT, conns.recv())
        .await
        .expect("bot should reconnect")
        .unwrap();
    send_event(&mut ws2, &private_message_event("ping")).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(inits.load(Ordering::SeqCst), 1);
    bot.shutdown();
    let _ = tokio::time::timeout(TEST_TIMEOUT, run).await;
}
