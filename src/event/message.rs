use serde::{Deserialize, Serialize};

use crate::message::{
    self, IntoMessage,
    segments::{ReplySegment, Segment},
};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PrivateSubType {
    Friend,
    Group,
    Other,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupSubType {
    Normal,
    Anonymous,
    Notice,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SenderSex {
    Male,
    Female,
    Unknown,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrivateSenderInfo {
    pub user_id: Option<i64>,
    pub nickname: Option<String>,
    pub sex: Option<SenderSex>,
    pub age: Option<i32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrivateMessageInfo {
    pub sub_type: PrivateSubType,
    pub sender: PrivateSenderInfo,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupSenderRole {
    Owner,
    Admin,
    Member,
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct GroupMessageInfo {
    pub sub_type: GroupSubType,
    pub group_id: i64,
    pub sender: GroupSenderInfo,
    pub anonymous: Option<GroupAnonymousInfo>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "message_type")]
#[serde(rename_all = "snake_case")]
pub enum TypedMessageInfo {
    Group(GroupMessageInfo),
    Private(PrivateMessageInfo),
}

#[derive(Deserialize, Debug, Clone)]
pub struct Message {
    pub message_id: i32,
    pub user_id: i64,
    pub message: message::Message,
    pub raw_message: String,
    pub font: i32,
    #[serde(flatten)]
    pub info: TypedMessageInfo,
}

impl Message {
    pub fn reply<T>(&self, message: T) -> message::Message
    where
        T: IntoMessage,
    {
        let mut ret = vec![Segment::Reply(ReplySegment {
            id: self.message_id.to_string(),
        })];

        ret.extend(message.into_message());
        ret
    }
}
