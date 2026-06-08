use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::json;
use tokio::sync::{Mutex, oneshot};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    api::{ApiResponse, api_ext::ApiExt},
    error::FlowError,
    event::BotEvent,
    extract::FromEvent,
};

pub struct Context {
    pub(crate) sink: Mutex<Option<tokio::sync::mpsc::Sender<Message>>>,
    pending_requests: Arc<DashMap<String, oneshot::Sender<String>>>,
    pub(crate) state: StateMap,
}

impl Context {
    pub(crate) fn new(states: StateMap) -> Self {
        #[allow(unused_mut)]
        let mut states = states;
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

        // Send message via channel; clone sender so we don't hold the mutex across await
        let sender = {
            let sink = self.sink.lock().await;
            sink.as_ref().ok_or(FlowError::NoConnection)?.clone()
        };
        sender
            .send(msg)
            .await
            .map_err(|_| FlowError::NoConnection)?;

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
        // DashMap::remove returns Option<(K, V)>, extract the sender
        if let Some((_, tx)) = self.pending_requests.remove(&echo) {
            let _ = tx.send(data); // Ignore error if receiver dropped
        }
        // If echo not found, response arrived after timeout - silently ignore
    }

    pub async fn get_self_id(&self) -> Result<i64, FlowError> {
        let info = self.get_login_info().await?;
        Ok(info.user_id)
    }
}

pub type BotContext = Arc<Context>;

#[async_trait]
pub trait BotContextExt {
    async fn extract<T: FromEvent>(&self, event: BotEvent) -> Option<T>;
}

#[async_trait]
impl BotContextExt for BotContext {
    async fn extract<T: FromEvent>(&self, event: BotEvent) -> Option<T> {
        T::from_event(Arc::clone(self), event).await
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
