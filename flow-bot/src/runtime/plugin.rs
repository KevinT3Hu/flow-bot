//! Plugin runtime implementation
//!
//! This module provides the implementation of the API Host trait that
//! WASM plugins can call into.

use std::path::PathBuf;

use crate::api::api_ext::ApiExt;
use crate::base::context::BotContext;
use crate::runtime::flow_bot::onebot11::api::Host;
use crate::runtime::flow_bot::onebot11::types as wit;
use crate::runtime::flow_bot::onebot11::types::Host as TypesHost;
use flow_bot_onebot11::api::GroupMemberInfo;
use serde_json::json;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

/// Plugin state that holds the bot context for API calls
pub struct PluginState {
    context: BotContext,
    table: ResourceTable,
    max_execution_time_ms: u64,
    wasi: WasiCtx,
}

impl PluginState {
    /// Create a new PluginState with the given context and WASI context
    pub fn new(context: BotContext, wasi: WasiCtx) -> Self {
        Self {
            context,
            table: ResourceTable::new(),
            max_execution_time_ms: 5000,
            wasi,
        }
    }

    /// Get a reference to the resource table
    pub fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    /// Get the maximum execution time in milliseconds
    pub fn max_execution_time_ms(&self) -> u64 {
        self.max_execution_time_ms
    }

    /// Set the maximum execution time in milliseconds
    pub fn set_max_execution_time_ms(&mut self, ms: u64) {
        self.max_execution_time_ms = ms;
    }
}

impl WasiView for PluginState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl TypesHost for PluginState {}

/// Information about a loaded plugin
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: PathBuf,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
}

impl PluginInfo {
    pub fn new(name: String, version: String, description: String, path: PathBuf) -> Self {
        Self {
            name,
            version,
            description,
            path,
            loaded_at: chrono::Utc::now(),
        }
    }
}

impl Host for PluginState {
    async fn send_private_message(
        &mut self,
        user_id: i64,
        message: String,
        auto_escape: Option<bool>,
    ) -> Result<wit::SendMessageResponse, String> {
        self.context
            .send_private_message(user_id, message, auto_escape)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn send_group_message(
        &mut self,
        group_id: i64,
        message: String,
        auto_escape: Option<bool>,
    ) -> Result<wit::SendMessageResponse, String> {
        self.context
            .send_group_message(group_id, message, auto_escape)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn delete_message(&mut self, message_id: i64) -> Result<(), String> {
        self.context
            .delete_message(message_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_message(&mut self, message_id: i64) -> Result<wit::GetMessageResponse, String> {
        self.context
            .get_message(message_id)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_forward_message(
        &mut self,
        message_id: i64,
    ) -> Result<wit::GetForwardResponse, String> {
        self.context
            .get_forward_message(message_id)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn send_like(&mut self, user_id: i64, times: Option<i32>) -> Result<(), String> {
        self.context
            .send_like(user_id, times)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_kick(
        &mut self,
        group_id: i64,
        user_id: i64,
        reject_add_request: Option<bool>,
    ) -> Result<(), String> {
        self.context
            .set_group_kick(group_id, user_id, reject_add_request)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_ban(
        &mut self,
        group_id: i64,
        user_id: i64,
        duration: Option<i64>,
    ) -> Result<(), String> {
        self.context
            .set_group_ban(group_id, user_id, duration)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_anonymous_ban(
        &mut self,
        group_id: i64,
        anonymous: Option<wit::GroupAnonymousInfo>,
        flag: Option<String>,
        duration: Option<i64>,
    ) -> Result<(), String> {
        let anonymous = anonymous.map(Into::into);
        self.context
            .set_group_anonymous_ban(group_id, anonymous, flag, duration)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_whole_group_ban(
        &mut self,
        group_id: i64,
        enable: Option<bool>,
    ) -> Result<(), String> {
        self.context
            .set_whole_group_ban(group_id, enable)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_admin(
        &mut self,
        group_id: i64,
        user_id: i64,
        enable: Option<bool>,
    ) -> Result<(), String> {
        self.context
            .set_group_admin(group_id, user_id, enable)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_anonymous(
        &mut self,
        group_id: i64,
        enable: Option<bool>,
    ) -> Result<(), String> {
        self.context
            .set_group_anonymous(group_id, enable)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_card(
        &mut self,
        group_id: i64,
        user_id: i64,
        card: Option<String>,
    ) -> Result<(), String> {
        self.context
            .set_group_card(group_id, user_id, card)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_name(&mut self, group_id: i64, group_name: String) -> Result<(), String> {
        self.context
            .set_group_name(group_id, group_name)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_leave(
        &mut self,
        group_id: i64,
        is_dismiss: Option<bool>,
    ) -> Result<(), String> {
        self.context
            .set_group_leave(group_id, is_dismiss)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_special_title(
        &mut self,
        group_id: i64,
        user_id: i64,
        special_title: Option<String>,
        duration: Option<i64>,
    ) -> Result<(), String> {
        self.context
            .set_group_special_title(group_id, user_id, special_title, duration)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_friend_add_request(
        &mut self,
        flag: String,
        approve: Option<bool>,
        remark: Option<String>,
    ) -> Result<(), String> {
        self.context
            .set_friend_add_request(flag, approve, remark)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_group_add_request(
        &mut self,
        flag: String,
        sub_type: wit::GroupRequestSubType,
        approve: Option<bool>,
        reason: Option<String>,
    ) -> Result<(), String> {
        let sub_type = sub_type.into();
        self.context
            .set_group_add_request(flag, sub_type, approve, reason)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_login_info(&mut self) -> Result<wit::LoginInfo, String> {
        self.context
            .get_login_info()
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_stranger_info(
        &mut self,
        user_id: i64,
        no_cache: Option<bool>,
    ) -> Result<wit::StrangerInfo, String> {
        self.context
            .get_stranger_info(user_id, no_cache)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_friend_list(&mut self) -> Result<Vec<wit::FriendInfo>, String> {
        self.context
            .get_friend_list()
            .await
            .map(|list| list.into_iter().map(Into::into).collect())
            .map_err(|e| e.to_string())
    }

    async fn get_group_info(
        &mut self,
        group_id: i64,
        no_cache: Option<bool>,
    ) -> Result<wit::GroupInfoResponse, String> {
        self.context
            .get_group_info(group_id, no_cache)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_group_list(&mut self) -> Result<Vec<wit::GroupInfoResponse>, String> {
        self.context
            .get_group_list()
            .await
            .map(|list| list.into_iter().map(Into::into).collect())
            .map_err(|e| e.to_string())
    }

    async fn get_group_member_info(
        &mut self,
        group_id: i64,
        user_id: i64,
        no_cache: Option<bool>,
    ) -> Result<wit::GroupMemberInfo, String> {
        // Call send_obj directly because ApiExt has wrong return type
        let params_json = json!({
            "group_id": group_id,
            "user_id": user_id,
            "no_cache": no_cache,
        });
        let result: Result<GroupMemberInfo, _> = self
            .context
            .send_obj("get_group_member_info".to_string(), params_json)
            .await
            .map(|r| r.data);
        result.map(Into::into).map_err(|e| e.to_string())
    }

    async fn get_group_member_list(
        &mut self,
        group_id: i64,
    ) -> Result<Vec<wit::GroupMemberInfo>, String> {
        // Call send_obj directly because ApiExt has wrong return type
        let params_json = json!({
            "group_id": group_id,
        });
        let result: Result<Vec<GroupMemberInfo>, _> = self
            .context
            .send_obj("get_group_member_list".to_string(), params_json)
            .await
            .map(|r| r.data);
        result
            .map(|list| list.into_iter().map(Into::into).collect())
            .map_err(|e| e.to_string())
    }

    async fn get_group_honor_info(
        &mut self,
        group_id: i64,
        honor_type: wit::GroupHonorType,
    ) -> Result<wit::GroupHonorInfo, String> {
        let honor_type = honor_type.into();
        self.context
            .get_group_honor_info(group_id, honor_type)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_cookies(
        &mut self,
        domain: Option<String>,
    ) -> Result<wit::GetCookiesResponse, String> {
        self.context
            .get_cookies(domain)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_csrf_token(&mut self) -> Result<wit::GetCsrfTokenResponse, String> {
        self.context
            .get_csrf_token()
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_credentials(
        &mut self,
        domain: Option<String>,
    ) -> Result<wit::GetCredentialsResponse, String> {
        self.context
            .get_credentials(domain)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_record(
        &mut self,
        file: String,
        out_format: wit::RecordFormat,
    ) -> Result<wit::GetFileResponse, String> {
        let out_format = out_format.into();
        self.context
            .get_record(file, out_format)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_image(&mut self, file: String) -> Result<wit::GetFileResponse, String> {
        self.context
            .get_image(file)
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn can_send_image(&mut self) -> Result<wit::CanSendResponse, String> {
        self.context
            .can_send_image()
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn can_send_record(&mut self) -> Result<wit::CanSendResponse, String> {
        self.context
            .can_send_record()
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_status(&mut self) -> Result<wit::BotStatus, String> {
        self.context
            .get_status()
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn get_version_info(&mut self) -> Result<wit::VersionInfo, String> {
        self.context
            .get_version_info()
            .await
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    async fn set_restart(&mut self, delay: Option<i32>) -> Result<(), String> {
        self.context
            .set_restart(delay)
            .await
            .map_err(|e| e.to_string())
    }

    async fn clean_cache(&mut self) -> Result<(), String> {
        self.context.clean_cache().await.map_err(|e| e.to_string())
    }
}
