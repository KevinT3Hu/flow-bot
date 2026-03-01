use crate::message::IntoMessage;
use crate::{
    event::message::Message,
    message,
    message::segments::{ReplySegment, Segment},
};

pub trait MessageExt {
    fn reply<T>(&self, message: T) -> message::Message
    where
        T: IntoMessage;
}

impl MessageExt for Message {
    fn reply<T>(&self, message: T) -> message::Message
    where
        T: IntoMessage,
    {
        let mut ret = vec![Segment::Reply(ReplySegment {
            id: self.message_id.to_string(),
        })];

        ret.extend(message.into_message());
        ret
    }
}
