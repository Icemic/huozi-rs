use fontdue::{Font, Metrics};
use image::{DynamicImage, Rgba};
use log::debug;

use crate::glyph::common::Glyph;

use super::common::{FontHMetrics, GlyphExtractor};

pub struct FontdueExtractor {
    font: Font,
    font_size: f32,
}

impl Into<Glyph> for Metrics {
    fn into(self) -> Glyph {
        Glyph {
            width: self.width as u32,
            height: self.height as u32,
            h_advance: self.advance_width.ceil() as u32,
            v_advance: self.advance_height.ceil() as u32,
            x_min: self.xmin,
            y_min: self.ymin,
            x_max: self.xmin + self.width as i32,
            y_max: self.ymin + self.height as i32,
        }
    }
}

impl GlyphExtractor for FontdueExtractor {
    fn new(font_data: Vec<u8>, font_size: f32) -> Self {
        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()).unwrap();

        Self { font, font_size }
    }
    fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
    }
    fn get_glyph(&self, ch: char) -> Glyph {
        let metrics = self.font.metrics(ch, self.font_size);
        metrics.into()
    }
    fn transform_to_glyph(&self, ch: char) -> Glyph {
        let metrics = self.font.metrics(ch, self.font_size);

        Glyph {
            width: metrics.width as u32,
            height: metrics.height as u32,
            h_advance: metrics.advance_width.ceil() as u32,
            v_advance: metrics.advance_height.ceil() as u32,
            x_min: metrics.xmin,
            y_min: metrics.ymin,
            x_max: metrics.xmin + metrics.width as i32,
            y_max: metrics.ymin + metrics.height as i32,
        }
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
    fn get_bitmap_and_metrics(&self, ch: char) -> (Vec<u8>, Glyph) {
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
        (bitmap, metrics.into())
    }
}
