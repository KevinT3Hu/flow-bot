use serde::Deserialize;

use crate::{api::BotStatus, impl_from_event};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleSubType {
    Enable,
    Disable,
    Connect,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Lifecycle {
    pub sub_type: LifecycleSubType,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Heartbeat {
    pub interval: i64,
    pub status: BotStatus,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "meta_event_type", rename_all = "snake_case")]
pub enum MetaEvent {
    Lifecycle(Lifecycle),
    Heartbeat(Heartbeat),
}

impl_from_event!(MetaEvent);

impl_from_event!(MetaEvent, Lifecycle);

impl_from_event!(MetaEvent, Heartbeat);
