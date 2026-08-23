use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use tokio::sync::Notify;

use crate::{
    api::{ApiResponse, QuickOperation, api_ext::ApiExt, parse_api_response},
    base::transport::ApiTransport,
    error::FlowError,
    event::{BotEvent, Event},
    extract::FromEvent,
};

/// A quick operation attached to an event whose HTTP-POST response is still
/// pending: the operation accumulated by handlers so far, plus the
/// completion signal the webhook response waits on.
#[derive(Default)]
pub(crate) struct QuickOpSlot {
    pub(crate) operation: Mutex<Option<QuickOperation>>,
    pub(crate) done: Notify,
}

pub struct Context {
    transport: RwLock<Option<Arc<dyn ApiTransport>>>,
    api_timeout: Duration,
    /// Quick-op slots for events whose HTTP-POST response is still pending,
    /// keyed by the event allocation's address (stable across `Arc` clones,
    /// unique while the dispatched event is alive).
    quick_ops: Mutex<HashMap<usize, Arc<QuickOpSlot>>>,
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
            quick_ops: Mutex::new(HashMap::new()),
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

    /// Register the quick-op slot for an event whose HTTP-POST response will
    /// wait for the handler chain. Must be called before the event is
    /// enqueued.
    pub(crate) fn register_quick_op(&self, event: &Event) -> Arc<QuickOpSlot> {
        let slot = Arc::new(QuickOpSlot::default());
        self.quick_ops
            .lock()
            .expect("quick-op lock poisoned")
            .insert(event_key(event), slot.clone());
        slot
    }

    /// Attach a quick operation to an event with a pending response,
    /// merging with any operation attached earlier. Returns `false` when no
    /// response is pending (other transports, or the response already went
    /// out), in which case the caller should fall back to the
    /// `.handle_quick_operation` API action.
    pub(crate) fn attach_quick_op(&self, event: &Event, operation: QuickOperation) -> bool {
        let Some(slot) = self
            .quick_ops
            .lock()
            .expect("quick-op lock poisoned")
            .get(&event_key(event))
            .cloned()
        else {
            return false;
        };
        let mut guard = slot.operation.lock().expect("quick-op lock poisoned");
        match &mut *guard {
            Some(existing) => existing.merge(operation),
            slot @ None => *slot = Some(operation),
        }
        true
    }

    /// Drop the quick-op slot without signaling completion (used when the
    /// response deadline expires or the event was never dispatched).
    pub(crate) fn remove_quick_op(&self, event: &Event) {
        self.quick_ops
            .lock()
            .expect("quick-op lock poisoned")
            .remove(&event_key(event));
    }

    /// Signal that the handler chain finished with `event`, waking the
    /// HTTP-POST response waiting to collect a quick operation.
    pub(crate) fn finish_event(&self, event: &Event) {
        if let Some(slot) = self
            .quick_ops
            .lock()
            .expect("quick-op lock poisoned")
            .remove(&event_key(event))
        {
            slot.done.notify_waiters();
        }
    }
}

/// Events are keyed by allocation address: the dispatched `Arc<Event>` (and
/// every clone handed to handlers) shares one address, which stays unique
/// for as long as the event is being processed.
fn event_key(event: &Event) -> usize {
    std::ptr::from_ref(event) as usize
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
