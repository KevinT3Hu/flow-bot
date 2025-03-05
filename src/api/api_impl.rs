use async_trait::async_trait;
use serde_json::json;

use crate::{
    base::context::Context,
    error::FlowError,
    event::{message::GroupAnonymousInfo, request::GroupRequestSubType},
    message::IntoMessage,
};

use super::{
    BotStatus, CanSendResponse, FriendInfo, GetCookiesResponse, GetCredentialsResponse,
    GetCsrfTokenResponse, GetFileResponse, GetForwardResponse, GetMessageResponse, GroupHonorInfo,
    GroupHonorType, GroupInfoResponse, LoginInfo, RecordFormat, SendMessageResponse, VersionInfo,
    api_ext::ApiExt,
};

macro_rules! impl_api {

    ($s:ident,$name:ident) => {{
        let resp = $s.send_obj(stringify!($name).to_string(), json!({})).await;
        resp.map(|r| r.data)
    }};

    ($s:ident,$name:ident, $($params:ident),*) => {{
            let params_json = json!({
                $(
                    stringify!($params): $params,
                )*
            });
            let resp = $s.send_obj(stringify!($name).to_string(), params_json).await;
            resp.map(|r| r.data)
    }};
}

#[async_trait]
impl ApiExt for Context {
    type Error = FlowError;

    async fn send_private_message<M>(
        &self,
        user_id: i64,
        message: M,
        auto_escape: Option<bool>,
    ) -> Result<SendMessageResponse, Self::Error>
    where
        M: IntoMessage + Send,
    {
        let message = message.into_message();
        let params_json = json!({
            "user_id": user_id,
            "message": message,
            "auto_escape": auto_escape,
        });
        let resp = self
            .send_obj("send_private_msg".to_string(), params_json)
            .await;
        resp.map(|r| r.data)
    }

    async fn send_group_message<M>(
        &self,
        group_id: i64,
        message: M,
        auto_escape: Option<bool>,
    ) -> Result<SendMessageResponse, Self::Error>
    where
        M: IntoMessage + Send,
    {
        let message = message.into_message();
        let params_json = json!({
            "group_id": group_id,
            "message": message,
            "auto_escape": auto_escape,
        });
        let resp = self
            .send_obj("send_group_msg".to_string(), params_json)
            .await;
        resp.map(|r| r.data)
    }

    async fn delete_message(&self, message_id: i64) -> Result<(), Self::Error> {
        impl_api!(self, delete_message, message_id)
    }

    async fn get_message(&self, message_id: i64) -> Result<GetMessageResponse, Self::Error> {
        impl_api!(self, get_msg, message_id)
    }

    async fn get_forward_message(
        &self,
        message_id: i64,
    ) -> Result<GetForwardResponse, Self::Error> {
        impl_api!(self, get_forward_msg, message_id)
    }

    async fn send_like(&self, user_id: i64, times: Option<i32>) -> Result<(), Self::Error> {
        impl_api!(self, send_like, user_id, times)
    }

    async fn set_group_kick(
        &self,
        group_id: i64,
        user_id: i64,
        reject_add_request: Option<bool>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_group_kick, group_id, user_id, reject_add_request)
    }

    async fn set_group_ban(
        &self,
        group_id: i64,
        user_id: i64,
        duration: Option<i64>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_group_ban, group_id, user_id, duration)
    }

    async fn set_group_anonymous_ban(
        &self,
        group_id: i64,
        anonymous: Option<GroupAnonymousInfo>,
        flag: Option<String>,
        duration: Option<i64>,
    ) -> Result<(), Self::Error> {
        impl_api!(
            self,
            set_group_anonymous_ban,
            group_id,
            anonymous,
            flag,
            duration
        )
    }

    async fn set_whole_group_ban(
        &self,
        group_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_whole_group_ban, group_id, enable)
    }

    async fn set_group_admin(
        &self,
        group_id: i64,
        user_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_group_admin, group_id, user_id, enable)
    }

    async fn set_group_anonymous(
        &self,
        group_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_group_anonymous, group_id, enable)
    }

    async fn set_group_card(
        &self,
        group_id: i64,
        user_id: i64,
        card: Option<String>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_group_card, group_id, user_id, card)
    }

    async fn set_group_name(&self, group_id: i64, group_name: String) -> Result<(), Self::Error> {
        impl_api!(self, set_group_name, group_id, group_name)
    }

    async fn set_group_leave(
        &self,
        group_id: i64,
        is_dismiss: Option<bool>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_group_leave, group_id, is_dismiss)
    }

    async fn set_group_special_title(
        &self,
        group_id: i64,
        user_id: i64,
        special_title: Option<String>,
        duration: Option<i64>,
    ) -> Result<(), Self::Error> {
        impl_api!(
            self,
            set_group_special_title,
            group_id,
            user_id,
            special_title,
            duration
        )
    }

    async fn set_friend_add_request(
        &self,
        flag: String,
        approve: Option<bool>,
        remark: Option<String>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_friend_add_request, flag, approve, remark)
    }

    async fn set_group_add_request(
        &self,
        flag: String,
        sub_type: GroupRequestSubType,
        approve: Option<bool>,
        reason: Option<String>,
    ) -> Result<(), Self::Error> {
        impl_api!(self, set_group_add_request, flag, sub_type, approve, reason)
    }

    async fn get_login_info(&self) -> Result<LoginInfo, Self::Error> {
        impl_api!(self, get_login_info)
    }

    async fn get_stranger_info(
        &self,
        user_id: i64,
        no_cache: Option<bool>,
    ) -> Result<crate::api::StrangerInfo, Self::Error> {
        impl_api!(self, get_stranger_info, user_id, no_cache)
    }

    async fn get_friend_list(&self) -> Result<Vec<FriendInfo>, Self::Error> {
        impl_api!(self, get_friend_list)
    }

    async fn get_group_info(
        &self,
        group_id: i64,
        no_cache: Option<bool>,
    ) -> Result<crate::api::GroupInfoResponse, Self::Error> {
        impl_api!(self, get_group_info, group_id, no_cache)
    }

    async fn get_group_list(&self) -> Result<Vec<GroupInfoResponse>, Self::Error> {
        impl_api!(self, get_group_list)
    }

    async fn get_group_member_info(
        &self,
        group_id: i64,
        user_id: i64,
        no_cache: Option<bool>,
    ) -> Result<crate::api::GroupInfoResponse, Self::Error> {
        impl_api!(self, get_group_member_info, group_id, user_id, no_cache)
    }

    async fn get_group_member_list(
        &self,
        group_id: i64,
    ) -> Result<Vec<crate::api::FriendInfo>, Self::Error> {
        impl_api!(self, get_group_member_list, group_id)
    }

    async fn get_group_honor_info(
        &self,
        group_id: i64,
        ty: GroupHonorType,
    ) -> Result<GroupHonorInfo, Self::Error> {
        let params_json = json!({
            "group_id": group_id,
            "type": ty,
        });
        let resp = self
            .send_obj("get_group_honor_info".to_string(), params_json)
            .await;
        resp.map(|r| r.data)
    }

    async fn get_cookies(&self, domain: Option<String>) -> Result<GetCookiesResponse, Self::Error> {
        impl_api!(self, get_cookies, domain)
    }

    async fn get_csrf_token(&self) -> Result<GetCsrfTokenResponse, Self::Error> {
        impl_api!(self, get_csrf_token)
    }

    async fn get_credentials(
        &self,
        domain: Option<String>,
    ) -> Result<GetCredentialsResponse, Self::Error> {
        impl_api!(self, get_credentials, domain)
    }

    async fn get_record(
        &self,
        file: String,
        out_format: RecordFormat,
    ) -> Result<GetFileResponse, Self::Error> {
        impl_api!(self, get_record, file, out_format)
    }

    async fn get_image(&self, file: String) -> Result<GetFileResponse, Self::Error> {
        impl_api!(self, get_image, file)
    }

    async fn can_send_image(&self) -> Result<CanSendResponse, Self::Error> {
        impl_api!(self, can_send_image)
    }

    async fn can_send_record(&self) -> Result<CanSendResponse, Self::Error> {
        impl_api!(self, can_send_record)
    }

    async fn get_status(&self) -> Result<BotStatus, Self::Error> {
        impl_api!(self, get_status)
    }

    async fn get_version_info(&self) -> Result<VersionInfo, Self::Error> {
        impl_api!(self, get_version_info)
    }

    async fn set_restart(&self, delay: Option<i32>) -> Result<(), Self::Error> {
        impl_api!(self, set_restart, delay)
    }

    async fn clean_cache(&self) -> Result<(), Self::Error> {
        impl_api!(self, clean_cache)
    }
}
