use crate::Color;

pub struct TextStyle {
    // pub font_face
    pub font_size: u32,
    pub fill_color: Color,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<ShadowStyle>,
}

pub struct StrokeStyle {
    pub stroke_color: Color,
    pub stroke_width: f32,
}

pub struct ShadowStyle {
    pub shadow_width: f32,
    pub shadow_blur: f32,
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub shadow_color: f32,
}

pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

pub struct LayoutStyle {
    pub direction: LayoutDirection,
    pub box_width: u32,
    pub box_height: u32,
}
