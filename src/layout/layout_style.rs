use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// 段落结束行的行内对齐方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParagraphAlignment {
    Start,
    Center,
    End,
}

impl Default for ParagraphAlignment {
    fn default() -> Self {
        Self::Start
    }
}

/// 源空白转换策略，与自动间距独立配置。
///
/// 语义与 tiqian 的同名类型一致，由 Huozi 再次定义以满足配置序列化需要。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WhiteSpaceMode {
    /// 折叠连续普通空白与普通换行为一个空格，并移除行首尾空白。
    Collapse,
    /// 保留普通换行，其余普通空白按行首尾折叠。
    PreserveBreaks,
    /// 保留全部空白与换行，行尾空白可悬挂。
    Preserve,
    /// 保留全部空白与换行，每个空格与 Tab 后都可断行。
    BreakSpaces,
}

impl Default for WhiteSpaceMode {
    fn default() -> Self {
        Self::PreserveBreaks
    }
}

/// 软换行策略，保留的硬换行始终生效。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextWrapMode {
    Wrap,
    NoWrap,
}

impl Default for TextWrapMode {
    fn default() -> Self {
        Self::Wrap
    }
}

/// Tab stop 的重复间距。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TabSize {
    /// 段落默认字体空格 advance 的倍数。
    SpaceAdvanceMultiple(f64),
    /// 引擎布局单位。
    LayoutUnits(f64),
}

impl Default for TabSize {
    fn default() -> Self {
        Self::SpaceAdvanceMultiple(8.0)
    }
}

/// 书写方向。竖排尚未进入布局管线，取值会被原样传给 Tiqian，但当前不改变输出。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WritingMode {
    HorizontalTb,
    VerticalRl,
}

impl Default for WritingMode {
    fn default() -> Self {
        Self::HorizontalTb
    }
}

/// 拼音注音对行高的影响方式，语义与 Tiqian 同名类型一致。
///
/// `PerLine` 只在含注文的行加高，`UniformParagraph` 把同样的增量应用到整段每一行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RubyLineHeightMode {
    PerLine,
    UniformParagraph,
}

impl Default for RubyLineHeightMode {
    fn default() -> Self {
        Self::PerLine
    }
}

/// 行长字号整数倍量化（grid-first）。
///
/// `enabled` 打开时，可用行长向下取整到字号整数倍；剩余不足一字的余量按
/// `body_alignment` 摆放，`None` 表示跟随段落末行对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LineLengthGrid {
    pub enabled: bool,
    pub body_alignment: Option<ParagraphAlignment>,
}

impl Default for LineLengthGrid {
    fn default() -> Self {
        Self {
            enabled: true,
            body_alignment: None,
        }
    }
}

impl LineLengthGrid {
    pub fn new(enabled: bool, body_alignment: Option<ParagraphAlignment>) -> Self {
        Self {
            enabled,
            body_alignment,
        }
    }

    pub fn with_enabled(enabled: bool) -> Self {
        Self {
            enabled,
            body_alignment: None,
        }
    }
}

/// 段首缩进随行长自适应的策略，语义与 Tiqian 同名类型一致。
///
/// 窄行（`measure_em < short_below_em`）缩 `short_em` 字，宽行缩 `long_em` 字。
/// 仅当 `FirstLineIndent::Auto` 时生效。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MeasureAdaptiveFirstLineIndent {
    pub short_below_em: f64,
    pub short_em: f64,
    pub long_em: f64,
}

impl Default for MeasureAdaptiveFirstLineIndent {
    fn default() -> Self {
        Self {
            short_below_em: 14.0,
            short_em: 1.0,
            long_em: 2.0,
        }
    }
}

impl MeasureAdaptiveFirstLineIndent {
    pub fn new(short_below_em: f64, short_em: f64, long_em: f64) -> Self {
        Self {
            short_below_em,
            short_em,
            long_em,
        }
    }
}

/// 段首缩进的来源。
///
/// `Fixed` 使用固定字数并忽略 `MeasureAdaptiveFirstLineIndent`；`Auto` 交由策略按行长决定。
/// 序列化时 `Fixed` 写成数值，`Auto` 写成 `"auto"`，因此既有以数值配置缩进的调用方无需改动。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FirstLineIndent {
    Auto,
    Fixed(f64),
}

impl Default for FirstLineIndent {
    fn default() -> Self {
        Self::Fixed(0.0)
    }
}

impl FirstLineIndent {
    /// 返回固定缩进字数；`Auto` 没有固定值。
    pub fn fixed_count(self) -> Option<f64> {
        match self {
            Self::Auto => None,
            Self::Fixed(count) => Some(count),
        }
    }
}

impl Serialize for FirstLineIndent {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Auto => serializer.serialize_str("auto"),
            Self::Fixed(count) => serializer.serialize_f64(*count),
        }
    }
}

impl<'de> Deserialize<'de> for FirstLineIndent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FirstLineIndentVisitor;

        impl<'de> Visitor<'de> for FirstLineIndentVisitor {
            type Value = FirstLineIndent;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a number or \"auto\"")
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Ok(FirstLineIndent::Fixed(value))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(FirstLineIndent::Fixed(value as f64))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(FirstLineIndent::Fixed(value as f64))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match value {
                    "auto" => Ok(FirstLineIndent::Auto),
                    other => other
                        .parse::<f64>()
                        .map(FirstLineIndent::Fixed)
                        .map_err(|_| {
                            E::invalid_value(de::Unexpected::Str(other), &"a number or \"auto\"")
                        }),
                }
            }
        }

        deserializer.deserialize_any(FirstLineIndentVisitor)
    }
}

/// This is the setting of the full text in a `box`, which is also known as `text window`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LayoutStyle {
    /// The maximum width of the box, or `None` for no width limit.
    pub box_width: Option<f64>,
    /// The maximum height of the box, or `None` for no height limit.
    pub box_height: Option<f64>,
    /// The baseline-to-baseline line height as a multiplier of the paragraph base font size.
    pub line_height: f64,
    /// 首行缩进：固定字数或交由策略自适应，单位 `ic`。
    pub indent: FirstLineIndent,
    /// 首行缩进随行长自适应的策略；仅在 `indent` 为 `Auto` 时生效。
    pub first_line_indent_policy: MeasureAdaptiveFirstLineIndent,
    /// 所有行的起始缩进，单位 `ic`。
    pub block_indent: f64,
    /// 段落结束行的行内对齐方式。
    pub align: ParagraphAlignment,
    /// 书写方向。
    pub writing_mode: WritingMode,
    /// 源空白转换策略。
    pub white_space: WhiteSpaceMode,
    /// 软换行策略。
    pub text_wrap: TextWrapMode,
    /// Tab stop 的重复间距。
    pub tab_size: TabSize,
    /// 行长字号整数倍量化。
    pub line_length_grid: LineLengthGrid,
    /// 拼音注音对行高的影响方式。
    pub ruby_line_height_mode: RubyLineHeightMode,
    /// 行内对象与相邻行可见内容之间必须保留的最小净空，单位 em。
    pub inline_object_minimum_clearance: f64,
    /// 着重号圆点墨迹与被注文字字面底边的净空，单位 em。
    pub emphasis_dot_gap: f64,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            box_width: None,
            box_height: None,
            line_height: 1.5,
            indent: FirstLineIndent::default(),
            first_line_indent_policy: MeasureAdaptiveFirstLineIndent::default(),
            block_indent: 0.,
            align: ParagraphAlignment::Start,
            writing_mode: WritingMode::default(),
            white_space: WhiteSpaceMode::default(),
            text_wrap: TextWrapMode::default(),
            tab_size: TabSize::default(),
            line_length_grid: LineLengthGrid::default(),
            ruby_line_height_mode: RubyLineHeightMode::default(),
            inline_object_minimum_clearance: 0.1,
            emphasis_dot_gap: 0.1,
        }
    }
}
