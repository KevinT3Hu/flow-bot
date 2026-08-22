use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextSegment {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FaceSegment {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageSegment {
    pub file: String,
    /// Image type, `flash` for flash images (spec receive field).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    /// Image URL (spec receive-only field, dropped by implementations is fine).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecordSegment {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoSegment {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AtSegment {
    pub qq: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiceSegment;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpsSegment;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShakeSegment;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PokeSegment {
    #[serde(rename = "type")]
    pub ty: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnonymousSegment;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShareSegment {
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ContactType {
    QQ,
    Group,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContactSegment {
    #[serde(rename = "type")]
    pub ty: ContactType,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocationSegment {
    pub lat: String,
    pub lon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MusicSegment {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReplySegment {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForwardSegment {
    pub id: String,
}

/// Content of a forward node: either a message segment array (the array form
/// used by `get_forward_msg` responses) or a raw string (the CQ-code form).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum NodeContent {
    Segments(Vec<Segment>),
    Text(String),
}

/// Forward node. The spec defines two forms: a send-only form carrying an
/// `id` referencing an existing message, and a custom form carrying
/// `user_id`/`nickname`/`content` (what `get_forward_msg` returns).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeSegment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<NodeContent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XmlSegment {
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonSegment {
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Segment {
    Text(TextSegment),
    Face(FaceSegment),
    Image(ImageSegment),
    Record(RecordSegment),
    Video(VideoSegment),
    At(AtSegment),
    Dice(DiceSegment),
    Rps(RpsSegment),
    Shake(ShakeSegment),
    Poke(PokeSegment),
    Anonymous(AnonymousSegment),
    Share(ShareSegment),
    Contact(ContactSegment),
    Location(LocationSegment),
    Music(MusicSegment),
    Reply(ReplySegment),
    Forward(ForwardSegment),
    Node(NodeSegment),
    Xml(XmlSegment),
    Json(JsonSegment),
}
