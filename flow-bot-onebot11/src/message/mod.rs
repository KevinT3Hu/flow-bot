use segments::TextSegment;

pub mod message_ext;
pub mod segments;

pub type Message = Vec<segments::Segment>;

pub trait IntoMessage {
    fn into_message(self) -> Message;
}

impl IntoMessage for String {
    fn into_message(self) -> Message {
        vec![segments::Segment::Text(TextSegment { text: self })]
    }
}

impl IntoMessage for &str {
    fn into_message(self) -> Message {
        vec![segments::Segment::Text(TextSegment {
            text: self.to_string(),
        })]
    }
}

impl IntoMessage for &String {
    fn into_message(self) -> Message {
        vec![segments::Segment::Text(TextSegment { text: self.clone() })]
    }
}

impl<T> IntoMessage for Vec<T>
where
    T: Into<segments::Segment>,
{
    fn into_message(self) -> Message {
        self.into_iter().map(|s| s.into()).collect()
    }
}
