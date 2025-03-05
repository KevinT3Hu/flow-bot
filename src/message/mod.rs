use segments::TextSegment;

pub mod message_ext;
pub mod segments;

pub type Message = Vec<segments::Segment>;

pub trait IntoMessage {
    fn into_message(self) -> Message;
}

impl<T> IntoMessage for T
where
    T: AsRef<str>,
{
    fn into_message(self) -> Message {
        vec![segments::Segment::Text(TextSegment {
            text: self.as_ref().to_string(),
        })]
    }
}
