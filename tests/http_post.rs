//! HTTP POST (webhook) connection tests: a fake OneBot implementation POSTs
//! events to the SDK's HTTP server.

mod common;

use std::{net::SocketAddr, time::Duration};

use common::*;
use flow_bot::{
    FlowError,
    api::api_ext::ApiExt,
    base::transport::{ConnectionConfig, HttpConfig, HttpPostConfig},
};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha1::Sha1;

fn webhook_cfg(
    bind: SocketAddr,
    secret: Option<String>,
    api: Option<HttpConfig>,
) -> ConnectionConfig {
    ConnectionConfig::HttpPost(HttpPostConfig {
        bind,
        path: "/".to_owned(),
        secret,
        api,
    })
}

fn sign(secret: &str, body: &str) -> String {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let mut hex = String::new();
    for byte in mac.finalize().into_bytes() {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("sha1={hex}")
}

/// POST an event body, waiting until the webhook server is listening.
async fn post_event(bind: SocketAddr, body: &str, signature: Option<String>) -> reqwest::Response {
    let client = reqwest::Client::new();
    let url = format!("http://{bind}/");
    let deadline = tokio::time::Instant::now() + TEST_TIMEOUT;
    loop {
        let mut request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body.to_owned());
        if let Some(signature) = &signature {
            request = request.header("X-Signature", signature);
        }
        match request.send().await {
            Ok(response) => return response,
            Err(e) if e.is_connect() => {
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "webhook server never started listening"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => panic!("POST failed: {e}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn receives_signed_events_and_answers_204() {
    let bind = reserve_addr().await;
    let (bot, events, _run) =
        spawn_recording_bot(webhook_cfg(bind, Some("s3cret".to_owned()), None));

    let body = private_message_event("signed hello").to_string();
    let response = post_event(bind, &body, Some(sign("s3cret", &body))).await;
    assert_eq!(response.status(), 204);

    wait_until(|| !events.lock().unwrap().is_empty()).await;
    assert_eq!(events.lock().unwrap()[0], "signed hello");
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_or_missing_signature_is_rejected() {
    let bind = reserve_addr().await;
    let (bot, events, _run) =
        spawn_recording_bot(webhook_cfg(bind, Some("s3cret".to_owned()), None));

    let body = private_message_event("forged").to_string();
    let response = post_event(bind, &body, Some("sha1=deadbeef".to_owned())).await;
    assert_eq!(response.status(), 403);

    let response = post_event(bind, &body, None).await;
    assert_eq!(response.status(), 403);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        events.lock().unwrap().is_empty(),
        "forged events must not dispatch"
    );
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_calls_use_the_configured_http_endpoint() {
    let (api_addr, _) = spawn_http_api(
        200,
        json!({"status": "ok", "retcode": 0, "data": {"user_id": 10000, "nickname": "bot"}, "echo": null}),
    )
    .await;
    let bind = reserve_addr().await;
    let (bot, _events, _run) = spawn_recording_bot(webhook_cfg(
        bind,
        None,
        Some(HttpConfig {
            base_url: format!("http://{api_addr}"),
            access_token: None,
        }),
    ));

    // Wait for the server, then call an API through the separate endpoint.
    let body = private_message_event("warmup").to_string();
    let response = post_event(bind, &body, None).await;
    assert_eq!(response.status(), 204);

    let info = bot.context().get_login_info().await.unwrap();
    assert_eq!(info.user_id, 10000);
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_api_endpoint_api_calls_fail_fast() {
    let bind = reserve_addr().await;
    let (bot, _events, _run) = spawn_recording_bot(webhook_cfg(bind, None, None));

    let body = private_message_event("warmup").to_string();
    assert_eq!(post_event(bind, &body, None).await.status(), 204);

    let err = tokio::time::timeout(TEST_TIMEOUT, bot.context().get_login_info())
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(err, FlowError::ApiUnavailable), "got {err:?}");
    bot.shutdown();
}
