use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct FriendRequest {
    pub user_id: i64,
    pub comment: String,
    pub flag: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GroupRequestSubType {
    Add,
    Invite,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GroupRequest {
    pub user_id: i64,
    pub sub_type: GroupRequestSubType,
    pub group_id: i64,
    pub comment: String,
    pub flag: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "request_type")]
#[serde(rename_all = "snake_case")]
pub enum Request {
    Friend(FriendRequest),
    Group(GroupRequest),
}
