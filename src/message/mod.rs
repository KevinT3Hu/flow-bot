//! OneBot 11 messages in both wire formats.
//!
//! The spec allows `message`-typed fields as a CQ-code **string**, a segment
//! **array**, or a single segment **object**; [`Message`] deserializes from
//! all three (strings via [`cq::parse_cq`]) and always serializes to the
//! array form. [`Display`](std::fmt::Display) renders the CQ-code string
//! form.

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use serde::{
    Deserialize, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};

pub mod cq;
pub mod message_ext;
pub mod segments;

use segments::{Segment, TextSegment};

/// A message: an ordered sequence of message [`Segment`]s.
///
/// Behaves like a `[Segment]` slice through `Deref`, so iteration, indexing,
/// and the `MessageExt` helpers keep working; use [`Message::push`] and the
/// `From`/`FromIterator` impls to build one.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Message(Vec<Segment>);

impl Message {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, segment: Segment) {
        self.0.push(segment);
    }

    pub fn into_segments(self) -> Vec<Segment> {
        self.0
    }
}

impl Deref for Message {
    type Target = [Segment];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Message {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<Segment>> for Message {
    fn from(segments: Vec<Segment>) -> Self {
        Self(segments)
    }
}

impl From<Message> for Vec<Segment> {
    fn from(message: Message) -> Self {
        message.0
    }
}

impl From<Segment> for Message {
    fn from(segment: Segment) -> Self {
        Self(vec![segment])
    }
}

impl From<String> for Message {
    fn from(text: String) -> Self {
        Self::from(Segment::Text(TextSegment { text }))
    }
}

impl From<&str> for Message {
    fn from(text: &str) -> Self {
        Self::from(Segment::Text(TextSegment {
            text: text.to_owned(),
        }))
    }
}

impl FromIterator<Segment> for Message {
    fn from_iter<I: IntoIterator<Item = Segment>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Extend<Segment> for Message {
    fn extend<I: IntoIterator<Item = Segment>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl IntoIterator for Message {
    type Item = Segment;
    type IntoIter = std::vec::IntoIter<Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Message {
    type Item = &'a Segment;
    type IntoIter = std::slice::Iter<'a, Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&cq::to_cq_string(self))
    }
}

impl Serialize for Message {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MessageVisitor;

        impl<'de> Visitor<'de> for MessageVisitor {
            type Value = Message;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a CQ-code string, a segment array, or a segment object")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(cq::parse_cq(value))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut segments = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(segment) = seq.next_element()? {
                    segments.push(segment);
                }
                Ok(Message(segments))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut object = serde_json::Map::new();
                while let Some(key) = map.next_key()? {
                    object.insert(key, map.next_value()?);
                }
                let segment = Segment::deserialize(serde_json::Value::Object(object))
                    .map_err(de::Error::custom)?;
                Ok(Message(vec![segment]))
            }
        }

        deserializer.deserialize_any(MessageVisitor)
    }
}

pub trait IntoMessage {
    fn into_message(self) -> Message;
}

impl IntoMessage for String {
    fn into_message(self) -> Message {
        Message::from(Segment::Text(TextSegment { text: self }))
    }
}

impl IntoMessage for &str {
    fn into_message(self) -> Message {
        Message::from(Segment::Text(TextSegment {
            text: self.to_string(),
        }))
    }
}

impl IntoMessage for &String {
    fn into_message(self) -> Message {
        Message::from(Segment::Text(TextSegment { text: self.clone() }))
    }
}

impl IntoMessage for Message {
    fn into_message(self) -> Message {
        self
    }
}

impl IntoMessage for Segment {
    fn into_message(self) -> Message {
        Message::from(self)
    }
}

impl<T> IntoMessage for Vec<T>
where
    T: Into<Segment>,
{
    fn into_message(self) -> Message {
        self.into_iter().map(Into::into).collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn deserializes_from_all_three_forms() {
        let expected = Message::from(vec![Segment::Text(TextSegment {
            text: "hi".to_owned(),
        })]);

        let from_array: Message = serde_json::from_value(json!([
            {"type": "text", "data": {"text": "hi"}}
        ]))
        .unwrap();
        assert_eq!(from_array, expected);

        let from_string: Message = serde_json::from_value(json!("hi")).unwrap();
        assert_eq!(from_string, expected);

        let from_object: Message =
            serde_json::from_value(json!({"type": "text", "data": {"text": "hi"}})).unwrap();
        assert_eq!(from_object, expected);
    }

    #[test]
    fn serializes_to_array_form_and_displays_as_cq() {
        let message = cq::parse_cq("[CQ:at,qq=all] 你好");
        let wire = serde_json::to_value(&message).unwrap();
        assert_eq!(
            wire,
            json!([
                {"type": "at", "data": {"qq": "all"}},
                {"type": "text", "data": {"text": " 你好"}},
            ])
        );
        assert_eq!(message.to_string(), "[CQ:at,qq=all] 你好");
    }
}
