use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::impl_from_event;

#[derive(Deserialize, Debug, Clone)]
pub struct GroupFile {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub busid: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupUpload {
    pub group_id: i64,
    pub user_id: i64,
    pub file: GroupFile,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupAdminSubType {
    Set,
    Unset,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupAdmin {
    pub group_id: i64,
    pub user_id: i64,
    pub sub_type: GroupAdminSubType,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupDecreaseSubType {
    Leave,
    Kick,
    KickMe,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupDecrease {
    pub group_id: i64,
    pub user_id: i64,
    pub operator_id: i64,
    pub sub_type: GroupDecreaseSubType,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupIncreaseSubType {
    Approve,
    Invite,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupIncrease {
    pub group_id: i64,
    pub user_id: i64,
    pub operator_id: i64,
    pub sub_type: GroupIncreaseSubType,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupBanSubType {
    Ban,
    LiftBan,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupBan {
    pub group_id: i64,
    pub user_id: i64,
    pub operator_id: i64,
    pub sub_type: GroupBanSubType,
    pub duration: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FriendAdd {
    pub user_id: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupRecall {
    pub group_id: i64,
    pub user_id: i64,
    pub operator_id: i64,
    pub message_id: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FriendRecall {
    pub user_id: i64,
    pub message_id: i64,
}

#[derive(Deserialize, Debug, Clone)]
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
    Notify {
        #[serde(flatten)]
        data: HashMap<String, Value>,
    },
}

impl_from_event!(Notice);

impl_from_event!(Notice, GroupUpload);

impl_from_event!(Notice, GroupAdmin);

impl_from_event!(Notice, GroupDecrease);

impl_from_event!(Notice, GroupIncrease);

impl_from_event!(Notice, GroupBan);

impl_from_event!(Notice, FriendAdd);

impl_from_event!(Notice, GroupRecall);

impl_from_event!(Notice, FriendRecall);
