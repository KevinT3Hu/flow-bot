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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecordSegment {
    pub file: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoSegment {
    pub file: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AtSegment {
    pub qq: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiceSegment;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShakeSegment;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PokeSegment {
    #[serde(rename = "type")]
    pub ty: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnonymousSegment;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShareSegment {
    pub url: String,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MusicSegment {
    #[serde(rename = "type")]
    pub ty: String,
    pub id: Option<String>,
    pub url: Option<String>,
    pub audio: Option<String>,
    pub title: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReplySegment {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForwardSegment {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeSegment {
    pub id: String,
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
