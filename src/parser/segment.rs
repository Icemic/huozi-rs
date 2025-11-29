use std::borrow::Cow;

/// An identifier for a segment in the source content, which can be either a String or u32.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentId {
    Tag(String),
    Lite(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment<'s> {
    pub id: Option<SegmentId>,
    pub content: Cow<'s, str>,
}

impl<'s> Segment<'s> {
    pub const fn dummy(content: &'s str) -> Self {
        Self {
            id: None,
            content: Cow::Borrowed(content),
        }
    }
}
