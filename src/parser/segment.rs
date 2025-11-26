/// An identifier for a segment in the source content, which can be either a String or u32.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentId {
    Tag(String),
    Lite(u32),
}

pub struct Segment<'s> {
    pub id: SegmentId,
    pub content: &'s str,
}
