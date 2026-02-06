use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use async_trait::async_trait;
use futures::{SinkExt, lock::Mutex, stream::SplitSink};
use serde_json::json;
use tokio::{net::TcpStream, sync::broadcast::Sender};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

use crate::{
    api::{ApiResponse, api_ext::ApiExt},
    error::FlowError,
    event::BotEvent,
};

use super::extract::FromEvent;

pub struct Context {
    pub(crate) sink: Mutex<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    sender: Sender<(String, String)>,
    pub(crate) state: StateMap,
}

impl Context {
    pub(crate) fn new(states: StateMap) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(10);
        Self {
            sink: Mutex::new(None),
            sender,
            state: states,
        }
    }
}

impl Context {
    pub(crate) async fn send_obj<T, R>(
        &self,
        action: String,
        obj: T,
    ) -> Result<ApiResponse<R>, FlowError>
    where
        T: serde::Serialize,
        R: for<'de> serde::Deserialize<'de>,
    {
        // generate random echo string
        let echo = uuid::Uuid::new_v4().to_string();
        let msg = json!({
            "action": action,
            "params": obj,
            "echo": echo,
        });
        let text = serde_json::to_string(&msg)?;
        let msg = Message::Text(text.into());
        let mut sink = self.sink.lock().await;
        let sink = sink.as_mut().ok_or(FlowError::NoConnection)?;
        sink.send(msg).await?;

        let mut recv = self.sender.subscribe();
        while let Ok((e, r)) = recv.recv().await {
            if e == echo {
                return Ok(serde_json::from_str(&r)?);
            }
        }
        Err(FlowError::NoResponse)
    }

    pub(crate) fn on_recv_echo(&self, echo: String, data: String) {
        let _ = self.sender.send((echo, data));
    }

    pub async fn get_self_id(&self) -> Result<i64, FlowError> {
        let info = self.get_login_info().await?;
        Ok(info.user_id)
    }
}

pub type BotContext = Arc<Context>;

#[async_trait]
impl FromEvent for BotContext {
    async fn from_event(context: BotContext, _: BotEvent) -> Option<Self> {
        Some(context)
    }
}

pub(crate) struct StateMap {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl StateMap {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn insert<T: Any + Send + Sync>(&mut self, state: T) {
        self.map.insert(TypeId::of::<T>(), Arc::new(state));
    }

    pub(crate) fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|state| Arc::clone(state).downcast::<T>().ok())
    }
}
