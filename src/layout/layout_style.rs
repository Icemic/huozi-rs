use serde::{Deserialize, Serialize};

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
    /// The first-line indent in CJK character widths.
    pub indent: f64,
    /// 段落结束行的行内对齐方式。
    pub align: ParagraphAlignment,
    /// 源空白转换策略。
    pub white_space: WhiteSpaceMode,
    /// 软换行策略。
    pub text_wrap: TextWrapMode,
    /// Tab stop 的重复间距。
    pub tab_size: TabSize,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            box_width: None,
            box_height: None,
            line_height: 1.5,
            indent: 0.,
            align: ParagraphAlignment::Start,
            white_space: WhiteSpaceMode::default(),
            text_wrap: TextWrapMode::default(),
            tab_size: TabSize::default(),
        }
    }
}
