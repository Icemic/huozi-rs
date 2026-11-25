use std::{ops::Range, sync::Arc};

use tiqian::core::geometry::Rect;

use crate::effects::Effect;
use crate::glyph_vertices::UnitVertices;

use super::SegmentGlyphSpan;

/// 富文本布局的公开输出。
#[derive(Debug, Clone)]
pub struct RichTextLayoutOutput {
    pub glyphs: Vec<UnitVertices>,
    /// 实际 shaping cluster 与其完整绘制元素范围，供逐字显示和动效消费。
    pub clusters: Vec<GlyphCluster>,
    /// 已送入排版的正文 scalar 总数；不含标签，包含对象占位，不重复计算注音。
    pub text_length: usize,
    pub segment_glyph_spans: Vec<SegmentGlyphSpan>,
    pub interactions: Vec<Interaction>,
    pub width: u32,
    pub height: u32,
}

/// 一个真实字形簇及其附属绘制元素。
#[derive(Debug, Clone)]
pub struct GlyphCluster {
    /// 文档连续显示文本的 scalar 半开范围；与含标签的原始来源范围不同。
    pub source: Range<usize>,
    /// `RichTextLayoutOutput::glyphs` 中的半开范围；空格等可没有绘制元素。
    pub glyph_range: Range<usize>,
    /// 按声明顺序组合的绘制效果；跨标签的连字采用首 scalar 的状态。
    pub effects: Arc<[Effect]>,
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
