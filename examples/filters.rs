use flow_bot::{
    FlowBotBuilder,
    base::{
        control::{HandlerControl, HandlerError},
        transport::{ConnectionConfig, ForwardWebSocketConfig, ReconnectionStrategy},
    },
    extract::Message,
    flow_filter, group_message, private_message,
};

#[group_message]
async fn on_group_message(msg: Message) -> Result<HandlerControl, HandlerError> {
    println!("Group message: {:?}", msg.message);
    Ok(HandlerControl::Continue)
}

#[private_message]
async fn on_private_message(msg: Message) -> Result<HandlerControl, HandlerError> {
    println!("Private message: {:?}", msg.message);
    Ok(HandlerControl::Continue)
}

#[flow_filter(guard = flow_bot::extract::filters::IsGroupMessage)]
async fn on_group_with_filter(msg: Message) -> Result<HandlerControl, HandlerError> {
    println!("Group message via #[filter]: {:?}", msg.message);
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
    .with_handler(on_group_message)
    .with_handler(on_private_message)
    .with_handler(on_group_with_filter)
    .build();

    bot.run().await.unwrap();
}
