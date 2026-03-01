#[cfg(feature = "api")]
use async_trait::async_trait;

use crate::event::message::GroupAnonymousInfo;
use crate::event::request::GroupRequestSubType;
use crate::message::IntoMessage;

use super::{
    BotStatus, CanSendResponse, FriendInfo, GetCookiesResponse, GetCredentialsResponse,
    GetCsrfTokenResponse, GetFileResponse, GetForwardResponse, GetMessageResponse, GroupHonorInfo,
    GroupHonorType, GroupInfoResponse, LoginInfo, RecordFormat, SendMessageResponse, StrangerInfo,
    VersionInfo,
};

/// Trait providing OneBot-11 API methods.
///
/// This trait defines the standard OneBot-11 API interface that can be implemented
/// by bot clients to interact with the OneBot protocol.
#[cfg(feature = "api")]
#[async_trait]
pub trait ApiExt {
    /// The error type returned by API methods.
    type Error;

    /// Send a private message to a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The target user's ID
    /// * `message` - The message to send (can be anything implementing `IntoMessage`)
    /// * `auto_escape` - Whether to treat the message as plain text
    async fn send_private_message<M>(
        &self,
        user_id: i64,
        message: M,
        auto_escape: Option<bool>,
    ) -> Result<SendMessageResponse, Self::Error>
    where
        M: IntoMessage + Send;

    /// Send a message to a group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The target group's ID
    /// * `message` - The message to send (can be anything implementing `IntoMessage`)
    /// * `auto_escape` - Whether to treat the message as plain text
    async fn send_group_message<M>(
        &self,
        group_id: i64,
        message: M,
        auto_escape: Option<bool>,
    ) -> Result<SendMessageResponse, Self::Error>
    where
        M: IntoMessage + Send;

    /// Delete a message.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The ID of the message to delete
    async fn delete_message(&self, message_id: i64) -> Result<(), Self::Error>;

    /// Get a message by its ID.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The ID of the message to retrieve
    async fn get_message(&self, message_id: i64) -> Result<GetMessageResponse, Self::Error>;

    /// Get a forwarded message by its ID.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The ID of the forwarded message
    async fn get_forward_message(&self, message_id: i64)
        -> Result<GetForwardResponse, Self::Error>;

    /// Send a "like" to a user's profile.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The target user's ID
    /// * `times` - Number of likes to send (default: 1)
    async fn send_like(&self, user_id: i64, times: Option<i32>) -> Result<(), Self::Error>;

    /// Kick a user from a group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `user_id` - The user to kick
    /// * `reject_add_request` - Whether to reject future join requests from this user
    async fn set_group_kick(
        &self,
        group_id: i64,
        user_id: i64,
        reject_add_request: Option<bool>,
    ) -> Result<(), Self::Error>;

    /// Ban a user in a group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `user_id` - The user to ban
    /// * `duration` - Ban duration in seconds (0 = unban)
    async fn set_group_ban(
        &self,
        group_id: i64,
        user_id: i64,
        duration: Option<i64>,
    ) -> Result<(), Self::Error>;

    /// Ban an anonymous user in a group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `anonymous` - Anonymous user info
    /// * `flag` - Anonymous user flag
    /// * `duration` - Ban duration in seconds
    async fn set_group_anonymous_ban(
        &self,
        group_id: i64,
        anonymous: Option<GroupAnonymousInfo>,
        flag: Option<String>,
        duration: Option<i64>,
    ) -> Result<(), Self::Error>;

    /// Enable or disable group-wide ban.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `enable` - Whether to enable the ban
    async fn set_whole_group_ban(
        &self,
        group_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error>;

    /// Set a user as group admin or remove admin status.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `user_id` - The user ID
    /// * `enable` - Whether to make them an admin
    async fn set_group_admin(
        &self,
        group_id: i64,
        user_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error>;

    /// Enable or disable anonymous chat in a group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `enable` - Whether to enable anonymous chat
    async fn set_group_anonymous(
        &self,
        group_id: i64,
        enable: Option<bool>,
    ) -> Result<(), Self::Error>;

    /// Set a user's group card (nickname in group).
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `user_id` - The user ID
    /// * `card` - The new card name (None = clear)
    async fn set_group_card(
        &self,
        group_id: i64,
        user_id: i64,
        card: Option<String>,
    ) -> Result<(), Self::Error>;

    /// Set the group name.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `group_name` - The new group name
    async fn set_group_name(&self, group_id: i64, group_name: String) -> Result<(), Self::Error>;

    /// Leave a group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `is_dismiss` - Whether to dismiss the group (owner only)
    async fn set_group_leave(
        &self,
        group_id: i64,
        is_dismiss: Option<bool>,
    ) -> Result<(), Self::Error>;

    /// Set a user's special title in a group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `user_id` - The user ID
    /// * `special_title` - The special title (None = clear)
    /// * `duration` - Duration in seconds (-1 = permanent)
    async fn set_group_special_title(
        &self,
        group_id: i64,
        user_id: i64,
        special_title: Option<String>,
        duration: Option<i64>,
    ) -> Result<(), Self::Error>;

    /// Process a friend add request.
    ///
    /// # Arguments
    ///
    /// * `flag` - Request flag
    /// * `approve` - Whether to approve the request
    /// * `remark` - Remark name for the friend
    async fn set_friend_add_request(
        &self,
        flag: String,
        approve: Option<bool>,
        remark: Option<String>,
    ) -> Result<(), Self::Error>;

    /// Process a group add request.
    ///
    /// # Arguments
    ///
    /// * `flag` - Request flag
    /// * `sub_type` - Request subtype (add or invite)
    /// * `approve` - Whether to approve the request
    /// * `reason` - Rejection reason
    async fn set_group_add_request(
        &self,
        flag: String,
        sub_type: GroupRequestSubType,
        approve: Option<bool>,
        reason: Option<String>,
    ) -> Result<(), Self::Error>;

    /// Get bot's login information.
    async fn get_login_info(&self) -> Result<LoginInfo, Self::Error>;

    /// Get stranger information.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user's ID
    /// * `no_cache` - Whether to skip cache
    async fn get_stranger_info(
        &self,
        user_id: i64,
        no_cache: Option<bool>,
    ) -> Result<StrangerInfo, Self::Error>;

    /// Get the bot's friend list.
    async fn get_friend_list(&self) -> Result<Vec<FriendInfo>, Self::Error>;

    /// Get group information.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `no_cache` - Whether to skip cache
    async fn get_group_info(
        &self,
        group_id: i64,
        no_cache: Option<bool>,
    ) -> Result<GroupInfoResponse, Self::Error>;

    /// Get the bot's group list.
    async fn get_group_list(&self) -> Result<Vec<GroupInfoResponse>, Self::Error>;

    /// Get group member information.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `user_id` - The user ID
    /// * `no_cache` - Whether to skip cache
    async fn get_group_member_info(
        &self,
        group_id: i64,
        user_id: i64,
        no_cache: Option<bool>,
    ) -> Result<GroupInfoResponse, Self::Error>;

    /// Get group member list.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    async fn get_group_member_list(&self, group_id: i64) -> Result<Vec<FriendInfo>, Self::Error>;

    /// Get group honor information.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID
    /// * `ty` - The type of honor to retrieve
    async fn get_group_honor_info(
        &self,
        group_id: i64,
        ty: GroupHonorType,
    ) -> Result<GroupHonorInfo, Self::Error>;

    /// Get cookies.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain for cookies
    async fn get_cookies(&self, domain: Option<String>) -> Result<GetCookiesResponse, Self::Error>;

    /// Get CSRF token.
    async fn get_csrf_token(&self) -> Result<GetCsrfTokenResponse, Self::Error>;

    /// Get credentials (cookies + CSRF token).
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain for cookies
    async fn get_credentials(
        &self,
        domain: Option<String>,
    ) -> Result<GetCredentialsResponse, Self::Error>;

    /// Get a voice record file.
    ///
    /// # Arguments
    ///
    /// * `file` - File name
    /// * `out_format` - Output format
    async fn get_record(
        &self,
        file: String,
        out_format: RecordFormat,
    ) -> Result<GetFileResponse, Self::Error>;

    /// Get an image file.
    ///
    /// # Arguments
    ///
    /// * `file` - File name
    async fn get_image(&self, file: String) -> Result<GetFileResponse, Self::Error>;

    /// Check if the bot can send images.
    async fn can_send_image(&self) -> Result<CanSendResponse, Self::Error>;

    /// Check if the bot can send voice records.
    async fn can_send_record(&self) -> Result<CanSendResponse, Self::Error>;

    /// Get bot status.
    async fn get_status(&self) -> Result<BotStatus, Self::Error>;

    /// Get version information.
    async fn get_version_info(&self) -> Result<VersionInfo, Self::Error>;

    /// Restart the bot.
    ///
    /// # Arguments
    ///
    /// * `delay` - Delay in milliseconds before restarting
    async fn set_restart(&self, delay: Option<i32>) -> Result<(), Self::Error>;

    /// Clean cache.
    async fn clean_cache(&self) -> Result<(), Self::Error>;
}
