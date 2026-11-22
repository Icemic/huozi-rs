use csscolorparser::Color;
use serde::{Deserialize, Serialize};

use crate::de_str::impl_from_str_via_serde;
use crate::layout::{
    FirstLineIndent, MeasureAdaptiveFirstLineIndent, ParagraphAlignment, TextWrapMode,
    WhiteSpaceMode, WritingMode,
};
use crate::parser::{ShadowStyle, SourceRange, StrokeStyle, TextRun, TextStyle};

pub use crate::layout::RubyLineHeightMode;

/// 解析后的富文本内容，按段落保留标签声明。
#[derive(Debug, Clone, Default)]
pub struct ParsedText {
    pub paragraphs: Vec<ParsedParagraph>,
}

/// 一个逻辑段落及其行内内容。
#[derive(Debug, Clone, Default)]
pub struct ParsedParagraph {
    pub nodes: Vec<InlineNode>,
    pub paragraph_style: ParagraphStyleOverride,
}

/// 后续段落相对前一段的样式覆盖。
#[derive(Debug, Clone, Default)]
pub struct ParagraphStyleOverride {
    pub indent: Option<FirstLineIndent>,
    pub first_line_indent_policy: Option<MeasureAdaptiveFirstLineIndent>,
    pub line_height: Option<f32>,
    pub block_indent: Option<f32>,
    pub last_line_alignment: Option<ParagraphAlignment>,
    pub writing_mode: Option<WritingMode>,
    pub line_length_grid: Option<bool>,
    /// 外层 `None` 表示未设置；`Some(None)` 表示重置为跟随段落末行对齐。
    pub line_length_grid_body_alignment: Option<Option<ParagraphAlignment>>,
    pub white_space: Option<WhiteSpaceMode>,
    pub text_wrap: Option<TextWrapMode>,
    pub tab_size: Option<f32>,
    pub ruby_line_height_mode: Option<RubyLineHeightMode>,
    pub inline_object_minimum_clearance: Option<f32>,
    pub emphasis_dot_gap: Option<f32>,
}

/// 按输入顺序排列的行内内容。
#[derive(Debug, Clone)]
pub enum InlineNode {
    Text(TextRun),
    Scope {
        kind: InlineScopeKind,
        children: Vec<InlineNode>,
    },
    Object(InlineObject),
}

/// 覆盖行内范围的非文字样式语义。
#[derive(Debug, Clone)]
pub enum InlineScopeKind {
    Background(BackgroundStyle),
    Underline(LineStyle),
    LineThrough(LineStyle),
    Ruby(RubyStyle),
    Decoration(DecorationKind),
    Link { id: Option<String>, target: String },
    Technical,
    InlineCode(InlineCodeStyle),
    AutoSpaceSuppressed,
    InlineBox(InlineBoxStyle),
}

/// 背景范围的绘制和行内几何参数。
#[derive(Debug, Clone)]
pub struct BackgroundStyle {
    pub fill_color: Color,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<ShadowStyle>,
    pub padding_x: f32,
    pub padding_y: f32,
    pub radius: f32,
    pub continuation_radius: Option<f32>,
    pub metric_policy: BackgroundMetricPolicy,
    pub clearance: f32,
}

impl Default for BackgroundStyle {
    fn default() -> Self {
        Self {
            fill_color: Color::new(0.0, 0.0, 0.0, 1.0),
            stroke: None,
            shadow: None,
            padding_x: 0.0,
            padding_y: 0.0,
            radius: 0.0,
            continuation_radius: None,
            metric_policy: BackgroundMetricPolicy::MarkedFaces,
            clearance: 0.0,
        }
    }
}

/// 背景高度的度量来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackgroundMetricPolicy {
    MarkedFaces,
    UniformTextStyle,
    UniformParagraphStyle,
}

impl_from_str_via_serde!(BackgroundMetricPolicy);

/// 下划线和删除线的绘制参数。
#[derive(Debug, Clone)]
pub struct LineStyle {
    pub fill_color: Color,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<ShadowStyle>,
    pub thickness: f32,
    pub pattern: LinePattern,
    pub clearance: f32,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            fill_color: Color::new(0.0, 0.0, 0.0, 1.0),
            stroke: None,
            shadow: None,
            thickness: 1.0,
            pattern: LinePattern::Solid,
            clearance: 0.0,
        }
    }
}

/// 线条的重复方式。
#[derive(Debug, Clone, PartialEq)]
pub enum LinePattern {
    Solid,
    Dashed { dash_length: f32, gap_length: f32 },
    Dotted { gap_length: f32 },
}

/// Ruby 或 bopomofo 注音内容。
#[derive(Debug, Clone)]
pub struct RubyStyle {
    pub text: String,
    pub font_families: Vec<String>,
    pub locale: Option<String>,
    pub kind: RubyKind,
}

/// 注音的排版类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RubyKind {
    Pinyin,
    Bopomofo,
}

/// CLREQ 装饰类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationKind {
    Emphasis,
    Mourning,
    ProperNoun,
    BookTitle,
}

/// 行内代码的文字和背景样式。
#[derive(Debug, Clone)]
pub struct InlineCodeStyle {
    pub text_style: TextStyle,
    pub background: BackgroundStyle,
}

/// 行内盒在文字两侧保留的空间。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InlineBoxStyle {
    pub start: f32,
    pub end: f32,
    pub spacing: InlineBoxSpacing,
}

/// 行内盒边缘的自动间距归属。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InlineBoxSpacing {
    Narrow,
    Source,
}

impl_from_str_via_serde!(InlineBoxSpacing);

/// 插入当前位置的行内对象。
#[derive(Debug, Clone)]
pub struct InlineObject {
    pub id: Option<String>,
    pub alt: String,
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub leading_boundary: InlineObjectBoundary,
    pub trailing_boundary: InlineObjectBoundary,
    pub source_range: SourceRange,
}

/// 行内对象边界的调整方式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InlineObjectBoundary {
    Fixed,
}
