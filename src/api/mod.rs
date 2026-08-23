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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

/// Where [`send_msg`](crate::api::api_ext::ApiExt::send_message) delivers a
/// message: a private chat or a group.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageTarget {
    Private { user_id: i64 },
    Group { group_id: i64 },
}

/// A quick operation on an event, delivered either through
/// [`handle_quick_operation`](crate::api::api_ext::ApiExt::handle_quick_operation)
/// or (for HTTP-POST webhooks) in the HTTP response body.
///
/// Which fields apply depends on the event: `reply`/`auto_escape` on message
/// events (`at_sender`/`delete`/`kick`/`ban`/`ban_duration` additionally on
/// group messages), `approve`/`remark` on friend-add requests and
/// `approve`/`reason` on group-add requests. Absent fields are omitted from
/// the wire and, per the spec, only present fields take effect.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct QuickOperation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<message::Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_escape: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_sender: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kick: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ban: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ban_duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approve: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl QuickOperation {
    /// Merge `other` into `self`, with `other`'s set fields winning. Used
    /// when several handlers attach quick operations to the same event.
    pub fn merge(&mut self, other: QuickOperation) {
        let QuickOperation {
            reply,
            auto_escape,
            at_sender,
            delete,
            kick,
            ban,
            ban_duration,
            approve,
            remark,
            reason,
        } = other;
        if reply.is_some() {
            self.reply = reply;
        }
        if auto_escape.is_some() {
            self.auto_escape = auto_escape;
        }
        if at_sender.is_some() {
            self.at_sender = at_sender;
        }
        if delete.is_some() {
            self.delete = delete;
        }
        if kick.is_some() {
            self.kick = kick;
        }
        if ban.is_some() {
            self.ban = ban;
        }
        if ban_duration.is_some() {
            self.ban_duration = ban_duration;
        }
        if approve.is_some() {
            self.approve = approve;
        }
        if remark.is_some() {
            self.remark = remark;
        }
        if reason.is_some() {
            self.reason = reason;
        }
    }
}

#[cfg(test)]
mod quick_operation_tests {
    use super::*;
    use crate::message::IntoMessage;

    #[test]
    fn none_fields_are_omitted_and_merge_last_write_wins() {
        let mut op = QuickOperation {
            reply: Some("hi".to_string().into_message()),
            ban: Some(true),
            ..Default::default()
        };
        let wire = serde_json::to_value(&op).unwrap();
        assert_eq!(
            wire,
            serde_json::json!({"reply": [{"type": "text", "data": {"text": "hi"}}], "ban": true})
        );

        op.merge(QuickOperation {
            ban_duration: Some(600),
            ban: Some(false),
            ..Default::default()
        });
        assert_eq!(op.ban, Some(false));
        assert_eq!(op.ban_duration, Some(600));
        assert!(op.kick.is_none());
    }
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
