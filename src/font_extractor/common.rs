use std::default;

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

#[derive(Debug, Clone)]
pub struct FontHMetrics {
    pub ascent: i32,
    pub descent: i32,
    pub line_gap: i32,
    pub line_height: i32,
    pub content_height: i32,
}

pub trait GlyphExtractorTrait {
    fn new(font_data: Vec<u8>, font_size: f32) -> Self;

    fn set_font_size(&mut self, font_size: f32);

    fn exist(&self, ch: char) -> bool;

    fn get_glyph_metrics(&self, ch: char) -> GlyphMetrics;

    fn font_metrics(&self) -> FontHMetrics;

    fn get_bitmap_and_metrics(&self, ch: char) -> (Vec<u8>, GlyphMetrics);
}
