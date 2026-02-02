use flow_bot::{
    FlowBotBuilder,
    base::{
        connect::ReverseConnectionConfig, extract::MessageBody, handler::HandlerControl,
        service::Service,
    },
    flow_service,
};

async fn on_message(msg: MessageBody) -> HandlerControl {
    println!("{:?}", msg.message);
    HandlerControl::Continue
}

pub struct SimpleService;

#[flow_service]
impl Service for SimpleService {
    async fn handle_msg(&self, msg: MessageBody) -> HandlerControl {
        println!("Service received message: {:?}", msg.message);
        HandlerControl::Continue
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let bot = FlowBotBuilder::new(ReverseConnectionConfig {
        target: "ws://localhost:19999".to_string(),
        auth: None,
    })
    .with_state(())
    .with_service(SimpleService)
    .with_handler(on_message)
    .build();

    bot.run().await.unwrap();
}
