use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// This is the setting of the full text in a `box`, which is also known as `text window`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LayoutStyle {
    /// the writing direction of the text in the box,
    /// only `Horizontal` (right-to-left) or `Vertical` (top-to-bottom) is valid.
    pub direction: LayoutDirection,
    /// the width of box.
    pub box_width: f64,
    /// the height of box.
    pub box_height: f64,
    /// the size of the glyph grid which each character be fit to, usually equals to `font_size`.
    pub glyph_grid_size: f64,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            direction: Default::default(),
            box_width: 1280.,
            box_height: 720.,
            glyph_grid_size: 24.,
        }
    }
}
