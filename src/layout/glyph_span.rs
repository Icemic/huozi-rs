use crate::parser::SegmentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentGlyphSpan {
    pub segment_id: SegmentId,
    pub glyph_range: std::ops::Range<usize>, // [start, end)
}
