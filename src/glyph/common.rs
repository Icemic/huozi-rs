pub struct GlyphMetrics {
    pub width: u32,
    pub height: u32,
    pub h_advance: u32,
    pub v_advance: u32,
    pub x_min: i32,
    pub y_min: i32,
    pub x_max: i32,
    pub y_max: i32,
}

#[derive(Debug, Clone)]
pub struct FontHMetrics {
    pub ascent: i32,
    pub descent: i32,
    pub line_gap: i32,
    pub line_height: i32,
    pub content_height: i32,
}

pub trait GlyphExtractor {
    fn new(font_data: Vec<u8>, font_size: f32) -> Self;

    fn set_font_size(&mut self, font_size: f32);

    fn get_glyph(&self, ch: char) -> GlyphMetrics;

    fn transform_to_glyph(&self, ch: char) -> GlyphMetrics;

    fn font_metrics(&self) -> FontHMetrics;

    fn get_bitmap_and_metrics(&self, ch: char) -> (Vec<u8>, GlyphMetrics);
}
