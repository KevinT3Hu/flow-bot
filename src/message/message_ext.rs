use super::{Message, segments::Segment};

pub trait MessageExt {
    fn extract_plain_text(&self) -> String;

    fn is_plain_text(&self) -> bool;

    fn extract_if_plain_text(&self) -> Option<String> {
        if self.is_plain_text() {
            Some(self.extract_plain_text())
        } else {
            None
        }
    }
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

    fn is_plain_text(&self) -> bool {
        self.iter()
            .all(|segment| matches!(segment, Segment::Text(_)))
    }
}
