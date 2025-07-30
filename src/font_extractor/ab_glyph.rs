use ab_glyph::{Font, FontVec, PxScale, ScaleFont};

use super::common::{FontHMetrics, GlyphExtractorTrait, GlyphMetrics};

pub struct GlyphExtractor {
    font: FontVec,
    scale: PxScale,
    font_size: f32,
}

impl GlyphExtractor {
    fn get_glyph_metrics_inner(&self, ch: char, scale: Option<PxScale>) -> GlyphMetrics {
        let font = self.font.as_scaled(scale.unwrap_or(self.scale));
        let glyph = font.glyph_id(ch).with_scale(scale.unwrap_or(self.scale));
        let h_advance = font.h_advance(glyph.id);
        let v_advance = font.v_advance(glyph.id);

        let (width, height, x_min, y_min, x_max, y_max) = match font.outline_glyph(glyph) {
            Some(q) => {
                let r = q.px_bounds();

                (
                    r.width() as u32,
                    r.height() as u32,
                    r.min.x,
                    // y coordiante is reversed, i don't know why...
                    -r.max.y,
                    r.max.x,
                    -r.min.y,
                )
            }
            None => (0, 0, 0., 0., 0., 0.),
        };

        GlyphMetrics {
            width,
            height,
            h_advance,
            v_advance,
            x_min,
            y_min,
            x_max,
            y_max,
            x_scale: None,
            y_scale: None,
        }
    }
}

impl GlyphExtractorTrait for GlyphExtractor {
    fn new(font_data: Vec<u8>, font_size: f32) -> Self {
        let font = FontVec::try_from_vec(font_data).unwrap();
        let scale =
            PxScale::from(font_size * font.height_unscaled() / font.units_per_em().unwrap());
        Self {
            font,
            scale,
            font_size,
        }
    }
    fn set_font_size(&mut self, font_size: f32) {
        let scale = PxScale::from(
            font_size * self.font.height_unscaled() / self.font.units_per_em().unwrap(),
        );
        self.scale = scale;
        self.font_size = font_size;
    }
    fn get_glyph_metrics(&self, ch: char) -> GlyphMetrics {
        self.get_glyph_metrics_inner(ch, None)
    }

    fn font_metrics(&self) -> FontHMetrics {
        let font = self.font.as_scaled(self.scale);
        FontHMetrics {
            ascent: font.ascent().ceil() as i32,
            descent: font.descent().ceil() as i32,
            line_gap: font.line_gap().ceil() as i32,
            line_height: (font.ascent() - font.descent() + font.line_gap()).ceil() as i32,
            content_height: (font.ascent() - font.descent()).ceil() as i32,
        }
    }

    fn get_bitmap_and_metrics(&self, ch: char) -> (Vec<u8>, GlyphMetrics) {
        let mut metrics = self.get_glyph_metrics(ch);

        let glyph = {
            if metrics.width as f32 > self.font_size {
                let mut scale = self.scale.clone();

                // it means bitmap is scaled by `x_scale` to fit max width (=font_size).
                let x_scale = self.font_size / metrics.width as f32;
                scale.x = scale.x * x_scale;
                scale.y = scale.y * x_scale;

                let new_metrics = self.get_glyph_metrics_inner(ch, Some(scale));
                metrics.width = new_metrics.width;
                metrics.height = new_metrics.height;
                metrics.x_scale = Some(x_scale);
                metrics.y_scale = Some(x_scale);
                self.font.glyph_id(ch).with_scale(scale)
            } else {
                self.font.glyph_id(ch).with_scale(self.scale)
            }
        };

        let capacity = (metrics.width * metrics.height) as usize;
        let mut bitmap = vec![0u8; capacity];

        if let Some(q) = self.font.outline_glyph(glyph) {
            let width = metrics.width as usize;
            q.draw(|x, y: u32, c| {
                let index = y as usize * width + x as usize;
                bitmap[index] = (c * 255.0 + 0.5) as u8;
            });
        };

        (bitmap, metrics)
    }

    fn exist(&self, ch: char) -> bool {
        self.font.glyph_id(ch).0 != 0
    }
}
