#[derive(Debug, Clone, Default)]
pub enum LayoutDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// This is the setting of the full text in a `box`, which is also known as `text window`.
#[derive(Debug, Clone, Default)]
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
    /// the width of viewport, which will affect the value of the vertex coordinates.
    pub viewport_width: f64,
    /// the height of viewport, which will affect the value of the vertex coordinates.
    pub viewport_height: f64,
}
