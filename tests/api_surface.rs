//! Spec-model API surface tests against a fake OneBot implementation:
//! `send_msg`, the generic `call_action` (incl. `_async`), reply shorthand,
//! quick operations, and string-form messages.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use flow_bot::{
    BotContext, BotEvent, FlowBotBuilder, HandlerControl, MessageTarget, QuickOperation,
    api::api_ext::ApiExt,
    base::transport::{ConnectionConfig, ForwardWebSocketConfig, ReconnectionStrategy},
    extract::MessageBody,
};
use serde_json::{Value, json};

fn forward_cfg(url: String) -> ConnectionConfig {
    ConnectionConfig::ForwardWebSocket(ForwardWebSocketConfig {
        url,
        access_token: None,
        reconnection: ReconnectionStrategy::None,
    })
}

/// Spawn a bot with `handler` and run it in the background.
async fn spawn_bot<T, H>(addr: &std::net::SocketAddr, handler: H) -> Arc<flow_bot::FlowBot>
where
    T: Send + Sync + 'static,
    H: flow_bot::Handler<T> + Send + Sync + 'static,
{
    let bot = FlowBotBuilder::new(forward_cfg(format!("ws://{addr}/")))
        .with_state(())
        .with_handler(handler)
        .build();
    let bot = Arc::new(bot);
    let run_bot = bot.clone();
    tokio::spawn(async move {
        let _ = run_bot.run().await;
    });
    bot
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_message_uses_send_msg_with_message_type() {
    let (addr, mut conns) = spawn_ws_server().await;
    let (bot, events, _run) = spawn_recording_bot(forward_cfg(format!("ws://{addr}/")));
    let mut ws = conns.recv().await.unwrap();
    send_event(&mut ws, &private_message_event("primer")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;

    let ctx = bot.context();
    let call = tokio::spawn(async move {
        ctx.send_message(MessageTarget::Group { group_id: 7 }, "hi group")
            .await
    });
    let (action, params, echo) = read_api_request(&mut ws).await;
    assert_eq!(action, "send_msg");
    assert_eq!(
        params,
        json!({
            "message_type": "group",
            "group_id": 7,
            "message": [{"type": "text", "data": {"text": "hi group"}}],
        })
    );
    respond(&mut ws, echo, "ok", 0, json!({"message_id": 42})).await;
    assert_eq!(call.await.unwrap().unwrap().message_id, 42);
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn call_action_supports_async_suffixed_actions() {
    let (addr, mut conns) = spawn_ws_server().await;
    let (bot, events, _run) = spawn_recording_bot(forward_cfg(format!("ws://{addr}/")));
    let mut ws = conns.recv().await.unwrap();
    send_event(&mut ws, &private_message_event("primer")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;

    let ctx = bot.context();
    let call = tokio::spawn(async move {
        ctx.call_action(
            "send_private_msg_rate_limited",
            json!({"user_id": 20000, "message": "queued"}),
        )
        .await
    });
    let (action, params, echo) = read_api_request(&mut ws).await;
    assert_eq!(action, "send_private_msg_rate_limited");
    assert_eq!(params["user_id"], 20000);
    // Suffixed calls answer with status "async" and null data.
    respond(&mut ws, echo, "async", 1, Value::Null).await;
    assert_eq!(call.await.unwrap().unwrap(), Value::Null);
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_message_omits_null_auto_escape() {
    let (addr, mut conns) = spawn_ws_server().await;
    let (bot, events, _run) = spawn_recording_bot(forward_cfg(format!("ws://{addr}/")));
    let mut ws = conns.recv().await.unwrap();
    send_event(&mut ws, &private_message_event("primer")).await;
    wait_until(|| !events.lock().unwrap().is_empty()).await;

    let ctx = bot.context();
    let call = tokio::spawn(async move { ctx.send_group_message(7, "hello", None).await });
    let (action, params, echo) = read_api_request(&mut ws).await;
    assert_eq!(action, "send_group_msg");
    assert!(
        params.get("auto_escape").is_none(),
        "auto_escape must be omitted when None: {params}"
    );
    respond(&mut ws, echo, "ok", 0, json!({"message_id": 1})).await;
    call.await.unwrap().unwrap();
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reply_shorthand_quotes_the_event_message() {
    let (addr, mut conns) = spawn_ws_server().await;
    let mut group_event = private_message_event("to be quoted");
    group_event["message_type"] = json!("group");
    group_event["sub_type"] = json!("normal");
    group_event["group_id"] = json!(7);

    let bot = spawn_bot(&addr, |ctx: BotContext, event: BotEvent| async move {
        ctx.reply(event, "right back").await?;
        Ok(HandlerControl::Continue)
    })
    .await;

    let mut ws = conns.recv().await.unwrap();
    send_event(&mut ws, &group_event).await;

    let (action, params, echo) = read_api_request(&mut ws).await;
    assert_eq!(action, "send_msg");
    assert_eq!(params["message_type"], "group");
    assert_eq!(params["group_id"], 7);
    // The reply segment quotes the incoming message_id and precedes the text.
    assert_eq!(
        params["message"][0],
        json!({"type": "reply", "data": {"id": "15"}})
    );
    assert_eq!(params["message"][1]["data"]["text"], "right back");
    respond(&mut ws, echo, "ok", 0, json!({"message_id": 99})).await;
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quick_operations_over_websocket_use_the_hidden_action() {
    let (addr, mut conns) = spawn_ws_server().await;
    let bot = spawn_bot(&addr, |ctx: BotContext, event: BotEvent| async move {
        ctx.handle_quick_operation(
            event.clone(),
            QuickOperation {
                reply: Some("watch your language".into()),
                ban: Some(true),
                ban_duration: Some(60),
                ..Default::default()
            },
        )
        .await?;
        Ok(HandlerControl::Continue)
    })
    .await;

    let mut ws = conns.recv().await.unwrap();
    send_event(&mut ws, &private_message_event("rude words")).await;

    let (action, params, echo) = read_api_request(&mut ws).await;
    assert_eq!(action, ".handle_quick_operation");
    // The context round-trips the event; the operation carries the handler's
    // quick-op fields.
    assert_eq!(params["context"]["post_type"], "message");
    assert_eq!(params["context"]["raw_message"], "rude words");
    assert_eq!(
        params["operation"]["reply"],
        json!([{"type": "text", "data": {"text": "watch your language"}}])
    );
    assert_eq!(params["operation"]["ban"], true);
    assert_eq!(params["operation"]["ban_duration"], 60);
    respond(&mut ws, echo, "ok", 0, Value::Null).await;
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn string_form_messages_dispatch_as_segments() {
    let (addr, mut conns) = spawn_ws_server().await;
    let bodies: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen = bodies.clone();
    let bot = spawn_bot(&addr, move |body: MessageBody| {
        let seen = seen.clone();
        async move {
            seen.lock().unwrap().push(body.0.to_string());
            Ok(HandlerControl::Continue)
        }
    })
    .await;

    let mut ws = conns.recv().await.unwrap();
    // An implementation configured with `event.message_format: string` sends
    // the CQ-code form and omits `font`.
    send_event(
        &mut ws,
        &json!({
            "time": 123,
            "self_id": 10000,
            "post_type": "message",
            "message_type": "private",
            "sub_type": "friend",
            "message_id": 15,
            "user_id": 20000,
            "message": "[CQ:at,qq=all] hi &#91;flow&#93;",
            "raw_message": "[CQ:at,qq=all] hi [flow]",
            "sender": {"nickname": "tester"},
        }),
    )
    .await;

    wait_until(|| !bodies.lock().unwrap().is_empty()).await;
    assert_eq!(
        bodies.lock().unwrap()[0],
        "[CQ:at,qq=all] hi &#91;flow&#93;"
    );
    bot.shutdown();
}
