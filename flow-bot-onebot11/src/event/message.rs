use serde::{Deserialize, Serialize};

use crate::message;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PrivateSubType {
    Friend,
    Group,
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupSubType {
    Normal,
    Anonymous,
    Notice,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SenderSex {
    Male,
    Female,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivateSenderInfo {
    pub user_id: Option<i64>,
    pub nickname: Option<String>,
    pub sex: Option<SenderSex>,
    pub age: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivateMessageInfo {
    pub sub_type: PrivateSubType,
    pub sender: PrivateSenderInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupSenderRole {
    Owner,
    Admin,
    Member,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupSenderInfo {
    pub user_id: Option<i64>,
    pub nickname: Option<String>,
    pub card: Option<String>,
    pub sex: Option<SenderSex>,
    pub age: Option<i32>,
    pub area: Option<String>,
    pub level: Option<String>,
    pub role: Option<GroupSenderRole>,
    pub title: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupAnonymousInfo {
    pub id: i64,
    pub name: String,
    pub flag: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupMessageInfo {
    pub sub_type: GroupSubType,
    pub group_id: i64,
    pub sender: GroupSenderInfo,
    pub anonymous: Option<GroupAnonymousInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "message_type")]
#[serde(rename_all = "snake_case")]
pub enum TypedMessageInfo {
    Group(GroupMessageInfo),
    Private(PrivateMessageInfo),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub message_id: i32,
    pub user_id: i64,
    pub message: message::Message,
    pub raw_message: String,
    pub font: i32,
    #[serde(flatten)]
    pub info: TypedMessageInfo,
}
