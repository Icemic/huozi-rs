use fontdue::{Font, Metrics};

use super::common::{FontHMetrics, GlyphExtractorTrait, GlyphMetrics};

pub struct GlyphExtractor {
    font: Font,
    font_size: f32,
}

impl Into<GlyphMetrics> for Metrics {
    fn into(self) -> GlyphMetrics {
        GlyphMetrics {
            width: self.width as u32,
            height: self.height as u32,
            h_advance: self.advance_width,
            v_advance: self.advance_height,
            x_min: self.xmin as f32,
            y_min: self.ymin as f32,
            x_max: self.xmin as f32 + self.width as f32,
            y_max: self.ymin as f32 + self.height as f32,
            /// scale glyph size once it larger than 1em, it will affect width or height.
            /// For example, `width / x_scale` should be the actual size if `x_scale` is not None.
            x_scale: None,
            y_scale: None,
        }
    }
}

impl GlyphExtractorTrait for GlyphExtractor {
    fn new(font_data: Vec<u8>, font_size: f32) -> Self {
        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()).unwrap();

        Self { font, font_size }
    }
    fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
    }
    fn get_glyph_metrics(&self, ch: char) -> GlyphMetrics {
        let metrics = self.font.metrics(ch, self.font_size);
        metrics.into()
    }
    fn font_metrics(&self) -> FontHMetrics {
        let h_metrics = self
            .font
            .horizontal_line_metrics(self.font_size)
            .expect("Cannot get font metrics.");

        FontHMetrics {
            ascent: h_metrics.ascent.ceil() as i32,
            descent: h_metrics.descent.floor() as i32,
            line_gap: h_metrics.line_gap.ceil() as i32,
            line_height: h_metrics.new_line_size.ceil() as i32,
            content_height: (h_metrics.ascent - h_metrics.descent).ceil() as i32,
        }
    }
    fn get_bitmap_and_metrics(&self, ch: char) -> (Vec<u8>, GlyphMetrics) {
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);

        if metrics.width as f32 > self.font_size {
            let x_scale = self.font_size / metrics.width as f32;
            let y_scale = x_scale;
            let (new_metrics, bitmap) = self.font.rasterize(ch, self.font_size * x_scale);
            let mut new_metrics: GlyphMetrics = new_metrics.into();
            let mut metrics: GlyphMetrics = metrics.into();

            metrics.width = new_metrics.width;
            metrics.height = new_metrics.height;
            metrics.x_scale = Some(x_scale);
            metrics.y_scale = Some(y_scale);

            return (bitmap, metrics);
        }

        (bitmap, metrics.into())
    }
}
