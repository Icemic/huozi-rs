#[derive(Debug, Clone, Default)]
pub struct GlyphMetrics {
    pub width: u32,
    pub height: u32,
    pub h_advance: f32,
    pub v_advance: f32,
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub x_scale: Option<f32>,
    pub y_scale: Option<f32>,
}