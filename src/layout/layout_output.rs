use tiqian::core::geometry::Rect;

use crate::glyph_vertices::GlyphVertices;

use super::SegmentGlyphSpan;

/// 富文本布局的公开输出。
#[derive(Debug, Clone)]
pub struct RichTextLayoutOutput {
    pub glyphs: Vec<GlyphVertices>,
    pub segment_glyph_spans: Vec<SegmentGlyphSpan>,
    pub interactions: Vec<Interaction>,
    pub width: u32,
    pub height: u32,
}

/// 调用方声明的交互元素及其可见区域。
#[derive(Debug, Clone, PartialEq)]
pub struct Interaction {
    pub id: String,
    pub areas: Vec<InteractionArea>,
}

/// 一个最终排版单元对应的交互区域。
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionArea {
    pub rect: Rect,
    pub row: u32,
    pub col: u32,
}
