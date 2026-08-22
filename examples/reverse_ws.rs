//! Reverse WebSocket: flow-bot runs a WebSocket server and the OneBot
//! implementation connects to it. Configure the implementation's
//! `ws_reverse.url` to point at this server.

use flow_bot::{
    FlowBotBuilder,
    base::{
        control::{HandlerControl, HandlerError},
        transport::{ConnectionConfig, ReverseWebSocketConfig},
    },
    extract::Message,
};

async fn on_message(msg: Message) -> Result<HandlerControl, HandlerError> {
    println!("{:?}", msg.message);
    Ok(HandlerControl::Continue)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let bot = FlowBotBuilder::new(ConnectionConfig::ReverseWebSocket(ReverseWebSocketConfig {
        bind: "127.0.0.1:8080".parse().unwrap(),
        path: Some("/ws".to_string()),
        access_token: None,
    }))
    .with_state(())
    .with_handler(on_message)
    .build();

    bot.run().await.unwrap();
}
