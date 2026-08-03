use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// Optional adjustments for full-width CJK punctuation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PunctuationStyle {
    /// Compress the space between adjacent adjustable punctuation marks.
    pub compression: bool,
    /// Allow a single punctuation mark to hang at the end of a line.
    pub hanging: bool,
    /// The maximum amount of overflow allowed for a hanging punctuation mark, in em.
    pub hanging_tolerance: f64,
}

impl Default for PunctuationStyle {
    fn default() -> Self {
        Self {
            compression: true,
            hanging: true,
            hanging_tolerance: 0.5,
        }
    }
}

/// This is the setting of the full text in a `box`, which is also known as `text window`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LayoutStyle {
    /// the writing direction of the text in the box,
    /// only `Horizontal` (right-to-left) or `Vertical` (top-to-bottom) is valid.
    pub direction: LayoutDirection,
    /// The maximum width of the box, or `None` for no width limit.
    pub box_width: Option<f64>,
    /// The maximum height of the box, or `None` for no height limit.
    pub box_height: Option<f64>,
    /// the size of the glyph grid which each character be fit to, usually equals to `font_size`.
    pub glyph_grid_size: f64,
    /// optional adjustments for full-width CJK punctuation.
    pub punctuation: PunctuationStyle,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            direction: Default::default(),
            box_width: None,
            box_height: None,
            glyph_grid_size: 24.,
            punctuation: Default::default(),
        }
    }
}
