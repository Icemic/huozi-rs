use crate::parser::{ScalarOffset, Segment, SegmentId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Element {
    Text {
        start: ScalarOffset,
        end: ScalarOffset,
        content: String,
        segment_id: Option<SegmentId>,
    },
    Block {
        start: ScalarOffset,
        end: ScalarOffset,
        inner: Vec<Element>,
        tag: String,
        attributes: Vec<Attribute>,
        segment_id: Option<SegmentId>,
    },
    SelfClosing {
        start: ScalarOffset,
        end: ScalarOffset,
        tag: String,
        attributes: Vec<Attribute>,
        segment_id: Option<SegmentId>,
    },
}

/// 标签头中按原始书写顺序保存的属性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
    pub value_start: ScalarOffset,
    pub value_end: ScalarOffset,
    pub segment_id: Option<SegmentId>,
}

struct TagHead {
    start: usize,
    tag: String,
    attributes: Vec<Attribute>,
    end: usize,
    self_closing: bool,
}

enum TagHeadResult {
    Valid(TagHead),
    Malformed { end: usize },
    NotTag,
}

struct Frame {
    start: usize,
    open_end: usize,
    tag: String,
    attributes: Vec<Attribute>,
    nodes: Vec<Element>,
    text: String,
    text_start: Option<usize>,
}

impl Frame {
    fn root() -> Self {
        Self {
            start: 0,
            open_end: 0,
            tag: String::new(),
            attributes: Vec::new(),
            nodes: Vec::new(),
            text: String::new(),
            text_start: None,
        }
    }

    fn tag(head: TagHead) -> Self {
        Self {
            start: head.start,
            open_end: head.end,
            tag: head.tag,
            attributes: head.attributes,
            nodes: Vec::new(),
            text: String::new(),
            text_start: None,
        }
    }

    fn append_char(&mut self, start: usize, value: char) {
        self.text_start.get_or_insert(start);
        self.text.push(value);
    }

    fn append_raw(&mut self, start: usize, raw: &[char]) {
        self.text_start.get_or_insert(start);
        self.text.extend(raw);
    }

    fn flush_text(&mut self, end: usize, segment_id: &Option<SegmentId>) {
        if let Some(start) = self.text_start.take()
            && !self.text.is_empty()
        {
            self.nodes.push(Element::Text {
                start: ScalarOffset(start),
                end: ScalarOffset(end),
                content: std::mem::take(&mut self.text),
                segment_id: segment_id.clone(),
            });
        }
    }
}

fn is_space(value: char) -> bool {
    matches!(value, ' ' | '\t' | '\r' | '\n')
}

fn skip_space(chars: &[char], mut index: usize) -> usize {
    while chars.get(index).is_some_and(|value| is_space(*value)) {
        index += 1;
    }
    index
}

fn is_token_char<const OPEN: char, const CLOSE: char>(value: char) -> bool {
    !is_space(value) && value != OPEN && value != CLOSE && !matches!(value, '=' | '/' | '\'' | '\"')
}

fn parse_token<const OPEN: char, const CLOSE: char>(
    chars: &[char],
    index: usize,
) -> Option<(String, usize)> {
    let end = chars[index..]
        .iter()
        .position(|value| !is_token_char::<OPEN, CLOSE>(*value))
        .map_or(chars.len(), |offset| index + offset);
    (end > index).then(|| (chars[index..end].iter().collect(), end))
}

fn primary_attribute(tag: &str) -> &str {
    match tag {
        "ruby" | "bopomofo" => "text",
        "link" => "target",
        "background" | "underline" | "lineThrough" | "color" | "fillColor" => "color",
        "font" => "family",
        "weight" => "weight",
        "locale" => "locale",
        "baseline" => "baseline",
        "attach" => "attach",
        "size" => "size",
        "stroke" => "stroke",
        "strokeColor" => "strokeColor",
        "strokeWidth" => "width",
        "shadow" => "shadow",
        "shadowOffsetX" => "offsetX",
        "shadowOffsetY" => "offsetY",
        "shadowBlur" => "blur",
        "shadowWidth" => "width",
        "shadowColor" => "shadowColor",
        _ => tag,
    }
}

fn parse_value<const OPEN: char, const CLOSE: char>(
    chars: &[char],
    mut index: usize,
) -> Option<(String, usize, usize, usize)> {
    index = skip_space(chars, index);
    let start = index;
    let quote = *chars.get(index)?;
    if matches!(quote, '\'' | '\"') {
        index += 1;
        let value_start = index;
        let mut value = String::new();
        while let Some(current) = chars.get(index) {
            if *current == quote {
                return Some((value, value_start, index, index + 1));
            }
            if *current == '\\' {
                let escaped = *chars.get(index + 1)?;
                if escaped == quote || escaped == '\\' {
                    value.push(escaped);
                    index += 2;
                    continue;
                }
                return None;
            }
            if matches!(current, '\r' | '\n') {
                return None;
            }
            value.push(*current);
            index += 1;
        }
        return None;
    }

    let (value, end) = parse_token::<OPEN, CLOSE>(chars, index)?;
    Some((value, start, end, end))
}

fn malformed_tag_end<const OPEN: char, const CLOSE: char>(chars: &[char], start: usize) -> usize {
    chars[start + 1..]
        .iter()
        .position(|value| *value == CLOSE || *value == OPEN)
        .map_or(chars.len(), |offset| {
            let boundary = start + offset + 1;
            if chars[boundary] == CLOSE {
                boundary + 1
            } else {
                boundary
            }
        })
}

fn parse_tag_head<const OPEN: char, const CLOSE: char>(
    chars: &[char],
    start: usize,
    segment_id: &Option<SegmentId>,
) -> TagHeadResult {
    if chars.get(start) != Some(&OPEN) || chars.get(start + 1) == Some(&'/') {
        return TagHeadResult::NotTag;
    }
    let mut index = skip_space(chars, start + 1);
    let Some((tag, tag_end)) = parse_token::<OPEN, CLOSE>(chars, index) else {
        return TagHeadResult::Malformed {
            end: malformed_tag_end::<OPEN, CLOSE>(chars, start),
        };
    };
    index = tag_end;
    let mut attributes = Vec::new();

    loop {
        index = skip_space(chars, index);
        match chars.get(index) {
            Some(value) if *value == CLOSE => {
                return TagHeadResult::Valid(TagHead {
                    start,
                    tag,
                    attributes,
                    end: index + 1,
                    self_closing: false,
                });
            }
            Some('/') => {
                let end = skip_space(chars, index + 1);
                if chars.get(end) == Some(&CLOSE) {
                    return TagHeadResult::Valid(TagHead {
                        start,
                        tag,
                        attributes,
                        end: end + 1,
                        self_closing: true,
                    });
                }
                return TagHeadResult::Malformed {
                    end: malformed_tag_end::<OPEN, CLOSE>(chars, start),
                };
            }
            Some('=') if attributes.is_empty() => {
                let Some((value, value_start, value_end, next)) =
                    parse_value::<OPEN, CLOSE>(chars, index + 1)
                else {
                    return TagHeadResult::Malformed { end: chars.len() };
                };
                attributes.push(Attribute {
                    name: primary_attribute(&tag).to_string(),
                    value,
                    value_start: ScalarOffset(value_start),
                    value_end: ScalarOffset(value_end),
                    segment_id: segment_id.clone(),
                });
                index = next;
            }
            Some(_) => {
                let Some((name, name_end)) = parse_token::<OPEN, CLOSE>(chars, index) else {
                    return TagHeadResult::Malformed {
                        end: malformed_tag_end::<OPEN, CLOSE>(chars, start),
                    };
                };
                let equals = skip_space(chars, name_end);
                if chars.get(equals) != Some(&'=') {
                    return TagHeadResult::Malformed {
                        end: malformed_tag_end::<OPEN, CLOSE>(chars, start),
                    };
                }
                let Some((value, value_start, value_end, next)) =
                    parse_value::<OPEN, CLOSE>(chars, equals + 1)
                else {
                    return TagHeadResult::Malformed { end: chars.len() };
                };
                attributes.push(Attribute {
                    name,
                    value,
                    value_start: ScalarOffset(value_start),
                    value_end: ScalarOffset(value_end),
                    segment_id: segment_id.clone(),
                });
                index = next;
            }
            None => return TagHeadResult::Malformed { end: chars.len() },
        }
    }
}

fn parse_end_tag<const OPEN: char, const CLOSE: char>(
    chars: &[char],
    start: usize,
) -> Option<(String, usize)> {
    if chars.get(start) != Some(&OPEN) || chars.get(start + 1) != Some(&'/') {
        return None;
    }
    let index = skip_space(chars, start + 2);
    let (tag, end) = parse_token::<OPEN, CLOSE>(chars, index)?;
    let end = skip_space(chars, end);
    (chars.get(end) == Some(&CLOSE)).then_some((tag, end + 1))
}

fn parse_elements<const OPEN: char, const CLOSE: char>(
    chars: &[char],
    segment_id: &Option<SegmentId>,
) -> Vec<Element> {
    let mut frames = vec![Frame::root()];
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == OPEN && chars.get(index + 1) == Some(&OPEN) {
            frames
                .last_mut()
                .expect("root frame is always present")
                .append_char(index, OPEN);
            index += 2;
            continue;
        }
        if chars[index] == CLOSE && chars.get(index + 1) == Some(&CLOSE) {
            frames
                .last_mut()
                .expect("root frame is always present")
                .append_char(index, CLOSE);
            index += 2;
            continue;
        }
        if chars[index] == OPEN {
            if let Some((tag, end)) = parse_end_tag::<OPEN, CLOSE>(chars, index) {
                let closes_current_frame =
                    frames.len() > 1 && frames.last().is_some_and(|frame| frame.tag == tag);
                let current = frames.last_mut().expect("root frame is always present");
                if closes_current_frame {
                    current.flush_text(index, segment_id);
                    let frame = frames.pop().expect("non-root frame was checked");
                    frames
                        .last_mut()
                        .expect("root frame is always present")
                        .nodes
                        .push(Element::Block {
                            start: ScalarOffset(frame.start),
                            end: ScalarOffset(end),
                            inner: frame.nodes,
                            tag: frame.tag,
                            attributes: frame.attributes,
                            segment_id: segment_id.clone(),
                        });
                } else {
                    current.append_raw(index, &chars[index..end]);
                }
                index = end;
                continue;
            }
            match parse_tag_head::<OPEN, CLOSE>(chars, index, segment_id) {
                TagHeadResult::Valid(head) => {
                    let head_end = head.end;
                    if head.self_closing {
                        let current = frames.last_mut().expect("root frame is always present");
                        current.flush_text(index, segment_id);
                        current.nodes.push(Element::SelfClosing {
                            start: ScalarOffset(head.start),
                            end: ScalarOffset(head.end),
                            tag: head.tag,
                            attributes: head.attributes,
                            segment_id: segment_id.clone(),
                        });
                    } else {
                        frames
                            .last_mut()
                            .expect("root frame is always present")
                            .flush_text(index, segment_id);
                        frames.push(Frame::tag(head));
                    }
                    index = head_end;
                    continue;
                }
                TagHeadResult::Malformed { end } => {
                    frames
                        .last_mut()
                        .expect("root frame is always present")
                        .append_raw(index, &chars[index..end]);
                    index = end;
                    continue;
                }
                TagHeadResult::NotTag => {}
            }
        }

        frames
            .last_mut()
            .expect("root frame is always present")
            .append_char(index, chars[index]);
        index += 1;
    }

    while frames.len() > 1 {
        let mut frame = frames.pop().expect("non-root frame was checked");
        frame.flush_text(chars.len(), segment_id);
        let parent = frames.last_mut().expect("root frame is always present");
        let raw_open = chars[frame.start..frame.open_end]
            .iter()
            .collect::<String>();
        if let Some(Element::Text {
            start,
            end,
            content,
            ..
        }) = frame.nodes.first_mut()
            && *start == ScalarOffset(frame.open_end)
        {
            content.insert_str(0, &raw_open);
            *start = ScalarOffset(frame.start);
            *end = ScalarOffset((*end).0);
        } else {
            parent.append_raw(frame.start, &chars[frame.start..frame.open_end]);
            parent.flush_text(frame.open_end, segment_id);
        }
        parent.nodes.extend(frame.nodes);
    }

    let mut root = frames.pop().expect("root frame is always present");
    root.flush_text(chars.len(), segment_id);
    root.nodes
}

/// Parse input with custom tag symbols.
///
/// # Type Parameters
/// * `OPEN` - The opening tag character (e.g., '[', '<', '{')
/// * `CLOSE` - The closing tag character (e.g., ']', '>', '}')
///
/// # Convention
/// Only one symbol combination should be used throughout the program's lifetime.
/// Mixing different symbol combinations in the same program run may produce incorrect results.
///
/// # Examples
/// ```ignore
/// // Use square brackets
/// let result = parse_with::<'[', ']'>("text [bold]content[/bold]");
///
/// // Use angle brackets
/// let result = parse_with::<'<', '>'>("text <bold>content</bold>");
///
/// // Use curly braces
/// let result = parse_with::<'{', '}'>("text {bold}content{/bold}");
/// ```
pub fn parse_with<const OPEN: char, const CLOSE: char>(
    input: &Segment<'_>,
) -> Result<Vec<Element>, String> {
    let chars = input.content.chars().collect::<Vec<_>>();
    Ok(parse_elements::<OPEN, CLOSE>(&chars, &input.id))
}

/// Parse input with default square bracket tags `[]`.
///
/// This is equivalent to calling `parse_with::<'[', ']'>(input)`.
pub fn parse(input: &Segment<'_>) -> Result<Vec<Element>, String> {
    parse_with::<'[', ']'>(input)
}
