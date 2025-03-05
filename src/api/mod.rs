use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    event::message::{GroupSenderInfo, GroupSenderRole, PrivateSenderInfo, SenderSex},
    message,
};

pub mod api_ext;
pub mod api_impl;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ApiRetStatus {
    Ok,
    Async,
    Failed,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiResponse<T> {
    pub status: ApiRetStatus,
    pub retcode: i32,
    pub data: T,
    pub echo: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BotStatus {
    pub online: Option<bool>,
    pub good: bool,
    #[serde(flatten)]
    pub data: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SendMessageResponse {
    pub message_id: i64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum GetMessageType {
    Private { sender: PrivateSenderInfo },
    Group { sender: GroupSenderInfo },
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetMessageResponse {
    pub time: i32,
    pub message_id: i32,
    pub real_id: i32,
    pub message: message::Message,
    #[serde(flatten)]
    pub ty: GetMessageType,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetForwardResponse {
    pub message: message::Message,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LoginInfo {
    pub user_id: i64,
    pub nickname: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StrangerInfo {
    pub user_id: i64,
    pub nickname: String,
    pub sex: SenderSex,
    pub age: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FriendInfo {
    pub user_id: i64,
    pub nickname: String,
    pub remark: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupInfoResponse {
    pub group_id: i64,
    pub group_name: String,
    pub member_count: i32,
    pub max_member_count: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupMemberInfo {
    pub group_id: i64,
    pub user_id: i64,
    pub nickname: String,
    pub card: String,
    pub sex: SenderSex,
    pub age: i32,
    pub area: Option<String>,
    pub join_time: i32,
    pub last_sent_time: i32,
    pub level: String,
    pub role: GroupSenderRole,
    pub unfriendly: bool,
    pub title: Option<String>,
    pub title_expire_time: i32,
    pub card_changeable: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TalkativeInfo {
    pub user_id: i64,
    pub nickname: String,
    pub avatar: String,
    pub day_count: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HonorInfo {
    pub user_id: i64,
    pub nickname: String,
    pub avatar: String,
    pub description: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupHonorInfo {
    pub group_id: i64,
    pub current_talkative: Option<TalkativeInfo>,
    pub talkative_list: Option<Vec<HonorInfo>>,
    pub performer_list: Option<Vec<HonorInfo>>,
    pub legend_list: Option<Vec<HonorInfo>>,
    pub strong_newbie_list: Option<Vec<HonorInfo>>,
    pub emotion_list: Option<Vec<HonorInfo>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupHonorType {
    Talkative,
    Performer,
    Legend,
    StrongNewbie,
    Emotion,
    All,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetCookiesResponse {
    pub cookies: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetCsrfTokenResponse {
    pub token: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetCredentialsResponse {
    pub cookies: String,
    pub csrf_token: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetFileResponse {
    pub file: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RecordFormat {
    Mp3,
    Amr,
    Wma,
    M4a,
    Spx,
    Ogg,
    Wav,
    Flac,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CanSendResponse {
    pub yes: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VersionInfo {
    pub app_name: String,
    pub app_version: String,
    pub protocol_version: String,
    #[serde(flatten)]
    pub data: HashMap<String, String>,
}
