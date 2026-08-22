use flow_bot::{
    FlowBotBuilder,
    base::{
        control::{HandlerControl, HandlerError},
        transport::{ConnectionConfig, ForwardWebSocketConfig, ReconnectionStrategy},
    },
    extract::MessageBody,
};

async fn on_message(MessageBody(msg): MessageBody) -> Result<HandlerControl, HandlerError> {
    println!("{:?}", msg);
    Ok(HandlerControl::Continue)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let bot = FlowBotBuilder::new(ConnectionConfig::ForwardWebSocket(ForwardWebSocketConfig {
        url: "ws://localhost:19999".to_string(),
        access_token: None,
        reconnection: ReconnectionStrategy::None,
    }))
    .with_state(())
    .with_handler(on_message)
    .build();

    bot.run().await.unwrap();
}
