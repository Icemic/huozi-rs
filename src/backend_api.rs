//! Owned replay data for renderers that do not use Huozi's reusable atlas.
use crate::font_backend::HuoziFontManager;
use tiqian::core::font_face::FontFaceId;
use tiqian::core::geometry::Rect;
use tiqian::shaping::font_backend::{FontBackend, FontBackendRequest};

#[derive(Clone, Debug)]
pub struct ShapedGlyph {
    pub face: FontFaceId,
    pub glyph_id: u32,
    pub source: std::ops::Range<i32>,
    pub advance: f32,
    pub x: f32,
    pub y: f32,
    pub bounds: Option<Rect>,
}

#[derive(Clone, Debug)]
pub struct ShapedText {
    pub glyphs: Vec<ShapedGlyph>,
    pub advance: f32,
}

#[derive(Clone, Debug)]
pub struct SdfGlyph {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Baseline-relative top-left of the padded bitmap, in y-down coordinates.
    pub left: f32,
    pub top: f32,
    pub radius: f32,
    pub cutoff: f32,
}

impl HuoziFontManager {
    /// Shapes the complete request, retaining HarfRust visual glyph order.
    pub fn shape_glyphs(&self, request: &FontBackendRequest) -> ShapedText {
        self.replay_shaping(self.shape(request).shaping)
    }

    pub fn shape_glyphs_for_face(
        &self,
        face: &FontFaceId,
        request: &FontBackendRequest,
    ) -> ShapedText {
        self.replay_shaping(self.shape_exact_face(face, request))
    }

    fn replay_shaping(
        &self,
        result: tiqian::shaping::text_shaper::ShapingResult,
    ) -> ShapedText {
        ShapedText {
            advance: result.glyph_runs.iter().map(|run| run.advance).sum(),
            glyphs: result
                .glyph_runs
                .iter()
                .flat_map(|run| {
                    run.glyphs.iter().map(|glyph| ShapedGlyph {
                        face: glyph
                            .render_font_face
                            .clone()
                            .unwrap_or_else(|| run.font_face.clone()),
                        glyph_id: glyph.id,
                        source: glyph.cluster_range.start().value()
                            ..glyph.cluster_range.end().value(),
                        advance: glyph.advance,
                        x: glyph.x,
                        y: glyph.y,
                        bounds: glyph.bounds,
                    })
                })
                .collect(),
        }
    }

    /// Returns independent pixels; subsequent atlas eviction cannot change them.
    pub fn rasterize_glyph(
        &self,
        face: &FontFaceId,
        glyph_id: u32,
        size: f32,
    ) -> Result<Option<SdfGlyph>, String> {
        let bitmap = crate::glyph_rasterizer::rasterize_outline(self, face, glyph_id, size)
            .map_err(|error| format!("{error:?}"))?;
        Ok(bitmap.map(|bitmap| {
            let (pixels, width, height) =
                crate::sdf::calculate_sdf(&bitmap.alpha, bitmap.width, bitmap.height);
            SdfGlyph {
                pixels,
                width,
                height,
                left: bitmap.x_min - crate::constant::BUFFER as f32,
                top: -bitmap.y_max - crate::constant::BUFFER as f32,
                radius: crate::constant::RADIUS,
                cutoff: crate::constant::CUTOFF,
            }
        }))
    }
}
