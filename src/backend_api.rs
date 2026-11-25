//! 给不使用 Huozi 图集的渲染方提供的回放数据：字形、来源区间与独立的 SDF 位图。
//!
//! 这些数据与 `RichTextLayoutOutput` 同源，但不依赖图集生命周期，也不会因为图集淘汰而改变。

use std::ops::Range;

use crate::font_backend::HuoziFontManager;
use tiqian::core::font_face::FontFaceId;
use tiqian::core::geometry::Rect;
use tiqian::shaping::font_backend::FontBackendRequest;

/// 一个已 shaping 的字形及其在输入文本中的来源区间。
#[derive(Clone, Debug)]
pub struct ShapedGlyph {
    pub face: FontFaceId,
    pub glyph_id: u32,
    /// 该字形覆盖的输入 scalar 半开范围；簇内多个字形相邻不重叠。
    pub source: Range<i32>,
    pub advance: f32,
    /// 相对所属排版簇起点的横向偏移。
    pub x: f32,
    pub y: f32,
    pub bounds: Option<Rect>,
}

/// 一次 shaping 请求的完整回放结果；`glyphs` 保持 HarfRust 的视觉顺序。
#[derive(Clone, Debug)]
pub struct ShapedText {
    pub glyphs: Vec<ShapedGlyph>,
    pub advance: f32,
}

/// 一个字形独立的 SDF 位图与其基线相对位置。
#[derive(Clone, Debug)]
pub struct SdfGlyph {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// 位图左上角相对基线的位置，y 轴向下。
    pub left: f32,
    pub top: f32,
    pub radius: f32,
    pub cutoff: f32,
}

impl HuoziFontManager {
    /// 按字体回退顺序 shaping，返回保留视觉顺序与逐字形来源的回放数据。
    pub fn shape_glyphs(&self, request: &FontBackendRequest) -> ShapedText {
        let (result, sources) = self.shape_with_sources(request);
        self.replay_shaping(result.shaping, sources)
    }

    /// 指定字体面 shaping，跳过回退链；用于验证某个字体本身的 shaping 行为。
    pub fn shape_glyphs_for_face(
        &self,
        face: &FontFaceId,
        request: &FontBackendRequest,
    ) -> ShapedText {
        let (shaping, sources) = self.shape_exact_face_with_sources(face, request);
        self.replay_shaping(shaping, sources)
    }

    fn replay_shaping(
        &self,
        result: tiqian::shaping::text_shaper::ShapingResult,
        mut sources: Vec<Range<i32>>,
    ) -> ShapedText {
        let glyph_count = result
            .glyph_runs
            .iter()
            .map(|run| run.glyphs.len())
            .sum::<usize>();
        debug_assert_eq!(sources.len(), glyph_count, "每个字形都应当有对应的来源区间");
        // 来源缺失时退回簇范围，保证回放数据仍然可用。
        let fallback = sources.last().cloned().unwrap_or_default();
        sources.resize(glyph_count, fallback);
        let mut cursor = 0;
        let mut glyphs = Vec::with_capacity(glyph_count);
        for run in &result.glyph_runs {
            for glyph in &run.glyphs {
                let source = std::mem::take(&mut sources[cursor]);
                cursor += 1;
                glyphs.push(ShapedGlyph {
                    face: glyph
                        .render_font_face
                        .clone()
                        .unwrap_or_else(|| run.font_face.clone()),
                    glyph_id: glyph.id,
                    source,
                    advance: glyph.advance,
                    x: glyph.x,
                    y: glyph.y,
                    bounds: glyph.bounds,
                });
            }
        }
        ShapedText {
            glyphs,
            advance: result.glyph_runs.iter().map(|run| run.advance).sum(),
        }
    }

    /// 返回独立像素；图集淘汰不会改变已返回的位图。
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
