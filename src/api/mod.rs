use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::FlowError,
    event::message::{GroupSenderInfo, GroupSenderRole, PrivateSenderInfo, SenderSex},
    message,
};

pub mod api_ext;
pub mod api_impl;

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiRetStatus {
    Ok,
    /// The request was queued by the implementation; the final outcome is
    /// unknowable (`retcode` is 1, `data` is null).
    Async,
    Failed,
    /// Any unrecognized `status` string; treated as a failure.
    #[default]
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiResponse<T> {
    pub status: ApiRetStatus,
    pub retcode: i32,
    pub data: T,
    /// The request `echo`, returned verbatim by the implementation (any JSON
    /// type per the spec; absent in HTTP responses).
    #[serde(default)]
    pub echo: Option<Value>,
}

/// Parse a raw API response envelope and check its status/retcode before
/// deserializing `data`, so a failed call surfaces as
/// [`FlowError::ApiError`](crate::error::FlowError::ApiError) carrying the
/// implementation's wording instead of a confusing null-deserialization error.
pub(crate) fn parse_api_response<R: serde::de::DeserializeOwned>(
    raw: &str,
) -> Result<ApiResponse<R>, FlowError> {
    let value: Value = serde_json::from_str(raw)?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let retcode = value.get("retcode").and_then(Value::as_i64).unwrap_or(-1) as i32;

    let succeeded = status == "ok" || status == "async";
    if !succeeded || !(retcode == 0 || retcode == 1) {
        let message = ["wording", "message", "msg", "info"]
            .iter()
            .find_map(|key| value.get(key).and_then(Value::as_str))
            .map(str::to_owned);
        return Err(FlowError::ApiError {
            status,
            retcode,
            message,
        });
    }

    let status_enum = serde_json::from_value(value.get("status").cloned().unwrap_or(Value::Null))
        .unwrap_or(ApiRetStatus::Other);
    let data = value.get("data").cloned().unwrap_or(Value::Null);
    Ok(ApiResponse {
        status: status_enum,
        retcode,
        data: serde_json::from_value(data)?,
        echo: value.get("echo").cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_successful_envelope() {
        let resp: ApiResponse<serde_json::Value> =
            parse_api_response(r#"{"status":"ok","retcode":0,"data":{"user_id":1},"echo":"42"}"#)
                .unwrap();
        assert!(matches!(resp.status, ApiRetStatus::Ok));
        assert_eq!(resp.data["user_id"], 1);
        assert_eq!(resp.echo, Some(Value::String("42".into())));
    }

    #[test]
    fn checks_retcode_even_when_status_is_ok() {
        let err = parse_api_response::<serde_json::Value>(
            r#"{"status":"ok","retcode":1200,"data":null,"wording":"nope"}"#,
        )
        .unwrap_err();
        match err {
            crate::error::FlowError::ApiError {
                retcode, message, ..
            } => {
                assert_eq!(retcode, 1200);
                assert_eq!(message.as_deref(), Some("nope"));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[test]
    fn failed_status_with_implementation_wording() {
        let err = parse_api_response::<serde_json::Value>(
            r#"{"status":"failed","retcode":1404,"data":null,"message":"API not found"}"#,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            crate::error::FlowError::ApiError { retcode: 1404, .. }
        ));
    }

    #[test]
    fn async_status_with_null_data_is_ok() {
        let resp: ApiResponse<()> =
            parse_api_response(r#"{"status":"async","retcode":1,"data":null}"#).unwrap();
        assert!(matches!(resp.status, ApiRetStatus::Async));
    }

    #[test]
    fn unknown_status_string_is_treated_as_failure() {
        let err =
            parse_api_response::<serde_json::Value>(r#"{"status":"weird","retcode":0,"data":{}}"#)
                .unwrap_err();
        assert!(matches!(
            err,
            crate::error::FlowError::ApiError { retcode: 0, .. }
        ));
    }

    #[test]
    fn missing_status_or_retcode_is_failure() {
        assert!(parse_api_response::<serde_json::Value>(r#"{"data":{}}"#).is_err());
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct BotStatus {
    pub online: Option<bool>,
    pub good: bool,
    /// Implementation-defined extra fields of arbitrary JSON types.
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
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
    /// The spec notes that list responses may omit several fields that are
    /// present in single-member responses, hence `Option`.
    pub join_time: Option<i32>,
    pub last_sent_time: Option<i32>,
    pub level: String,
    pub role: GroupSenderRole,
    pub unfriendly: Option<bool>,
    pub title: Option<String>,
    pub title_expire_time: Option<i32>,
    pub card_changeable: Option<bool>,
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
    /// Implementation-defined extra fields of arbitrary JSON types.
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}
