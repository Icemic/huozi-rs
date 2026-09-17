use log::warn;
use lru::LruCache;
use std::num::NonZeroUsize;
use tiqian::layout::paragraph_layout_engine::{ParagraphLayoutEngine, ParagraphLayoutEngineBuilder};

use crate::constant::{BUFFER, CUTOFF, FONT_SIZE, GRID_SIZE, RADIUS, TEXTURE_SIZE};
use crate::font_backend::{FontSource, HuoziFontManager};
use crate::glyph_rasterizer::{rasterize_outline, GlyphBitmap};
use crate::glyph_metrics::GlyphMetrics;
use crate::sdf::TinySDF;
use tiqian::core::font_face::FontFaceId;

pub use crate::layout::ColorSpace;

#[derive(Debug, Clone, Default)]
pub struct Glyph {
    pub ch: char,
    pub font_face: Option<FontFaceId>,
    pub glyph_id: u32,
    pub metrics: GlyphMetrics,
    pub page: i32,
    pub index: u32,
    pub grid_count: u32,
    pub u_min: f32,
    pub u_max: f32,
    pub v_min: f32,
    pub v_max: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum AtlasKey {
    ShapedGlyph { face: FontFaceId, glyph_id: u32 },
}

pub struct TextureAtlas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl TextureAtlas {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn clear_channel_rect(&mut self, channel: usize, x: i32, y: i32, width: i32, height: i32) {
        for y in y..y + height {
            for x in x..x + width {
                self.set_channel(channel, x, y, 0);
            }
        }
    }

    fn set_channel(&mut self, channel: usize, x: i32, y: i32, value: u8) {
        let index = ((y as usize * self.width as usize + x as usize) * 4) + channel;
        self.pixels[index] = value;
    }
}

pub struct Huozi {
    pub(crate) font_manager: HuoziFontManager,
    pub(crate) layout_engine: ParagraphLayoutEngine,
    tiny_sdf: TinySDF,
    texture: TextureAtlas,
    cache: lru::LruCache<AtlasKey, Glyph>,
    fallback_glyph_ids: LruCache<AtlasKey, u32>,
    next_grid_index: u32,
    /// increase this flag when the cache is changed
    image_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HuoziError {
    NoValidFontFaces { detail: String },
}

impl std::fmt::Display for HuoziError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoValidFontFaces { detail } => {
                write!(formatter, "没有可用的字体 face：{detail}")
            }
        }
    }
}

impl std::error::Error for HuoziError {}

impl Huozi {
    pub fn new(font_sources: Vec<FontSource>) -> Result<Self, HuoziError> {
        let font_manager = HuoziFontManager::from_sources(font_sources)
            .map_err(|detail| HuoziError::NoValidFontFaces { detail })?;
        let layout_engine = ParagraphLayoutEngineBuilder::new(Box::new(font_manager.clone())).build();

        let texture = TextureAtlas::new(TEXTURE_SIZE, TEXTURE_SIZE);

        let tiny_sdf = TinySDF::new(GRID_SIZE as u32, BUFFER as u32, RADIUS, CUTOFF);

        let cache_capacity =
            NonZeroUsize::new((TEXTURE_SIZE / GRID_SIZE as u32).pow(2) as usize * 4).unwrap();
        let cache = LruCache::new(cache_capacity);

        Ok(Self {
            font_manager,
            layout_engine,
            tiny_sdf,
            texture,
            cache,
            fallback_glyph_ids: LruCache::new(cache_capacity),
            next_grid_index: 0,
            image_version: 0,
        })
    }

    pub fn get_glyph_by_id(&mut self, face: &FontFaceId, glyph_id: u32) -> Glyph {
        let key = AtlasKey::ShapedGlyph {
            face: face.clone(),
            glyph_id,
        };
        if let Some(glyph) = self.cache.get(&key) {
            return glyph.clone();
        }
        if let Some(actual_glyph_id) = self.fallback_glyph_ids.get(&key).copied() {
            let fallback_key = AtlasKey::ShapedGlyph {
                face: face.clone(),
                glyph_id: actual_glyph_id,
            };
            if let Some(glyph) = self.cache.get(&fallback_key) {
                return glyph.clone();
            }
            self.fallback_glyph_ids.pop(&key);
        }

        if self.font_manager.has_color_glyph(face, glyph_id) {
            warn!(
                "color glyph {glyph_id} from {face} uses the same-face glyph 0 SDF fallback"
            );
            return self.cache_fallback_glyph(face, key);
        }

        let bitmap = match rasterize_outline(&self.font_manager, face, glyph_id, FONT_SIZE as f32) {
            Ok(Some(bitmap)) if !bitmap.alpha.is_empty() => bitmap,
            Ok(_) if glyph_id != 0 => {
                warn!(
                    "glyph {glyph_id} from {face} has no single-channel outline; uses the same-face glyph 0 SDF fallback"
                );
                return self.cache_fallback_glyph(face, key);
            }
            Ok(_) => GlyphBitmap {
                alpha: Vec::new(),
                width: 0,
                height: 0,
                x_min: 0.0,
                y_min: 0.0,
                x_max: 0.0,
                y_max: 0.0,
            },
            Err(error) if glyph_id != 0 => {
                warn!("failed to rasterize glyph {glyph_id} from {face}: {error:?}");
                return self.cache_fallback_glyph(face, key);
            }
            Err(error) => {
                warn!("failed to rasterize glyph 0 from {face}: {error:?}");
                GlyphBitmap {
                    alpha: Vec::new(),
                    width: 0,
                    height: 0,
                    x_min: 0.0,
                    y_min: 0.0,
                    x_max: 0.0,
                    y_max: 0.0,
                }
            }
        };

        let metrics = GlyphMetrics {
            width: bitmap.width,
            height: bitmap.height,
            x_min: bitmap.x_min,
            y_min: bitmap.y_min,
            x_max: bitmap.x_max,
            y_max: bitmap.y_max,
            ..Default::default()
        };
        let grid_count = if bitmap.width as f64 > FONT_SIZE {
            ((bitmap.width as f64) / FONT_SIZE + 0.5) as u32
        } else {
            1
        };
        let (bitmap, width, height) = self
            .tiny_sdf
            .calculate(&bitmap.alpha, bitmap.width, bitmap.height, grid_count);
        let glyph = Glyph {
            ch: '\0',
            font_face: Some(face.clone()),
            glyph_id,
            metrics,
            page: 0,
            index: 0,
            grid_count: 0,
            u_min: 0.0,
            u_max: 0.0,
            v_min: 0.0,
            v_max: 0.0,
        };
        let grid_size = GRID_SIZE as i32;
        let line_count = self.texture.width() as i32 / grid_size;
        let (page, index_in_page, overwrite) =
            if let Some((_, expired_glyph)) = self.cache.push(key.clone(), glyph) {
                (expired_glyph.page, expired_glyph.index, true)
            } else {
                let page = self.next_grid_index as i32 / (line_count * line_count);
                let index_in_page = self.next_grid_index as i32 % (line_count * line_count);
                self.next_grid_index += grid_count;
                (page, index_in_page as u32, false)
            };
        let grid_x = grid_size * (index_in_page as i32 % line_count);
        let grid_y = grid_size * (index_in_page as i32 / line_count);
        if overwrite {
            self.texture.clear_channel_rect(page as usize, grid_x, grid_y, grid_size, grid_size);
        }
        let offset_x =
            grid_x + ((GRID_SIZE * grid_count as f64) / 2.0 - width as f64 / 2.0).ceil() as i32;
        let offset_y = grid_y + (GRID_SIZE / 2.0 - height as f64 / 2.0).ceil() as i32;
        let source_x_start = (grid_x - offset_x).max(0) as usize;
        let source_x_end = (grid_x + grid_size * grid_count as i32 - offset_x)
            .min(width as i32)
            .max(0) as usize;
        let texture_width = self.texture.width as usize;
        let channel = page as usize;
        for (source_y, row) in bitmap.chunks_exact(width as usize).enumerate() {
            let y = source_y as i32 + offset_y;
            if y <= grid_y || y >= grid_y + grid_size {
                continue;
            }
            let x = offset_x + source_x_start as i32;
            let mut texture_index = ((y as usize * texture_width + x as usize) * 4) + channel;
            for &value in &row[source_x_start..source_x_end] {
                self.texture.pixels[texture_index] = value;
                texture_index += 4;
            }
        }
        let texture_width = self.texture.width as f32;
        let glyph = self.cache.get_mut(&key).unwrap();
        glyph.page = page;
        glyph.index = index_in_page;
        glyph.grid_count = grid_count;
        glyph.u_min = grid_x as f32 / texture_width;
        glyph.v_min = grid_y as f32 / texture_width;
        glyph.u_max = (grid_x + grid_size * grid_count as i32) as f32 / texture_width;
        glyph.v_max = (grid_y + grid_size) as f32 / texture_width;
        let glyph = glyph.clone();
        self.image_version += 1;
        glyph
    }

    fn cache_fallback_glyph(
        &mut self,
        face: &FontFaceId,
        requested_key: AtlasKey,
    ) -> Glyph {
        let glyph = self.get_glyph_by_id(face, 0);
        self.fallback_glyph_ids.put(requested_key, 0);
        glyph
    }

    pub fn image_version(&self) -> u64 {
        self.image_version
    }

    pub fn texture_pixels(&self) -> &TextureAtlas {
        &self.texture
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::tiqian_input::HuoziTiqianInputAdapter;
    use crate::layout::tiqian_output::HuoziTiqianOutputAdapter;
    use crate::layout::{ColorSpace, LayoutStyle};
    use crate::parser::{
        ScalarOffset as HuoziScalarOffset, SegmentId, ShadowStyle, SourceRange, StrokeStyle,
        TextRun, TextSpan, TextStyle as HuoziTextStyle,
    };
    use crate::FontSource;
    use tiqian::core::geometry::{Size, text_range};
    use tiqian::core::int_range::IntRange;
    use tiqian::core::layout_model::{LayoutResult, LineBox};
    use tiqian::core::text::Text;
    use tiqian::core::text_model::TextStyle;
    use tiqian::font::font_policy::FontRole;
    use tiqian::shaping::font_backend::{FontBackend, FontBackendRequest};
    use tiqian::shaping::replayable_font_backend::ReplayableFontCatalog;

    #[test]
    fn reports_a_structured_error_when_no_font_source_is_valid() {
        let result = Huozi::new(vec![FontSource::new(vec![0])]);

        assert!(matches!(result, Err(HuoziError::NoValidFontFaces { .. })));
    }

    #[test]
    fn glyph_id_atlas_preserves_the_selected_font_face_and_glyph_id() {
        let fira_code = include_bytes!("../examples/assets/FiraCode-VF.ttf");
        let source_han_sans = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");
        let mut huozi = Huozi::new(vec![
            FontSource::with_alias(fira_code.to_vec(), "fira".to_owned()),
            FontSource::with_alias(source_han_sans.to_vec(), "source-han".to_owned()),
        ])
        .unwrap();
        let first_face = huozi.font_manager.faces()[0].id.clone();
        let second_face = huozi.font_manager.faces()[1].id.clone();

        let first = huozi.get_glyph_by_id(&first_face, 0);
        let second = huozi.get_glyph_by_id(&second_face, 0);

        assert_ne!(first_face, second_face);
        assert_eq!(first.font_face.as_ref(), Some(&first_face));
        assert_eq!(second.font_face.as_ref(), Some(&second_face));
        assert_eq!(first.glyph_id, 0);
        assert_eq!(second.glyph_id, 0);

        let image_version = huozi.image_version();
        let cached_first = huozi.get_glyph_by_id(&first_face, 0);
        let cached_second = huozi.get_glyph_by_id(&second_face, 0);

        assert_eq!(cached_first.font_face.as_ref(), Some(&first_face));
        assert_eq!(cached_second.font_face.as_ref(), Some(&second_face));
        assert_eq!(huozi.image_version(), image_version);
    }

    #[test]
    fn glyph_id_atlas_uses_same_face_notdef_when_outline_is_absent() {
        let font = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");
        let mut huozi = Huozi::new(vec![FontSource::new(font.to_vec())]).unwrap();
        let text = Text::from("中");
        let request = FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            TextStyle::default(),
            FontRole::CjkText,
        );
        let shaped = huozi.font_manager.shape(&request);
        let glyph = huozi.get_glyph_by_id(&shaped.face, u32::MAX);

        assert_eq!(glyph.font_face.as_ref(), Some(&shaped.face));
        assert_eq!(glyph.glyph_id, 0);
        let image_version = huozi.image_version();
        let cached = huozi.get_glyph_by_id(&shaped.face, u32::MAX);

        assert_eq!(cached.glyph_id, 0);
        assert_eq!(huozi.image_version(), image_version);
        assert_eq!(huozi.fallback_glyph_ids.len(), 1);
    }

    #[test]
    fn tiqian_output_replays_glyphs_paints_and_segment_identity() {
        let font = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");
        let mut huozi = Huozi::new(vec![FontSource::new(font.to_vec())]).unwrap();
        let style = HuoziTextStyle {
            fill_color: csscolorparser::Color::from_rgba8(0x12, 0x34, 0x56, 0x78),
            stroke: Some(StrokeStyle {
                stroke_color: csscolorparser::Color::from_rgba8(0x9a, 0xbc, 0xde, 0xf0),
                stroke_width: 2.0,
            }),
            shadow: Some(ShadowStyle {
                shadow_color: csscolorparser::Color::from_rgba8(0x11, 0x22, 0x33, 0x44),
                shadow_offset_x: 0.25,
                shadow_offset_y: 0.5,
                shadow_blur: 8.0,
                shadow_width: 3.0,
            }),
            ..HuoziTextStyle::default()
        };
        let input = HuoziTiqianInputAdapter::adapt(
            &[TextSpan {
                runs: vec![TextRun {
                    text: "中".to_string(),
                    style: style.clone(),
                    source_range: SourceRange {
                        segment_id: Some(SegmentId::Lite(7)),
                        start: HuoziScalarOffset(0),
                        end: HuoziScalarOffset(1),
                    },
                }],
                span_id: None,
            }],
            &LayoutStyle::default(),
            &style,
        );
        let crate::layout::tiqian_input::HuoziTiqianInput {
            layout_input,
            source_map,
        } = input;
        let text = layout_input.content.text.clone();
        let request = FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            layout_input.text_style.clone(),
            FontRole::CjkText,
        );
        let shaped = huozi.font_manager.shape(&request);
        let layout_result = LayoutResult::new(
            layout_input,
            Size {
                width: shaped.shaping.glyph_runs[0].advance,
                height: 48.0,
            },
            shaped.shaping.clusters,
            shaped.shaping.glyph_runs,
            vec![
                LineBox::builder(
                    text_range(0, 1),
                    IntRange::new(0, 0),
                    32.0,
                    0.0,
                    48.0,
                    32.0,
                    32.0,
                    32.0,
                )
                .build(),
            ],
        );

        let (glyphs, spans, width, height) = HuoziTiqianOutputAdapter::adapt(
            &mut huozi,
            &layout_result,
            &source_map,
            &ColorSpace::SRGB,
        );

        assert_eq!(glyphs.len(), 1);
        assert_eq!(glyphs[0].fill.len(), 4);
        assert_eq!(glyphs[0].fill[0].color, [0x12 as f32 / 255.0, 0x34 as f32 / 255.0, 0x56 as f32 / 255.0, 0x78 as f32 / 255.0]);
        assert_eq!(glyphs[0].fill[0].buffer, 0.735357);
        assert_eq!(glyphs[0].fill[0].fill_buffer, 2.0);
        assert!(glyphs[0].fill.iter().all(|vertex| vertex.page >= 0));
        assert!(glyphs[0].fill.iter().all(|vertex| {
            vertex.tex_coords[0] >= 0.0
                && vertex.tex_coords[0] <= 1.0
                && vertex.tex_coords[1] >= 0.0
                && vertex.tex_coords[1] <= 1.0
        }));
        let stroke = glyphs[0].stroke.unwrap();
        assert_eq!(stroke[0].color, [0x9a as f32 / 255.0, 0xbc as f32 / 255.0, 0xde as f32 / 255.0, 0xf0 as f32 / 255.0]);
        assert_eq!(stroke[0].fill_buffer, 0.735357);
        let shadow = glyphs[0].shadow.unwrap();
        assert_eq!(shadow[0].color, [0x11 as f32 / 255.0, 0x22 as f32 / 255.0, 0x33 as f32 / 255.0, 0x44 as f32 / 255.0]);
        assert_eq!(shadow[0].position[0] - glyphs[0].fill[0].position[0], 0.5);
        assert_eq!(shadow[0].position[1] - glyphs[0].fill[0].position[1], 1.0);
        assert!(shadow[0].gamma > glyphs[0].fill[0].gamma);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].segment_id, SegmentId::Lite(7));
        assert_eq!(spans[0].glyph_range, 0..1);
        assert_eq!(width, layout_result.size.width.round() as u32);
        assert_eq!(height, 48);
    }

    #[test]
    fn tiqian_output_keeps_all_glyphs_in_a_shaping_cluster() {
        let font = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");
        let mut huozi = Huozi::new(vec![FontSource::new(font.to_vec())]).unwrap();
        let style = HuoziTextStyle::default();
        let input = HuoziTiqianInputAdapter::adapt(
            &[TextSpan {
                runs: vec![TextRun {
                    text: "中国".to_string(),
                    style: style.clone(),
                    source_range: SourceRange {
                        segment_id: Some(SegmentId::Lite(8)),
                        start: HuoziScalarOffset(0),
                        end: HuoziScalarOffset(2),
                    },
                }],
                span_id: None,
            }],
            &LayoutStyle::default(),
            &style,
        );
        let crate::layout::tiqian_input::HuoziTiqianInput {
            layout_input,
            source_map,
        } = input;
        let text = layout_input.content.text.clone();
        let request = FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            layout_input.text_style.clone(),
            FontRole::CjkText,
        );
        let shaped = huozi.font_manager.shape(&request);
        let glyph_count = shaped.shaping.glyph_runs[0].glyphs.len();
        let layout_result = LayoutResult::new(
            layout_input,
            Size {
                width: shaped.shaping.glyph_runs[0].advance,
                height: 48.0,
            },
            shaped.shaping.clusters,
            shaped.shaping.glyph_runs,
            vec![
                LineBox::builder(
                    text_range(0, 2),
                    IntRange::new(0, 0),
                    32.0,
                    0.0,
                    48.0,
                    64.0,
                    64.0,
                    64.0,
                )
                .build(),
            ],
        );

        let (glyphs, spans, _, _) = HuoziTiqianOutputAdapter::adapt(
            &mut huozi,
            &layout_result,
            &source_map,
            &ColorSpace::SRGB,
        );

        assert_eq!(glyphs.len(), glyph_count);
        assert_eq!(glyphs[0].row, 0);
        assert_eq!(glyphs[0].col, 0);
        assert_eq!(glyphs[1].row, 0);
        assert_eq!(glyphs[1].col, 1);
        assert_ne!(glyphs[0].fill[0].position[0], glyphs[1].fill[0].position[0]);
        assert_eq!(spans[0].segment_id, SegmentId::Lite(8));
        assert_eq!(spans[0].glyph_range, 0..glyph_count);
    }

    #[test]
    fn tiqian_output_replays_line_end_hyphen_at_the_line_visual_end() {
        let font = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");
        let mut huozi = Huozi::new(vec![FontSource::new(font.to_vec())]).unwrap();
        let style = HuoziTextStyle::default();
        let input = HuoziTiqianInputAdapter::adapt(
            &[TextSpan {
                runs: vec![TextRun {
                    text: "中".to_string(),
                    style: style.clone(),
                    source_range: SourceRange {
                        segment_id: Some(SegmentId::Lite(10)),
                        start: HuoziScalarOffset(0),
                        end: HuoziScalarOffset(1),
                    },
                }],
                span_id: None,
            }],
            &LayoutStyle::default(),
            &style,
        );
        let crate::layout::tiqian_input::HuoziTiqianInput {
            layout_input,
            source_map,
        } = input;
        let text = layout_input.content.text.clone();
        let request = FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            layout_input.text_style.clone(),
            FontRole::CjkText,
        );
        let shaped = huozi.font_manager.shape(&request);
        let hyphen = huozi.font_manager.shape(&FontBackendRequest::new(
            Text::from("-"),
            text_range(0, 1),
            layout_input.text_style.clone(),
            FontRole::LatinText,
        ));
        let hyphen_advance = hyphen.shaping.clusters[0].advance;
        let hyphen_glyphs = hyphen
            .shaping
            .glyph_runs
            .into_iter()
            .flat_map(|run| run.glyphs)
            .collect();
        let layout_result = LayoutResult::new(
            layout_input,
            Size {
                width: 48.0,
                height: 48.0,
            },
            shaped.shaping.clusters,
            shaped.shaping.glyph_runs,
            vec![
                LineBox::builder(
                    text_range(0, 1),
                    IntRange::new(0, 0),
                    32.0,
                    0.0,
                    48.0,
                    48.0,
                    48.0,
                    48.0,
                )
                .hyphen_advance(hyphen_advance)
                .hyphen_glyphs(hyphen_glyphs)
                .build(),
            ],
        );

        let (glyphs, spans, _, _) = HuoziTiqianOutputAdapter::adapt(
            &mut huozi,
            &layout_result,
            &source_map,
            &ColorSpace::SRGB,
        );

        assert_eq!(glyphs.len(), 2);
        assert_eq!(glyphs[1].x, 48);
        assert_eq!(glyphs[1].row, 0);
        assert_eq!(glyphs[1].col, 1);
        assert_eq!(spans[0].segment_id, SegmentId::Lite(10));
        assert_eq!(spans[0].glyph_range, 0..2);
    }

    #[test]
    fn tiqian_output_drops_a_line_that_exceeds_box_height() {
        let font = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");
        let mut huozi = Huozi::new(vec![FontSource::new(font.to_vec())]).unwrap();
        let style = HuoziTextStyle::default();
        let layout_style = LayoutStyle {
            box_height: Some(40.0),
            ..LayoutStyle::default()
        };
        let input = HuoziTiqianInputAdapter::adapt(
            &[TextSpan {
                runs: vec![TextRun {
                    text: "中".to_string(),
                    style: style.clone(),
                    source_range: SourceRange {
                        segment_id: Some(SegmentId::Lite(9)),
                        start: HuoziScalarOffset(0),
                        end: HuoziScalarOffset(1),
                    },
                }],
                span_id: None,
            }],
            &layout_style,
            &style,
        );
        let crate::layout::tiqian_input::HuoziTiqianInput {
            layout_input,
            source_map,
        } = input;
        let text = layout_input.content.text.clone();
        let request = FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            layout_input.text_style.clone(),
            FontRole::CjkText,
        );
        let shaped = huozi.font_manager.shape(&request);
        let layout_result = LayoutResult::new(
            layout_input,
            Size {
                width: shaped.shaping.glyph_runs[0].advance,
                height: 48.0,
            },
            shaped.shaping.clusters,
            shaped.shaping.glyph_runs,
            vec![
                LineBox::builder(
                    text_range(0, 1),
                    IntRange::new(0, 0),
                    32.0,
                    0.0,
                    48.0,
                    32.0,
                    32.0,
                    32.0,
                )
                .build(),
            ],
        );

        let (glyphs, spans, _, height) = HuoziTiqianOutputAdapter::adapt(
            &mut huozi,
            &layout_result,
            &source_map,
            &ColorSpace::SRGB,
        );

        assert!(glyphs.is_empty());
        assert!(spans.is_empty());
        assert_eq!(height, 0);
    }
}
