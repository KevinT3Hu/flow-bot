use async_trait::async_trait;

use serde_json::Value;

use crate::{
    api::{MessageTarget, QuickOperation},
    event::{BotEvent, message::GroupAnonymousInfo, request::GroupRequestSubType},
    message::IntoMessage,
};

use super::{
    BotStatus, CanSendResponse, FriendInfo, GetCookiesResponse, GetCredentialsResponse,
    GetCsrfTokenResponse, GetFileResponse, GetForwardResponse, GetMessageResponse, GroupHonorInfo,
    GroupHonorType, GroupInfoResponse, GroupMemberInfo, LoginInfo, RecordFormat,
    SendMessageResponse, StrangerInfo, VersionInfo,
};

#[async_trait]
pub trait ApiExt {
    type Error;

    async fn send_private_message<M>(
        &self,
        user_id: i64,
        message: M,
        auto_escape: Option<bool>,
    ) -> Result<SendMessageResponse, Self::Error>
    where
        M: IntoMessage + Send;

    async fn send_group_message<M>(
        &self,
        group_id: i64,
        message: M,
        auto_escape: Option<bool>,
    ) -> Result<SendMessageResponse, Self::Error>
    where
        M: IntoMessage + Send;

    /// Send a message through the spec's generic `send_msg` action, choosing
    /// the chat with a [`MessageTarget`].
    ///
    /// `auto_escape` is not a parameter here: it only affects string-form
    /// messages, while flow-bot always sends the array form. For `_async` or
    /// `_rate_limited` delivery of any action (spec §异步调用/限速调用), use
    /// [`call_action`](Self::call_action) with the suffixed action name,
    /// e.g. `send_group_msg_rate_limited`.
    async fn send_message<M>(
        &self,
        target: MessageTarget,
        message: M,
    ) -> Result<SendMessageResponse, Self::Error>
    where
        M: IntoMessage + Send;

    /// Call an arbitrary OneBot action with raw JSON params, returning the
    /// raw `data` field of the response.
    ///
    /// This is the escape hatch for implementation-specific actions and for
    /// the spec's `_async` and `_rate_limited` action suffixes, which queue
    /// the call on the implementation side and answer with
    /// `status: "async"`.
    async fn call_action(&self, action: &str, params: Value) -> Result<Value, Self::Error>;

    /// Attach a quick operation to an event (spec: 快速操作).
    ///
    /// Over the HTTP-POST communication type, the operation is carried in
    /// the pending webhook response body as long as the event's handlers are
    /// still running (or until the configured
    /// [`response_timeout`](crate::HttpPostConfig::response_timeout));
    /// afterwards — and always on the other communication types — it is
    /// delivered through the spec's hidden `.handle_quick_operation` API
    /// action, which requires a working API connection.
    async fn handle_quick_operation(
        &self,
        event: BotEvent,
        operation: QuickOperation,
    ) -> Result<(), Self::Error>;

    /// Quote-reply `message` to the sender of a message event, using the
    /// event's own message id and chat. Errors with
    /// [`FlowError::NotAMessageEvent`](crate::error::FlowError::NotAMessageEvent)
    /// on non-message events.
    async fn reply<M>(
        &self,
        event: BotEvent,
        message: M,
    ) -> Result<SendMessageResponse, Self::Error>
    where
        M: IntoMessage + Send;

    async fn delete_message(&self, message_id: i64) -> Result<(), Self::Error>;

    async fn get_message(&self, message_id: i64) -> Result<GetMessageResponse, Self::Error>;

    /// Fetch a forwarded message's content by its forward id (the `id` of a
    /// `forward` segment), per the spec parameter of `get_forward_msg`.
    async fn get_forward_message(&self, id: String) -> Result<GetForwardResponse, Self::Error>;

    async fn send_like(&self, user_id: i64, times: Option<i32>) -> Result<(), Self::Error>;

    async fn set_group_kick(
        &self,
        group_id: i64,
        user_id: i64,
        reject_add_request: Option<bool>,
    ) -> Result<(), Self::Error>;

    async fn set_group_ban(
        &self,
        group_id: i64,
        user_id: i64,
        duration: Option<i64>,
    ) -> Result<(), Self::Error>;

    async fn set_group_anonymous_ban(
        &self,
        group_id: i64,
        anonymous: Option<GroupAnonymousInfo>,
        flag: Option<String>,
        duration: Option<i64>,
    ) -> Result<(), Self::Error>;

    async fn set_whole_group_ban(
        &self,
        group_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error>;

    async fn set_group_admin(
        &self,
        group_id: i64,
        user_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error>;

    async fn set_group_anonymous(
        &self,
        group_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error>;

    async fn set_group_card(
        &self,
        group_id: i64,
        user_id: i64,
        card: Option<String>,
    ) -> Result<(), Self::Error>;

    async fn set_group_name(&self, group_id: i64, group_name: String) -> Result<(), Self::Error>;

    async fn set_group_leave(
        &self,
        group_id: i64,
        is_dismiss: Option<bool>,
    ) -> Result<(), Self::Error>;

    async fn set_group_special_title(
        &self,
        group_id: i64,
        user_id: i64,
        special_title: Option<String>,
        duration: Option<i64>,
    ) -> Result<(), Self::Error>;

    async fn set_friend_add_request(
        &self,
        flag: String,
        approve: Option<bool>,
        remark: Option<String>,
    ) -> Result<(), Self::Error>;

    async fn set_group_add_request(
        &self,
        flag: String,
        sub_type: GroupRequestSubType,
        approve: Option<bool>,
        reason: Option<String>,
    ) -> Result<(), Self::Error>;

    async fn get_login_info(&self) -> Result<LoginInfo, Self::Error>;

    async fn get_stranger_info(
        &self,
        user_id: i64,
        no_cache: Option<bool>,
    ) -> Result<StrangerInfo, Self::Error>;

    async fn get_friend_list(&self) -> Result<Vec<FriendInfo>, Self::Error>;

    async fn get_group_info(
        &self,
        group_id: i64,
        no_cache: Option<bool>,
    ) -> Result<GroupInfoResponse, Self::Error>;

    async fn get_group_list(&self) -> Result<Vec<GroupInfoResponse>, Self::Error>;

    async fn get_group_member_info(
        &self,
        group_id: i64,
        user_id: i64,
        no_cache: Option<bool>,
    ) -> Result<GroupMemberInfo, Self::Error>;

    async fn get_group_member_list(
        &self,
        group_id: i64,
    ) -> Result<Vec<GroupMemberInfo>, Self::Error>;

    async fn get_group_honor_info(
        &self,
        group_id: i64,
        ty: GroupHonorType,
    ) -> Result<GroupHonorInfo, Self::Error>;

    async fn get_cookies(&self, domain: Option<String>) -> Result<GetCookiesResponse, Self::Error>;

    async fn get_csrf_token(&self) -> Result<GetCsrfTokenResponse, Self::Error>;

    async fn get_credentials(
        &self,
        domain: Option<String>,
    ) -> Result<GetCredentialsResponse, Self::Error>;

    async fn get_record(
        &self,
        file: String,
        out_format: RecordFormat,
    ) -> Result<GetFileResponse, Self::Error>;

    async fn get_image(&self, file: String) -> Result<GetFileResponse, Self::Error>;

    async fn can_send_image(&self) -> Result<CanSendResponse, Self::Error>;

    async fn can_send_record(&self) -> Result<CanSendResponse, Self::Error>;

    async fn get_status(&self) -> Result<BotStatus, Self::Error>;

    async fn get_version_info(&self) -> Result<VersionInfo, Self::Error>;

    async fn set_restart(&self, delay: Option<i32>) -> Result<(), Self::Error>;

    async fn clean_cache(&self) -> Result<(), Self::Error>;
}
