use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::GroupHonorType;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GroupFile {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub busid: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GroupUpload {
    pub group_id: i64,
    pub user_id: i64,
    pub file: GroupFile,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupAdminSubType {
    Set,
    Unset,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GroupAdmin {
    pub group_id: i64,
    pub user_id: i64,
    pub sub_type: GroupAdminSubType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupDecreaseSubType {
    Leave,
    Kick,
    KickMe,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GroupDecrease {
    pub group_id: i64,
    pub user_id: i64,
    pub operator_id: i64,
    pub sub_type: GroupDecreaseSubType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupIncreaseSubType {
    Approve,
    Invite,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GroupIncrease {
    pub group_id: i64,
    pub user_id: i64,
    pub operator_id: i64,
    pub sub_type: GroupIncreaseSubType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupBanSubType {
    Ban,
    LiftBan,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GroupBan {
    pub group_id: i64,
    pub user_id: i64,
    pub operator_id: i64,
    pub sub_type: GroupBanSubType,
    pub duration: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FriendAdd {
    pub user_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GroupRecall {
    pub group_id: i64,
    pub user_id: i64,
    pub operator_id: i64,
    pub message_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FriendRecall {
    pub user_id: i64,
    pub message_id: i64,
}

/// 群内戳一戳 (`sub_type: poke`): someone poked someone. In private-chat
/// pokes implementations omit `group_id`, hence the `Option`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PokeEvent {
    pub group_id: Option<i64>,
    pub user_id: i64,
    pub target_id: i64,
}

/// 群红包运气王 (`sub_type: lucky_king`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LuckyKingEvent {
    pub group_id: i64,
    pub user_id: i64,
    pub target_id: i64,
}

/// 群成员荣誉变更 (`sub_type: honor`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HonorEvent {
    pub group_id: i64,
    pub user_id: i64,
    pub honor_type: GroupHonorType,
}

/// The typed shape of `notice_type: notify` events, dispatched on `sub_type`.
/// Sub-types outside the spec (implementations add e.g. `input_status`)
/// degrade to [`NotifyEvent::Unknown`] with the raw fields preserved, instead
/// of failing the whole event.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "sub_type", rename_all = "snake_case")]
pub enum NotifyEvent {
    Poke(PokeEvent),
    LuckyKing(LuckyKingEvent),
    Honor(HonorEvent),
    #[serde(untagged)]
    Unknown(HashMap<String, Value>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EssenceSubType {
    Add,
    Delete,
}

/// 精华消息 (`notice_type: essence`): a go-cqhttp de-facto extension adopted
/// by all major implementations; not part of the OneBot 11 standard.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EssenceEvent {
    pub group_id: i64,
    pub operator_id: i64,
    pub message_id: i64,
    pub sender_id: i64,
    pub sub_type: EssenceSubType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "notice_type")]
#[serde(rename_all = "snake_case")]
pub enum Notice {
    GroupUpload(GroupUpload),
    GroupAdmin(GroupAdmin),
    GroupDecrease(GroupDecrease),
    GroupIncrease(GroupIncrease),
    GroupBan(GroupBan),
    FriendAdd(FriendAdd),
    GroupRecall(GroupRecall),
    FriendRecall(FriendRecall),
    Notify(NotifyEvent),
    Essence(EssenceEvent),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn notice(value: serde_json::Value) -> Notice {
        serde_json::from_value(value).expect("notice event should parse")
    }

    #[test]
    fn poke_is_typed() {
        let poke = notice(json!({
            "notice_type": "notify", "sub_type": "poke",
            "group_id": 123, "user_id": 20000, "target_id": 10000,
        }));
        assert_eq!(
            poke,
            Notice::Notify(NotifyEvent::Poke(PokeEvent {
                group_id: Some(123),
                user_id: 20000,
                target_id: 10000,
            }))
        );
        // Private pokes carry no group_id.
        let poke = notice(json!({
            "notice_type": "notify", "sub_type": "poke",
            "user_id": 20000, "target_id": 10000,
        }));
        assert_eq!(
            poke,
            Notice::Notify(NotifyEvent::Poke(PokeEvent {
                group_id: None,
                user_id: 20000,
                target_id: 10000,
            }))
        );
    }

    #[test]
    fn lucky_king_and_honor_are_typed() {
        assert_eq!(
            notice(json!({
                "notice_type": "notify", "sub_type": "lucky_king",
                "group_id": 1, "user_id": 2, "target_id": 3,
            })),
            Notice::Notify(NotifyEvent::LuckyKing(LuckyKingEvent {
                group_id: 1,
                user_id: 2,
                target_id: 3,
            }))
        );
        assert_eq!(
            notice(json!({
                "notice_type": "notify", "sub_type": "honor",
                "group_id": 1, "user_id": 2, "honor_type": "talkative",
            })),
            Notice::Notify(NotifyEvent::Honor(HonorEvent {
                group_id: 1,
                user_id: 2,
                honor_type: GroupHonorType::Talkative,
            }))
        );
    }

    #[test]
    fn unknown_notify_sub_type_keeps_raw_fields() {
        assert_eq!(
            notice(json!({
                "notice_type": "notify", "sub_type": "input_status",
                "user_id": 2, "event": "input",
            })),
            Notice::Notify(NotifyEvent::Unknown(HashMap::from([
                ("sub_type".to_owned(), json!("input_status")),
                ("user_id".to_owned(), json!(2)),
                ("event".to_owned(), json!("input")),
            ])))
        );
    }

    #[test]
    fn essence_is_typed() {
        assert_eq!(
            notice(json!({
                "notice_type": "essence", "sub_type": "add",
                "group_id": 1, "operator_id": 2, "message_id": 3, "sender_id": 4,
            })),
            Notice::Essence(EssenceEvent {
                group_id: 1,
                operator_id: 2,
                message_id: 3,
                sender_id: 4,
                sub_type: EssenceSubType::Add,
            })
        );
    }
}
