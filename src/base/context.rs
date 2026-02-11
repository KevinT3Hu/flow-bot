use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use async_trait::async_trait;
use dashmap::DashMap;
use futures::{SinkExt, stream::SplitSink};
use serde_json::json;
use tokio::{
    net::TcpStream,
    sync::{Mutex, oneshot},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

use crate::{
    api::{ApiResponse, api_ext::ApiExt},
    error::FlowError,
    event::BotEvent,
};

use super::extract::FromEvent;

pub struct Context {
    pub(crate) sink: Mutex<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    pending_requests: Arc<DashMap<String, oneshot::Sender<String>>>,
    pub(crate) state: StateMap,
}

impl Context {
    pub(crate) fn new(mut states: StateMap) -> Self {
        #[cfg(feature = "turso")]
        {
            use crate::extensions::turso::TursoDispatcher;
            states.insert(TursoDispatcher::new());
        }

        Self {
            sink: Mutex::new(None),
            pending_requests: Arc::new(DashMap::new()),
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
        // Generate random echo string
        let echo = uuid::Uuid::new_v4().to_string();

        // Create oneshot channel for this specific request
        let (tx, rx) = oneshot::channel();

        // Register the request BEFORE sending (lock-free)
        self.pending_requests.insert(echo.clone(), tx);

        // Build and send the message
        let msg = json!({
            "action": action,
            "params": obj,
            "echo": echo,
        });
        let text = serde_json::to_string(&msg)?;
        let msg = Message::Text(text.into());

        // Send message and release lock immediately
        {
            let mut sink = self.sink.lock().await;
            let sink = sink.as_mut().ok_or(FlowError::NoConnection)?;
            sink.send(msg).await?;
        }

        // Wait for response with timeout
        let response = tokio::time::timeout(std::time::Duration::from_secs(30), rx).await;

        match response {
            Ok(Ok(data)) => Ok(serde_json::from_str(&data)?),
            Ok(Err(_)) => Err(FlowError::NoResponse), // Sender dropped
            Err(_) => {
                // Timeout occurred, clean up the pending request (lock-free)
                self.pending_requests.remove(&echo);
                Err(FlowError::Timeout(30000))
            }
        }
    }

    pub(crate) fn on_recv_echo(&self, echo: String, data: String) {
        let pending_requests = self.pending_requests.clone();
        tokio::spawn(async move {
            // DashMap::remove returns Option<(K, V)>, extract the sender
            if let Some((_, tx)) = pending_requests.remove(&echo) {
                let _ = tx.send(data); // Ignore error if receiver dropped
            }
            // If echo not found, response arrived after timeout - silently ignore
        });
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
