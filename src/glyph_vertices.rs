use crate::layout::Vertex;

/// Represents the vertices of a glyph, which consists of three layers: the shadow, the stroke, and the fill.\
/// It is recommanded to draw the layers in the order of shadow, stroke, and fill.
#[derive(Debug, Clone)]
pub struct GlyphVertices {
    /// The vertices of the shadow layer.
    pub shadow: Vec<Vertex>,
    /// The vertices of the stroke layer.
    pub stroke: Vec<Vertex>,
    /// The vertices of the fill layer.
    pub fill: Vec<Vertex>,
    /// order to draw the layers (CCW)
    pub indices: Vec<u16>,
    /// position on the direction of text flow
    pub col: u32,
    /// position on the direction perpendicular to the text flow
    pub row: u32,
    /// the x value of left-top corner of the bounding box
    pub x: u32,
    /// the y value of left-top corner of the bounding box
    pub y: u32,
    /// the width of the bounding box
    pub width: u32,
    /// the height of the bounding box
    pub height: u32,
    /// the scale ratio of the glyph
    pub scale_ratio: f32,
}
