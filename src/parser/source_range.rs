use crate::parser::SegmentId;

/// A range in the source content, identified by a segment ID and start/end positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRange {
    pub segment_id: SegmentId,
    pub start: usize,
    pub end: usize,
}
