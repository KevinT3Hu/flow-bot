//! HTTP connection tests against a fake OneBot implementation's HTTP API.

mod common;

use std::time::Duration;

use common::*;
use flow_bot::{
    FlowError,
    api::api_ext::ApiExt,
    base::transport::{ConnectionConfig, HttpConfig},
};
use serde_json::json;

fn http_cfg(base_url: String, access_token: Option<String>) -> ConnectionConfig {
    ConnectionConfig::Http(HttpConfig {
        base_url,
        access_token,
    })
}

/// Poll an API call while the transport is still being installed (HTTP bots
/// set it as soon as `run` starts), then return the first real outcome.
async fn first_api_outcome(bot: &flow_bot::FlowBot) -> Result<flow_bot::api::LoginInfo, FlowError> {
    let deadline = tokio::time::Instant::now() + TEST_TIMEOUT;
    loop {
        match bot.context().get_login_info().await {
            Err(FlowError::NoConnection) => {
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "transport never became ready"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            outcome => return outcome,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn calls_apis_over_http_with_bearer_auth() {
    let (addr, auth_seen) = spawn_http_api(
        200,
        json!({"status": "ok", "retcode": 0, "data": {"user_id": 10000, "nickname": "bot"}, "echo": null}),
    )
    .await;
    let (bot, _events, _run) = spawn_recording_bot(http_cfg(
        format!("http://{addr}"),
        Some("kSLuTF2GC2Q4q4ugm3".to_owned()),
    ));

    let info = first_api_outcome(&bot).await.unwrap();
    assert_eq!(info.user_id, 10000);
    assert_eq!(info.nickname, "bot");

    // The access token was sent as a bearer token.
    let seen = auth_seen.lock().unwrap();
    assert_eq!(
        seen.last().and_then(|h| h.as_deref()),
        Some("Bearer kSLuTF2GC2Q4q4ugm3")
    );
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_404_maps_to_api_error_1404() {
    let (addr, _) = spawn_http_api(404, json!({})).await;
    let (bot, _, _) = spawn_recording_bot(http_cfg(format!("http://{addr}"), None));
    match first_api_outcome(&bot).await.unwrap_err() {
        FlowError::ApiError { retcode, .. } => assert_eq!(retcode, 1404),
        other => panic!("expected ApiError 1404, got {other:?}"),
    }
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_401_maps_to_invalid_auth() {
    let (addr, _) = spawn_http_api(401, json!({})).await;
    let (bot, _, _) = spawn_recording_bot(http_cfg(format!("http://{addr}"), None));
    let err = first_api_outcome(&bot).await.unwrap_err();
    assert!(matches!(err, FlowError::InvalidAuth), "got {err:?}");
    bot.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_retcode_in_envelope_surfaces_as_api_error() {
    let (addr, _) = spawn_http_api(
        200,
        json!({"status": "failed", "retcode": 100, "data": null, "wording": "account frozen"}),
    )
    .await;
    let (bot, _, _) = spawn_recording_bot(http_cfg(format!("http://{addr}"), None));
    match first_api_outcome(&bot).await.unwrap_err() {
        FlowError::ApiError {
            retcode, message, ..
        } => {
            assert_eq!(retcode, 100);
            assert_eq!(message.as_deref(), Some("account frozen"));
        }
        other => panic!("expected ApiError, got {other:?}"),
    }
    bot.shutdown();
}
