use std::sync::Arc;

use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::font::Font;
use font_kit::hinting::HintingOptions;
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2I;

use super::common::{FontHMetrics, GlyphExtractorTrait, GlyphMetrics};

pub struct GlyphExtractor {
    font: Font,
    font_size: f32,
}

impl Into<GlyphMetrics> for RectI {
    fn into(self) -> GlyphMetrics {
        GlyphMetrics {
            width: self.width() as u32,
            height: self.height() as u32,
            h_advance: 0.,
            v_advance: 0.,
            x_min: self.min_x() as f32,
            y_min: -self.max_y() as f32,
            x_max: self.max_x() as f32,
            y_max: -self.min_y() as f32,
            // scale glyph size once it larger than 1em, it will affect width or height.
            // For example, `width / x_scale` should be the actual size if `x_scale` is not None.
            x_scale: None,
            y_scale: None,
        }
    }
}

impl GlyphExtractorTrait for GlyphExtractor {
    fn new(font_data: Vec<u8>, font_size: f32) -> Self {
        let font = font_kit::handle::Handle::from_memory(Arc::new(font_data), 0)
            .load()
            .unwrap();

        Self { font, font_size }
    }
    fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
    }
    fn get_glyph_metrics(&self, _ch: char) -> GlyphMetrics {
        todo!()
    }
    fn font_metrics(&self) -> FontHMetrics {
        let h_metrics = self.font.metrics();

        FontHMetrics {
            ascent: h_metrics.ascent.ceil() as i32,
            descent: h_metrics.descent.floor() as i32,
            line_gap: h_metrics.line_gap.ceil() as i32,
            line_height: (h_metrics.ascent - h_metrics.descent + h_metrics.line_gap).ceil() as i32,
            content_height: (h_metrics.ascent - h_metrics.descent).ceil() as i32,
        }
    }
    fn get_bitmap_and_metrics(&self, ch: char) -> (Vec<u8>, GlyphMetrics) {
        // let mut canvas = self.canvas.lock().unwrap();
        let mut canvas = Canvas::new(
            Vector2I::new(self.font_size as i32, self.font_size as i32),
            Format::A8,
        );
        if let Some(glyph_id) = self.font.glyph_for_char(ch) {
            let transform = Transform2F::default();
            let hinting_options = HintingOptions::None;
            let rasterization_options = RasterizationOptions::GrayscaleAa;

            let mut rect = self
                .font
                .raster_bounds(
                    glyph_id,
                    self.font_size,
                    transform,
                    hinting_options,
                    rasterization_options,
                )
                .unwrap();

            let mut metrics = if rect.width() as f32 > self.font_size {
                let x_scale = self.font_size / rect.width() as f32;
                let y_scale = x_scale;

                rect = self
                    .font
                    .raster_bounds(
                        glyph_id,
                        self.font_size * x_scale,
                        transform,
                        hinting_options,
                        rasterization_options,
                    )
                    .unwrap();

                let mut metrics: GlyphMetrics = rect.into();

                metrics.x_scale = Some(x_scale);
                metrics.y_scale = Some(y_scale);

                metrics
            } else if rect.height() as f32 > self.font_size {
                let y_scale = self.font_size / rect.height() as f32;
                let x_scale = y_scale;

                rect = self
                    .font
                    .raster_bounds(
                        glyph_id,
                        self.font_size * x_scale,
                        transform,
                        hinting_options,
                        rasterization_options,
                    )
                    .unwrap();

                let mut metrics: GlyphMetrics = rect.into();

                metrics.x_scale = Some(x_scale);
                metrics.y_scale = Some(y_scale);

                metrics
            } else {
                let metrics: GlyphMetrics = rect.into();
                metrics
            };

            let advance = self.font.advance(glyph_id).unwrap();

            metrics.h_advance =
                advance.x() / self.font.metrics().units_per_em as f32 * self.font_size;
            metrics.v_advance =
                advance.y() / self.font.metrics().units_per_em as f32 * self.font_size;

            let _ = self.font.rasterize_glyph(
                &mut canvas,
                glyph_id,
                self.font_size,
                Transform2F::from_translation(-rect.origin().to_f32()),
                hinting_options,
                rasterization_options,
            );

            let mut bitmap = vec![0; metrics.width as usize * metrics.height as usize];

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    bitmap[y as usize * metrics.width as usize + x as usize] =
                        canvas.pixels[y as usize * self.font_size as usize + x as usize];
                }
            }

            (bitmap, metrics.into())
        } else {
            (vec![], GlyphMetrics::default())
        }
    }

    fn exist(&self, ch: char) -> bool {
        self.font.glyph_for_char(ch).is_some()
    }
}
