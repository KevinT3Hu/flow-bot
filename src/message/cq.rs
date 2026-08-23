//! The CQ-code string message format (spec: `message/string.md`).
//!
//! Codes look like `[CQ:function,key=value,...]`. Plain text between codes is
//! itself the message; the three characters `&`, `[`, `]` are escaped as
//! `&amp;`, `&#91;`, `&#93;` in text, and those plus `,` (as `&#44;`) in
//! parameter values. Unescaping is a single left-to-right pass, so
//! `&amp;#91;` decodes to the literal `&#91;`.

use serde_json::{Map, Value, json};

use super::{Message, segments::Segment, segments::TextSegment, segments::UnknownSegment};

const CODE_START: &str = "[CQ:";

/// Parse a CQ-code string into segments. Malformed input is treated as
/// literal text rather than an error: an unterminated `[CQ:` contributes no
/// code, and a known function name with bad parameters degrades to
/// [`Segment::Unknown`] instead of failing the whole message.
pub fn parse_cq(s: &str) -> Message {
    let mut segments: Vec<Segment> = Vec::new();
    let mut text = String::new();
    let mut rest = s;

    while let Some(start) = rest.find(CODE_START) {
        text.push_str(&rest[..start]);
        let body_start = start + CODE_START.len();
        let body_and_tail = &rest[body_start..];
        let Some(end) = body_and_tail.find(']') else {
            // Unterminated code: the remainder is literal text (param values
            // can never contain a raw `]`, so none is coming).
            text.push_str(&rest[start..]);
            rest = "";
            break;
        };
        flush_text(&mut segments, &mut text);
        segments.push(parse_code(&body_and_tail[..end]));
        rest = &body_and_tail[end + 1..];
    }
    text.push_str(rest);
    flush_text(&mut segments, &mut text);
    Message::from(segments)
}

/// Encode segments as a CQ-code string (the inverse of [`parse_cq`]).
pub fn to_cq_string(message: &Message) -> String {
    let mut out = String::new();
    for segment in message.iter() {
        match segment {
            Segment::Text(text) => out.push_str(&escape_text(&text.text)),
            other => {
                let value = serde_json::to_value(other).expect("segments always serialize");
                let object = value.as_object().expect("segments serialize to objects");
                out.push_str(CODE_START);
                out.push_str(
                    object
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
                if let Some(data) = object.get("data").and_then(Value::as_object) {
                    for (key, value) in data {
                        out.push(',');
                        out.push_str(key);
                        out.push('=');
                        out.push_str(&escape_param(&param_value(value)));
                    }
                }
                out.push(']');
            }
        }
    }
    out
}

fn flush_text(segments: &mut Vec<Segment>, text: &mut String) {
    if !text.is_empty() {
        segments.push(Segment::Text(TextSegment {
            text: unescape(&std::mem::take(text), false),
        }));
    }
}

/// Build one segment from a code body (everything between `[CQ:` and `]`).
/// The function name runs to the first `,` or the end; each remaining part
/// splits at its first `=` (later `=`s belong to the value).
fn parse_code(body: &str) -> Segment {
    let (function, params) = match body.find(',') {
        Some(i) => (&body[..i], &body[i + 1..]),
        None => (body, ""),
    };
    let mut data = Map::new();
    for part in params.split(',') {
        if part.is_empty() {
            continue;
        }
        let (key, value) = match part.find('=') {
            Some(i) => (&part[..i], &part[i + 1..]),
            None => (part, ""),
        };
        data.insert(key.to_owned(), Value::String(unescape(value, true)));
    }
    let raw = json!({"type": function, "data": Value::Object(data.clone())});
    match serde_json::from_value(raw) {
        Ok(segment) => segment,
        // A known function name with unusable parameters (e.g. `[CQ:face]`
        // without an `id`): keep what we got instead of failing.
        Err(_) => Segment::Unknown(UnknownSegment {
            ty: function.to_owned(),
            data: Value::Object(data),
        }),
    }
}

/// Flag values serialize as `0`/`1` integers; numbers and strings are used
/// verbatim; anything else (nested arrays/objects) falls back to JSON text.
fn param_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => if *b { "1" } else { "0" }.to_owned(),
        other => other.to_string(),
    }
}

/// Unescape `&amp;`/`&#91;`/`&#93;`, plus `&#44;` when inside a parameter
/// value (commas are legal in plain text, so `&#44;` there stays literal).
fn unescape(s: &str, param: bool) -> String {
    const ENTITIES: [(&str, char); 4] = [
        ("&amp;", '&'),
        ("&#91;", '['),
        ("&#93;", ']'),
        ("&#44;", ','),
    ];
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while !rest.is_empty() {
        let Some(amp) = rest.find('&') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let mut matched = false;
        for (entity, ch) in ENTITIES {
            if !param && entity == "&#44;" {
                continue;
            }
            if let Some(stripped) = tail.strip_prefix(entity) {
                out.push(ch);
                rest = stripped;
                matched = true;
                break;
            }
        }
        if !matched {
            out.push('&');
            rest = &tail[1..];
        }
    }
    out
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
}

fn escape_param(s: &str) -> String {
    escape_text(s).replace(',', "&#44;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::segments::{
        AnonymousSegment, AtSegment, FaceSegment, Flag, ImageSegment, NodeSegment, RecordSegment,
        ShareSegment,
    };

    fn assert_round_trip(cq: &str, message: &Message) {
        assert_eq!(parse_cq(cq), *message, "parse of {cq:?}");
        assert_eq!(to_cq_string(message), cq, "encode of {message:?}");
    }

    #[test]
    fn plain_text_round_trip() {
        assert_round_trip(
            "- &#91;x&#93; 使用 &amp;data",
            &Message::from(vec![Segment::Text(TextSegment {
                text: "- [x] 使用 &data".to_owned(),
            })]),
        );
        assert_round_trip("", &Message::new());
    }

    #[test]
    fn spec_share_example_with_escaped_comma() {
        assert_round_trip(
            "[CQ:share,title=震惊&#44;小伙竟然...,url=http://baidu.com/?a=1&amp;b=2]",
            &Message::from(vec![Segment::Share(ShareSegment {
                title: "震惊,小伙竟然...".to_owned(),
                url: "http://baidu.com/?a=1&b=2".to_owned(),
                content: None,
                image: None,
            })]),
        );
    }

    #[test]
    fn at_all_and_numbers() {
        assert_round_trip(
            "[CQ:at,qq=all]",
            &Message::from(vec![Segment::At(AtSegment {
                qq: "all".to_owned(),
            })]),
        );
        assert_round_trip(
            "[CQ:at,qq=10001000]",
            &Message::from(vec![Segment::At(AtSegment {
                qq: "10001000".to_owned(),
            })]),
        );
    }

    #[test]
    fn mixed_text_and_codes() {
        assert_round_trip(
            "看这个 [CQ:image,file=1.jpg] 和 [CQ:face,id=123] 表情",
            &Message::from(vec![
                Segment::Text(TextSegment {
                    text: "看这个 ".to_owned(),
                }),
                Segment::Image(ImageSegment {
                    file: "1.jpg".to_owned(),
                    ty: None,
                    url: None,
                    cache: None,
                    proxy: None,
                    timeout: None,
                }),
                Segment::Text(TextSegment {
                    text: " 和 ".to_owned(),
                }),
                Segment::Face(FaceSegment {
                    id: "123".to_owned(),
                }),
                Segment::Text(TextSegment {
                    text: " 表情".to_owned(),
                }),
            ]),
        );
    }

    #[test]
    fn first_equals_splits_param() {
        let msg = parse_cq("[CQ:share,title=标题中有=等号,url=http://baidu.com]");
        match msg.first() {
            Some(Segment::Share(share)) => assert_eq!(share.title, "标题中有=等号"),
            other => panic!("wrong segment: {other:?}"),
        }
    }

    #[test]
    fn unknown_function_is_preserved() {
        assert_round_trip(
            "[CQ:magic_spam,key=值]",
            &Message::from(vec![Segment::Unknown(UnknownSegment {
                ty: "magic_spam".to_owned(),
                data: json!({"key": "值"}),
            })]),
        );
    }

    #[test]
    fn known_function_with_bad_params_degrades_to_unknown() {
        assert_eq!(
            parse_cq("[CQ:face]"),
            Message::from(vec![Segment::Unknown(UnknownSegment {
                ty: "face".to_owned(),
                data: json!({}),
            })]),
        );
    }

    #[test]
    fn unterminated_code_is_text() {
        assert_eq!(
            parse_cq("hello [CQ:face,id=1"),
            Message::from(vec![Segment::Text(TextSegment {
                text: "hello [CQ:face,id=1".to_owned(),
            })]),
        );
    }

    #[test]
    fn non_cq_brackets_are_text() {
        assert_eq!(
            parse_cq("[普通] 文本"),
            Message::from(vec![Segment::Text(TextSegment {
                text: "[普通] 文本".to_owned(),
            })]),
        );
    }

    #[test]
    fn send_flags_encode_as_01() {
        // Parameter order in the string form follows the (alphabetical) map
        // iteration and is semantically irrelevant.
        assert_round_trip(
            "[CQ:image,cache=0,file=a.jpg,proxy=1,timeout=30]",
            &Message::from(vec![Segment::Image(ImageSegment {
                file: "a.jpg".to_owned(),
                ty: None,
                url: None,
                cache: Some(Flag(false)),
                proxy: Some(Flag(true)),
                timeout: Some(30),
            })]),
        );
        assert_round_trip(
            "[CQ:record,file=a.mp3,magic=1]",
            &Message::from(vec![Segment::Record(RecordSegment {
                file: "a.mp3".to_owned(),
                magic: Some(Flag(true)),
                url: None,
                cache: None,
                proxy: None,
                timeout: None,
            })]),
        );
    }

    #[test]
    fn anonymous_ignores_flag() {
        assert_round_trip(
            "[CQ:anonymous,ignore=1]",
            &Message::from(vec![Segment::Anonymous(AnonymousSegment {
                ignore: Some(Flag(true)),
            })]),
        );
        assert_round_trip(
            "[CQ:anonymous]",
            &Message::from(vec![Segment::Anonymous(AnonymousSegment { ignore: None })]),
        );
    }

    #[test]
    fn spec_node_example_parses_nested_content() {
        // The CQ-string content parameter decodes into segments via the
        // dual-form Message deserializer.
        let msg = parse_cq(
            "[CQ:node,user_id=10001000,nickname=某人,content=&#91;CQ:face&#44;id=123&#93;哈喽～]",
        );
        match msg.first() {
            Some(Segment::Node(node)) => {
                assert_eq!(node.user_id.as_deref(), Some("10001000"));
                assert_eq!(node.nickname.as_deref(), Some("某人"));
                assert_eq!(
                    node.content,
                    Some(Message::from(vec![
                        Segment::Face(FaceSegment {
                            id: "123".to_owned()
                        }),
                        Segment::Text(TextSegment {
                            text: "哈喽～".to_owned()
                        }),
                    ]))
                );
            }
            other => panic!("wrong segment: {other:?}"),
        }
    }

    #[test]
    fn node_with_id_form_round_trips() {
        assert_round_trip(
            "[CQ:node,id=123456]",
            &Message::from(vec![Segment::Node(NodeSegment {
                id: Some("123456".to_owned()),
                user_id: None,
                nickname: None,
                content: None,
            })]),
        );
    }

    #[test]
    fn param_without_equals_has_empty_value() {
        assert_eq!(
            parse_cq("[CQ:weird,bare]"),
            Message::from(vec![Segment::Unknown(UnknownSegment {
                ty: "weird".to_owned(),
                data: json!({"bare": ""}),
            })]),
        );
    }

    #[test]
    fn double_escaping_is_not_unescaped_twice() {
        assert_eq!(
            parse_cq("a&amp;#91;b"),
            Message::from(vec![Segment::Text(TextSegment {
                text: "a&#91;b".to_owned(),
            })]),
        );
        // Re-escaping yields the original form, i.e. the round trip is stable.
        assert_eq!(to_cq_string(&parse_cq("a&amp;#91;b")), "a&amp;#91;b");
    }
}
