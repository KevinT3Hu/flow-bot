//! OneBot 11 message segments (spec: `message/segment.md`).
//!
//! Every segment is `{"type": <name>, "data": {...}}` on the wire. Segment
//! parameter values are strings except the boolean-ish flags (`magic`,
//! `cache`, `proxy`, `ignore`), which flow-bot sends as `0`/`1` integers and
//! accepts in any documented form (`0`/`1`, `yes`/`no`, `true`/`false`,
//! as bool, int, or string), and the numeric `timeout` (seconds).
//!
//! An unrecognized segment type deserializes into [`Segment::Unknown`]
//! instead of failing the whole message, so one exotic segment cannot
//! degrade an entire event to `TypedEvent::Unknown`.

use serde::{
    Deserialize, Serialize, Serializer,
    de::{self, IgnoredAny},
    ser::SerializeMap,
};

use super::Message;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TextSegment {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FaceSegment {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ImageSegment {
    pub file: String,
    /// Image type, `flash` for flash images (spec receive field).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    /// Image URL (spec receive-only field, dropped by implementations is fine).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Send-only: use the cached file when sending via URL (default on).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<Flag>,
    /// Send-only: download through a proxy when sending via URL (default on).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<Flag>,
    /// Send-only: download timeout in seconds (default: no timeout).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_timeout"
    )]
    pub timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RecordSegment {
    pub file: String,
    /// Voice transformation (变声), spec receive + send field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magic: Option<Flag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<Flag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<Flag>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_timeout"
    )]
    pub timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct VideoSegment {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<Flag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<Flag>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_timeout"
    )]
    pub timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AtSegment {
    pub qq: String,
}

/// A boolean-ish segment flag (`magic`, `cache`, `proxy`, `ignore`).
///
/// The spec allows `0`/`1` plus `no`/`yes` and `false`/`true` without pinning
/// the JSON type. flow-bot sends plain integers (`0`/`1`) and accepts every
/// documented form (bool, integer, string) when receiving.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flag(pub bool);

impl Serialize for Flag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(i32::from(self.0))
    }
}

impl<'de> Deserialize<'de> for Flag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::Bool(b) => Ok(Self(b)),
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(|i| Self(i != 0))
                .ok_or_else(|| de::Error::custom("flag is not an integer")),
            serde_json::Value::String(s) => match s.as_str() {
                "1" | "yes" | "true" => Ok(Self(true)),
                "0" | "no" | "false" => Ok(Self(false)),
                _ => Err(de::Error::custom(format!("invalid flag value: {s}"))),
            },
            other => Err(de::Error::custom(format!("invalid flag value: {other}"))),
        }
    }
}

/// Lenient `timeout` deserializer: seconds as a number or numeric string.
fn de_timeout<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    match Option::<serde_json::Value>::deserialize(deserializer)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| de::Error::custom("timeout is not an unsigned integer")),
        Some(serde_json::Value::String(s)) => s
            .parse::<u64>()
            .map(Some)
            .map_err(|_| de::Error::custom(format!("invalid timeout value: {s}"))),
        Some(other) => Err(de::Error::custom(format!("invalid timeout value: {other}"))),
    }
}

/// Defines a segment with no parameters (`dice`, `rps`, `shake`). The spec
/// sends `"data": {}` for these, but implementations may also emit `null` or
/// omit `data`, so deserialization tolerates any value and serialization
/// always emits `{}`.
macro_rules! empty_segment {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct $name;

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_map(Some(0))?.end()
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                IgnoredAny::deserialize(deserializer)?;
                Ok(Self)
            }
        }
    };
}

empty_segment!(
    #[doc = "Rock-paper-scissors magic emoji."]
    RpsSegment
);
empty_segment!(
    #[doc = "Dice magic emoji."]
    DiceSegment
);
empty_segment!(
    #[doc = "Window shake (a.k.a. basic poke), send-only."]
    ShakeSegment
);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PokeSegment {
    #[serde(rename = "type")]
    pub ty: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AnonymousSegment {
    /// Send-only: keep sending as non-anonymous when anonymity is unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Flag>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ShareSegment {
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContactType {
    QQ,
    Group,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ContactSegment {
    #[serde(rename = "type")]
    pub ty: ContactType,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LocationSegment {
    pub lat: String,
    pub lon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReplySegment {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ForwardSegment {
    pub id: String,
}

/// Forward node. The spec defines two forms: a send-only form carrying an
/// `id` referencing an existing message, and a custom form carrying
/// `user_id`/`nickname`/`content` (what `get_forward_msg` returns). `content`
/// accepts the CQ-code string form as well as the segment array form.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NodeSegment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Message>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct XmlSegment {
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JsonSegment {
    pub data: String,
}

/// An unrecognized segment: the original `type` string and raw `data`,
/// preserved for round-tripping and manual inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownSegment {
    pub ty: String,
    pub data: serde_json::Value,
}

/// Declares the segment enum from a single (variant, payload, wire-name)
/// table, generating the adjacently tagged `type`/`data` serde shape plus the
/// [`Segment::Unknown`] catch-all that keeps unknown types from failing the
/// surrounding message.
macro_rules! segment_enum {
    ($( $(#[$meta:meta])* $variant:ident ( $payload:ty ) = $name:literal; )*) => {
        #[derive(Debug, Clone, PartialEq)]
        pub enum Segment {
            $(
                $(#[$meta])*
                $variant($payload),
            )*
            /// Any segment type outside the OneBot 11 standard set.
            Unknown(UnknownSegment),
        }

        impl Serialize for Segment {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut map = serializer.serialize_map(Some(2))?;
                match self {
                    $(
                        Segment::$variant(payload) => {
                            map.serialize_entry("type", $name)?;
                            map.serialize_entry("data", payload)?;
                        }
                    )*
                    Segment::Unknown(unknown) => {
                        map.serialize_entry("type", &unknown.ty)?;
                        map.serialize_entry("data", &unknown.data)?;
                    }
                }
                map.end()
            }
        }

        impl<'de> Deserialize<'de> for Segment {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = serde_json::Value::deserialize(deserializer)?;
                let Some(ty) = value.get("type").and_then(|t| t.as_str()) else {
                    return Err(de::Error::custom("segment object missing string field `type`"));
                };
                let data = value.get("data").cloned().unwrap_or(serde_json::Value::Null);
                Ok(match ty {
                    $(
                        $name => Segment::$variant(
                            serde_json::from_value(data)
                                .map_err(|e| de::Error::custom(format!("malformed `{ty}` segment: {e}")))?,
                        ),
                    )*
                    _ => Segment::Unknown(UnknownSegment {
                        ty: ty.to_owned(),
                        data,
                    }),
                })
            }
        }
    };
}

segment_enum! {
    Text(TextSegment) = "text";
    Face(FaceSegment) = "face";
    Image(ImageSegment) = "image";
    Record(RecordSegment) = "record";
    Video(VideoSegment) = "video";
    At(AtSegment) = "at";
    Dice(DiceSegment) = "dice";
    Rps(RpsSegment) = "rps";
    Shake(ShakeSegment) = "shake";
    Poke(PokeSegment) = "poke";
    Anonymous(AnonymousSegment) = "anonymous";
    Share(ShareSegment) = "share";
    Contact(ContactSegment) = "contact";
    Location(LocationSegment) = "location";
    Music(MusicSegment) = "music";
    Reply(ReplySegment) = "reply";
    Forward(ForwardSegment) = "forward";
    Node(NodeSegment) = "node";
    Xml(XmlSegment) = "xml";
    Json(JsonSegment) = "json";
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn empty_segments_accept_any_data() {
        for data in [json!({}), json!(null)] {
            let seg: Segment =
                serde_json::from_value(json!({"type": "dice", "data": data})).unwrap();
            assert_eq!(seg, Segment::Dice(DiceSegment));
        }
        // `data` omitted entirely is tolerated too.
        let seg: Segment = serde_json::from_value(json!({"type": "rps"})).unwrap();
        assert_eq!(seg, Segment::Rps(RpsSegment));
        // Empty segments always serialize back to the spec's `{}` form.
        assert_eq!(
            serde_json::to_value(Segment::Shake(ShakeSegment)).unwrap(),
            json!({"type": "shake", "data": {}})
        );
    }

    #[test]
    fn unknown_segment_type_is_confined() {
        let raw = json!({"type": "minecraft", "data": {"block": "stone"}});
        let seg: Segment = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            seg,
            Segment::Unknown(UnknownSegment {
                ty: "minecraft".to_owned(),
                data: json!({"block": "stone"}),
            })
        );
        // Round-trips with the original type name.
        assert_eq!(serde_json::to_value(&seg).unwrap(), raw);
    }

    #[test]
    fn malformed_known_segment_still_errors() {
        // `face` is a known type missing its required `id`.
        serde_json::from_value::<Segment>(json!({"type": "face", "data": {}})).unwrap_err();
        // A known type with a wrongly-typed field errors as well.
        serde_json::from_value::<Segment>(json!({"type": "text", "data": {"text": 5}}))
            .unwrap_err();
    }

    #[test]
    fn flags_round_trip_as_integers() {
        let seg = Segment::Image(ImageSegment {
            file: "1.jpg".to_owned(),
            ty: None,
            url: None,
            cache: Some(Flag(false)),
            proxy: Some(Flag(true)),
            timeout: Some(30),
        });
        let wire = serde_json::to_value(&seg).unwrap();
        assert_eq!(
            wire["data"],
            json!({"file": "1.jpg", "cache": 0, "proxy": 1, "timeout": 30})
        );
        let back: Segment = serde_json::from_value(wire).unwrap();
        assert_eq!(back, seg);
    }

    #[test]
    fn flags_accept_every_documented_form() {
        for (raw, expected) in [
            (json!(true), true),
            (json!(false), false),
            (json!(1), true),
            (json!(0), false),
            (json!(2), true),
            (json!("1"), true),
            (json!("0"), false),
            (json!("yes"), true),
            (json!("no"), false),
            (json!("true"), true),
            (json!("false"), false),
        ] {
            let flag: Flag = serde_json::from_value(raw).unwrap();
            assert_eq!(flag, Flag(expected));
        }
        serde_json::from_value::<Flag>(json!("maybe")).unwrap_err();
    }

    #[test]
    fn timeout_accepts_number_or_numeric_string() {
        let seg: Segment = serde_json::from_value(
            json!({"type": "video", "data": {"file": "a.mp4", "timeout": "10"}}),
        )
        .unwrap();
        match seg {
            Segment::Video(v) => assert_eq!(v.timeout, Some(10)),
            other => panic!("wrong segment: {other:?}"),
        }
    }

    #[test]
    fn anonymous_carries_ignore_flag() {
        let seg: Segment =
            serde_json::from_value(json!({"type": "anonymous", "data": {"ignore": 1}})).unwrap();
        assert_eq!(
            seg,
            Segment::Anonymous(AnonymousSegment {
                ignore: Some(Flag(true))
            })
        );
        // Plain `{}` (the spec's example form) still deserializes.
        let seg: Segment =
            serde_json::from_value(json!({"type": "anonymous", "data": {}})).unwrap();
        assert_eq!(seg, Segment::Anonymous(AnonymousSegment { ignore: None }));
    }

    #[test]
    fn every_known_variant_round_trips() {
        let variants = [
            Segment::Text(TextSegment { text: "hi".into() }),
            Segment::Face(FaceSegment { id: "1".into() }),
            Segment::Image(ImageSegment {
                file: "a.jpg".into(),
                ty: Some("flash".into()),
                url: None,
                cache: None,
                proxy: None,
                timeout: None,
            }),
            Segment::Record(RecordSegment {
                file: "a.mp3".into(),
                magic: Some(Flag(true)),
                url: None,
                cache: None,
                proxy: None,
                timeout: None,
            }),
            Segment::Video(VideoSegment {
                file: "a.mp4".into(),
                url: None,
                cache: None,
                proxy: None,
                timeout: None,
            }),
            Segment::At(AtSegment { qq: "all".into() }),
            Segment::Dice(DiceSegment),
            Segment::Rps(RpsSegment),
            Segment::Shake(ShakeSegment),
            Segment::Poke(PokeSegment {
                ty: "126".into(),
                id: "2003".into(),
                name: None,
            }),
            Segment::Anonymous(AnonymousSegment { ignore: None }),
            Segment::Share(ShareSegment {
                url: "http://baidu.com".into(),
                title: "百度".into(),
                content: None,
                image: None,
            }),
            Segment::Contact(ContactSegment {
                ty: ContactType::QQ,
                id: "10001000".into(),
            }),
            Segment::Location(LocationSegment {
                lat: "39.8969426".into(),
                lon: "116.3109099".into(),
                title: None,
                content: None,
            }),
            Segment::Music(MusicSegment {
                ty: "163".into(),
                id: Some("28949129".into()),
                url: None,
                audio: None,
                title: None,
                content: None,
                image: None,
            }),
            Segment::Reply(ReplySegment { id: "123".into() }),
            Segment::Forward(ForwardSegment { id: "456".into() }),
            Segment::Node(NodeSegment {
                id: Some("789".into()),
                user_id: None,
                nickname: None,
                content: None,
            }),
            Segment::Xml(XmlSegment {
                data: "<?xml".into(),
            }),
            Segment::Json(JsonSegment {
                data: "{\"app\":1}".into(),
            }),
        ];
        for variant in variants {
            let wire = serde_json::to_value(&variant).unwrap();
            let back: Segment = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(back, variant, "round trip failed for {wire}");
        }
    }
}
