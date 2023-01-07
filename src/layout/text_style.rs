use super::Color;

#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    // pub font_face: Font
    pub font_size: f64,
    pub fill_color: Color,
    pub line_height: f64,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<ShadowStyle>,
}

#[derive(Debug, Clone, Default)]
pub struct StrokeStyle {
    pub stroke_color: Color,
    pub stroke_width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct ShadowStyle {
    pub shadow_width: f32,
    pub shadow_blur: f32,
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub shadow_color: f32,
}
