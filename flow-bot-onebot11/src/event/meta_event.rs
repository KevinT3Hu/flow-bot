use serde::{Deserialize, Serialize};

use crate::api::BotStatus;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleSubType {
    Enable,
    Disable,
    Connect,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lifecycle {
    pub sub_type: LifecycleSubType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Heartbeat {
    pub interval: i64,
    pub status: BotStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "meta_event_type", rename_all = "snake_case")]
pub enum MetaEvent {
    Lifecycle(Lifecycle),
    Heartbeat(Heartbeat),
}
