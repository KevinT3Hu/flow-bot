//! Builder validation tests (config-time panics) and config validation.

use flow_bot::{
    FlowBotBuilder,
    base::transport::{
        ConnectionConfig, ForwardWebSocketConfig, HttpConfig, HttpPostConfig, ReconnectionStrategy,
    },
};

fn forward(url: &str) -> ConnectionConfig {
    ConnectionConfig::ForwardWebSocket(ForwardWebSocketConfig {
        url: url.to_owned(),
        access_token: None,
        reconnection: ReconnectionStrategy::None,
    })
}

#[test]
#[should_panic(expected = "concurrent_limit must be greater than 0")]
fn zero_concurrent_limit_panics_at_build_time() {
    FlowBotBuilder::new(forward("ws://127.0.0.1:1"))
        .concurrent_limit(0)
        .with_state(())
        .build();
}

#[test]
#[should_panic(expected = "invalid connection configuration")]
fn bad_url_scheme_panics_at_build_time() {
    FlowBotBuilder::new(forward("ftp://example.com"))
        .with_state(())
        .build();
}

#[test]
#[cfg(feature = "tls")]
fn wss_and_https_validate_with_the_tls_feature() {
    forward("wss://example.com").validate().unwrap();
    ConnectionConfig::Http(HttpConfig {
        base_url: "https://example.com".to_owned(),
        access_token: None,
    })
    .validate()
    .unwrap();
}

#[test]
#[cfg(not(feature = "tls"))]
fn wss_and_https_rejected_without_the_tls_feature() {
    let err = forward("wss://example.com").validate().unwrap_err();
    assert!(
        err.contains("tls"),
        "error should mention the feature: {err}"
    );
    let cfg = ConnectionConfig::Http(HttpConfig {
        base_url: "https://example.com".to_owned(),
        access_token: None,
    });
    assert!(cfg.validate().unwrap_err().contains("tls"));
}

#[test]
fn webhook_path_must_start_with_slash() {
    let cfg = ConnectionConfig::HttpPost(HttpPostConfig {
        bind: "127.0.0.1:8080".parse().unwrap(),
        path: "onebot".to_owned(),
        secret: None,
        api: None,
    });
    assert!(cfg.validate().unwrap_err().contains("must start with `/`"));
}

#[test]
fn valid_configs_validate() {
    forward("ws://127.0.0.1:6700/").validate().unwrap();
    forward("ws://127.0.0.1:6700/api").validate().unwrap();

    ConnectionConfig::Http(HttpConfig {
        base_url: "http://127.0.0.1:5700".to_owned(),
        access_token: None,
    })
    .validate()
    .unwrap();

    ConnectionConfig::HttpPost(HttpPostConfig {
        bind: "127.0.0.1:8080".parse().unwrap(),
        path: "/".to_owned(),
        secret: None,
        api: Some(HttpConfig {
            base_url: "http://127.0.0.1:5700".to_owned(),
            access_token: None,
        }),
    })
    .validate()
    .unwrap();
}
