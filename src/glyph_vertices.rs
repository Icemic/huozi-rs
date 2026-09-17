use crate::layout::Vertex;

/// Represents the vertices of a glyph, which consists of three layers: the shadow, the stroke, and the fill.\
/// It is recommanded to draw the layers in the order of shadow, stroke, and fill.
#[derive(Debug, Clone)]
pub struct GlyphVertices {
    /// The vertices of the shadow layer.
    pub shadow: Option<[Vertex; 4]>,
    /// The vertices of the stroke layer.
    pub stroke: Option<[Vertex; 4]>,
    /// The vertices of the fill layer.
    pub fill: [Vertex; 4],
    /// order to draw the layers (CCW)
    pub indices: [u16; 6],
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
