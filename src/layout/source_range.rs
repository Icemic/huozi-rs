/// An identifier for a segment in the source content, which can be either a String or u32.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentId {
    Tag(String),
    Lite(u32),
}

/// A range in the source content, identified by a segment ID and start/end positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRange {
    pub segment_id: SegmentId,
    pub start: usize,
    pub end: usize,
}
