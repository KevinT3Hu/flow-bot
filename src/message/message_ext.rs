use super::{Message, segments::Segment};

pub trait MessageExt {
    fn extract_plain_text(&self) -> String;
}

impl MessageExt for Message {
    fn extract_plain_text(&self) -> String {
        self.iter()
            .filter_map(|segment| match segment {
                Segment::Text(text) => Some(text.text.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
            .join("")
    }
}
