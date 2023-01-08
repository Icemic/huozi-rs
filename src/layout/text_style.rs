use super::Color;

#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    // pub font_face: Font
    pub font_size: f64,
    pub fill_color: Color,
    pub line_height: f64,
    pub indent: f64,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<ShadowStyle>,
}

#[derive(Debug, Clone)]
pub struct StrokeStyle {
    pub stroke_color: Color,
    pub stroke_width: f32,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            stroke_color: Color::new(0., 0., 0., 0.),
            stroke_width: 0.,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ShadowStyle {
    pub shadow_width: f32,
    pub shadow_blur: f32,
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub shadow_color: f32,
}
