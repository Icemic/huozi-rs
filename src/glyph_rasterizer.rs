use ab_glyph_rasterizer::{Rasterizer, point};
use skrifa::outline::{DrawError, OutlinePen};
use tiqian::core::font_face::FontFaceId;

use crate::font_backend::HuoziFontManager;

pub(crate) struct GlyphBitmap {
    pub(crate) alpha: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) x_min: f32,
    pub(crate) y_min: f32,
    pub(crate) x_max: f32,
    pub(crate) y_max: f32,
}

pub(crate) fn rasterize_outline(
    font_manager: &HuoziFontManager,
    face: &FontFaceId,
    glyph_id: u32,
    font_size: f32,
) -> Result<Option<GlyphBitmap>, DrawError> {
    let mut pen = RasterizingPen::default();
    if !font_manager.draw_outline(face, glyph_id, font_size, &mut pen)? {
        return Ok(None);
    }
    Ok(Some(pen.finish()))
}

#[derive(Default)]
struct RasterizingPen {
    commands: Vec<PathCommand>,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    has_point: bool,
}

enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    Close,
}

impl RasterizingPen {
    fn record_point(&mut self, x: f32, y: f32) {
        if self.has_point {
            self.left = self.left.min(x);
            self.top = self.top.max(y);
            self.right = self.right.max(x);
            self.bottom = self.bottom.min(y);
        } else {
            self.left = x;
            self.top = y;
            self.right = x;
            self.bottom = y;
            self.has_point = true;
        }
    }

    fn finish(self) -> GlyphBitmap {
        if !self.has_point {
            return GlyphBitmap {
                alpha: Vec::new(),
                width: 0,
                height: 0,
                x_min: 0.0,
                y_min: 0.0,
                x_max: 0.0,
                y_max: 0.0,
            };
        }

        let left = self.left.floor();
        let top = self.top.ceil();
        let width = (self.right.ceil() - left).max(1.0) as usize;
        let height = (top - self.bottom.floor()).max(1.0) as usize;
        let mut rasterizer = Rasterizer::new(width, height);
        let mut current = None;
        let mut contour_start = None;

        for command in self.commands {
            match command {
                PathCommand::MoveTo(x, y) => {
                    current = Some((x, y));
                    contour_start = current;
                }
                PathCommand::LineTo(x, y) => {
                    if let Some((from_x, from_y)) = current {
                        rasterizer.draw_line(
                            point(from_x - left, top - from_y),
                            point(x - left, top - y),
                        );
                    }
                    current = Some((x, y));
                }
                PathCommand::QuadTo(control_x, control_y, x, y) => {
                    if let Some((from_x, from_y)) = current {
                        rasterizer.draw_quad(
                            point(from_x - left, top - from_y),
                            point(control_x - left, top - control_y),
                            point(x - left, top - y),
                        );
                    }
                    current = Some((x, y));
                }
                PathCommand::CurveTo(control_x0, control_y0, control_x1, control_y1, x, y) => {
                    if let Some((from_x, from_y)) = current {
                        rasterizer.draw_cubic(
                            point(from_x - left, top - from_y),
                            point(control_x0 - left, top - control_y0),
                            point(control_x1 - left, top - control_y1),
                            point(x - left, top - y),
                        );
                    }
                    current = Some((x, y));
                }
                PathCommand::Close => {
                    if let (Some((from_x, from_y)), Some((start_x, start_y))) =
                        (current, contour_start)
                    {
                        rasterizer.draw_line(
                            point(from_x - left, top - from_y),
                            point(start_x - left, top - start_y),
                        );
                        current = contour_start;
                    }
                }
            }
        }

        let mut alpha = vec![0; width * height];
        rasterizer.for_each_pixel(|index, coverage| {
            alpha[index] = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
        });
        GlyphBitmap {
            alpha,
            width: width as u32,
            height: height as u32,
            x_min: left,
            y_min: self.bottom.floor(),
            x_max: left + width as f32,
            y_max: top,
        }
    }
}

impl OutlinePen for RasterizingPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.record_point(x, y);
        self.commands.push(PathCommand::MoveTo(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.record_point(x, y);
        self.commands.push(PathCommand::LineTo(x, y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.record_point(cx0, cy0);
        self.record_point(x, y);
        self.commands.push(PathCommand::QuadTo(cx0, cy0, x, y));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.record_point(cx0, cy0);
        self.record_point(cx1, cy1);
        self.record_point(x, y);
        self.commands
            .push(PathCommand::CurveTo(cx0, cy0, cx1, cy1, x, y));
    }

    fn close(&mut self) {
        self.commands.push(PathCommand::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_backend::{FontSource, HuoziFontManager};
    use tiqian::core::geometry::text_range;
    use tiqian::core::text::Text;
    use tiqian::core::text_model::TextStyle;
    use tiqian::font::font_policy::FontRole;
    use tiqian::shaping::font_backend::{FontBackend, FontBackendRequest};

    const SOURCE_HAN_SANS: &[u8] = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");

    #[test]
    fn bitmap_bounds_match_ink_bounds_in_baseline_coordinates() {
        let manager =
            HuoziFontManager::from_sources(vec![FontSource::new(SOURCE_HAN_SANS.to_vec())])
                .unwrap();
        let text = Text::from("中");
        let shaped = manager.shape(&FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            TextStyle::builder().font_size(96.0).build(),
            FontRole::CjkText,
        ));
        let glyph_id = shaped.shaping.glyph_runs[0].glyphs[0].id;
        let bitmap = rasterize_outline(&manager, &shaped.face, glyph_id, 96.0)
            .unwrap()
            .unwrap();
        let ink = manager
            .glyph_ink_bounds(&shaped.face, glyph_id, 96.0)
            .unwrap();

        assert!((bitmap.x_min - ink.left).abs() <= 1.0);
        assert!((bitmap.x_max - ink.right).abs() <= 1.0);
        assert!((bitmap.y_min + ink.bottom).abs() <= 1.0);
        assert!((bitmap.y_max + ink.top).abs() <= 1.0);
    }
}
