use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::{
    api::{ApiResponse, api_ext::ApiExt, parse_api_response},
    base::transport::ApiTransport,
    error::FlowError,
    event::BotEvent,
    extract::FromEvent,
};

pub struct Context {
    transport: RwLock<Option<Arc<dyn ApiTransport>>>,
    api_timeout: Duration,
    pub(crate) state: StateMap,
}

impl Context {
    pub(crate) fn new(states: StateMap, api_timeout: Duration) -> Self {
        #[allow(unused_mut)]
        let mut states = states;
        #[cfg(feature = "turso")]
        {
            use crate::extensions::turso::TursoDispatcher;
            states.insert(TursoDispatcher::new());
        }

        Self {
            transport: RwLock::new(None),
            api_timeout,
            state: states,
        }
    }

    /// Install the transport used for outbound API calls (replaces any
    /// previous one, e.g. on reconnection).
    pub(crate) fn set_transport(&self, transport: Arc<dyn ApiTransport>) {
        *self.transport.write().expect("transport lock poisoned") = Some(transport);
    }

    pub(crate) fn clear_transport(&self) {
        *self.transport.write().expect("transport lock poisoned") = None;
    }

    /// Clear the transport only if it is still `transport` (a newer connection
    /// may have taken over the slot in the meantime).
    pub(crate) fn clear_transport_if(&self, transport: &Arc<dyn ApiTransport>) {
        let mut guard = self.transport.write().expect("transport lock poisoned");
        if guard
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, transport))
        {
            *guard = None;
        }
    }

    pub(crate) async fn send_obj<T, R>(
        &self,
        action: String,
        obj: T,
    ) -> Result<ApiResponse<R>, FlowError>
    where
        T: serde::Serialize,
        R: DeserializeOwned,
    {
        let transport = self
            .transport
            .read()
            .expect("transport lock poisoned")
            .clone()
            .ok_or(FlowError::NoConnection)?;
        let params = serde_json::to_value(obj)?;
        let raw = transport
            .send_request(&action, params, self.api_timeout)
            .await?;
        parse_api_response(&raw)
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
