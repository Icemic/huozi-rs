use crate::parser::SegmentId;

/// An offset measured in Unicode scalar values from the start of a segment's source content.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScalarOffset(pub usize);

/// A range in the source content, identified by a segment ID and start/end positions.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SourceRange {
    pub segment_id: Option<SegmentId>,
    pub start: ScalarOffset,
    pub end: ScalarOffset,
}
