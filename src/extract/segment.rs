use async_trait::async_trait;

use crate::{
    api::api_ext::ApiExt,
    base::context::BotContext,
    event::{BotEvent, TypedEvent},
    extract::FromEvent,
    message::{self, message_ext::MessageExt, segments::*},
};

/// Extract the text of the first text segment.
pub struct Text(pub String);

#[async_trait]
impl FromEvent for Text {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Text(t) => Some(Self(t.text.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract all text segments concatenated together.
pub struct PlainText(pub String);

#[async_trait]
impl FromEvent for PlainText {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            Some(Self(msg.message.extract_plain_text()))
        } else {
            None
        }
    }
}

/// Extract the first at segment's qq.
pub struct At(pub String);

#[async_trait]
impl FromEvent for At {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::At(at) => Some(Self(at.qq.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first face segment's id.
pub struct Face(pub String);

#[async_trait]
impl FromEvent for Face {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Face(f) => Some(Self(f.id.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first image segment's file.
pub struct Image(pub String);

#[async_trait]
impl FromEvent for Image {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Image(i) => Some(Self(i.file.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first record segment's file.
pub struct Record(pub String);

#[async_trait]
impl FromEvent for Record {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Record(r) => Some(Self(r.file.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first video segment's file.
pub struct Video(pub String);

#[async_trait]
impl FromEvent for Video {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Video(v) => Some(Self(v.file.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract if a dice segment is present.
pub struct Dice;

#[async_trait]
impl FromEvent for Dice {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Dice(_) => Some(Self),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract if a rock-paper-scissors segment is present.
pub struct Rps;

#[async_trait]
impl FromEvent for Rps {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Rps(_) => Some(Self),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract if a shake segment is present.
pub struct Shake;

#[async_trait]
impl FromEvent for Shake {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Shake(_) => Some(Self),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first poke segment.
pub struct Poke(pub PokeSegment);

#[async_trait]
impl FromEvent for Poke {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Poke(p) => Some(Self(p.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first share segment.
pub struct Share(pub ShareSegment);

#[async_trait]
impl FromEvent for Share {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Share(s) => Some(Self(s.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first contact segment.
pub struct Contact(pub ContactSegment);

#[async_trait]
impl FromEvent for Contact {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Contact(c) => Some(Self(c.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first location segment.
pub struct Location(pub LocationSegment);

#[async_trait]
impl FromEvent for Location {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Location(l) => Some(Self(l.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first music segment.
pub struct Music(pub MusicSegment);

#[async_trait]
impl FromEvent for Music {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Music(m) => Some(Self(m.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the replied message by looking up the reply segment via the API.
pub struct Reply(pub message::Message);

#[async_trait]
impl FromEvent for Reply {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self>
    where
        Self: Sized,
    {
        if let TypedEvent::Message(ref msg) = event.event {
            for segment in msg.message.iter() {
                if let Segment::Reply(reply) = segment {
                    let id = reply.id.parse::<i64>().ok()?;
                    let message = context
                        .get_message(id)
                        .await
                        .map(|msg| msg.message.clone())
                        .ok()?;
                    return Some(Self(message));
                }
            }
        }
        None
    }
}

#[async_trait]
impl FromEvent for ReplySegment {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Reply(r) => Some(r.clone()),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first forward segment's id.
pub struct Forward(pub String);

#[async_trait]
impl FromEvent for Forward {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Forward(f) => Some(Self(f.id.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first node segment's id, if the node references an existing
/// message (custom nodes carry `user_id`/`nickname`/`content` instead).
pub struct Node(pub String);

#[async_trait]
impl FromEvent for Node {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Node(n) => n.id.clone().map(Self),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first xml segment's data.
pub struct Xml(pub String);

#[async_trait]
impl FromEvent for Xml {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Xml(x) => Some(Self(x.data.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}

/// Extract the first json segment's data.
pub struct Json(pub String);

#[async_trait]
impl FromEvent for Json {
    async fn from_event(_: BotContext, event: BotEvent) -> Option<Self> {
        if let TypedEvent::Message(ref msg) = event.event {
            msg.message.iter().find_map(|seg| match seg {
                Segment::Json(j) => Some(Self(j.data.clone())),
                _ => None,
            })
        } else {
            None
        }
    }
}
