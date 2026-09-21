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
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            box_width: None,
            box_height: None,
            line_height: 1.5,
            indent: 0.,
            align: ParagraphAlignment::Start,
        }
    }
}
