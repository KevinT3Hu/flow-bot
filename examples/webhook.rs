//! HTTP POST (webhook): flow-bot runs an HTTP server and the OneBot
//! implementation POSTs events to it. Configure the implementation's
//! `http_post.url` to point at this server; API calls go through the
//! implementation's HTTP API endpoint configured below.

use flow_bot::{
    FlowBotBuilder,
    base::{
        control::{HandlerControl, HandlerError},
        transport::{ConnectionConfig, HttpConfig, HttpPostConfig},
    },
    extract::Message,
};

async fn on_message(msg: Message) -> Result<HandlerControl, HandlerError> {
    println!("{:?}", msg.message);
    Ok(HandlerControl::Continue)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let bot = FlowBotBuilder::new(ConnectionConfig::HttpPost(HttpPostConfig {
        bind: "127.0.0.1:8080".parse().unwrap(),
        path: "/".to_string(),
        // Verify `X-Signature` headers when the implementation sends them.
        secret: None,
        // Endpoint for outbound API calls.
        api: Some(HttpConfig {
            base_url: "http://127.0.0.1:5700".to_string(),
            access_token: None,
        }),
    }))
    .with_state(())
    .with_handler(on_message)
    .build();

    bot.run().await.unwrap();
}
