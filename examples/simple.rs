use flow_bot::{
    FlowBotBuilder,
    base::{connect::ReverseConnectionConfig, extract::Message, handler::HandlerControl},
};

async fn on_message(msg: Message) -> HandlerControl {
    println!("{:?}", msg.message);
    HandlerControl::Continue
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let bot = FlowBotBuilder::new(ReverseConnectionConfig {
        target: "ws://localhost:19999".to_string(),
        auth: None,
    })
    .with_state(())
    .with_handler(on_message)
    .build();

    bot.run().await.unwrap();
}
