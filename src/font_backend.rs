use harfrust::{Direction, Feature, FontRef as HarfRustFontRef, ShaperData, Tag, UnicodeBuffer};
use log::warn;
use skrifa::attribute::{Attributes, Style};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::pen::ControlBoundsPen;
use skrifa::outline::{DrawError, DrawSettings, OutlinePen};
use skrifa::raw::TableProvider;
use skrifa::{FontRef as SkrifaFontRef, GlyphId, MetadataProvider};
use std::collections::HashMap;
use std::sync::Arc;
use tiqian::common::HashSet;
use tiqian::core::font_face::{FontFaceId, FontVariationInstance};
use tiqian::core::geometry::Rect;
use tiqian::core::layout_model::{Cluster, Glyph, GlyphRun, ShapingDecisionInfo};
use tiqian::core::text::Text;
use tiqian::font::font_metrics::FontMetricSource;
use tiqian::font::font_metrics::FontMetricsRequest;
use tiqian::font::font_policy::{FontRole, RawFontMetrics};
use tiqian::shaping::font_backend::{
    FontBackend, FontBackendRequest, FontBackendShapingResult, FontCandidateAttempt,
};
use tiqian::shaping::replayable_font_backend::{
    FontBackendCapabilityReport, ReplayableFontCatalog, ReplayableFontFaceDescriptor,
};
use tiqian::shaping::text_shaper::{ShapingResult, ShapingSource};

#[derive(Clone, Debug)]
pub struct FontSource {
    pub bytes: Vec<u8>,
    pub alias: Option<String>,
}

impl FontSource {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, alias: None }
    }

    pub fn with_alias(bytes: Vec<u8>, alias: String) -> Self {
        Self {
            bytes,
            alias: Some(alias),
        }
    }
}

#[derive(Clone)]
struct FontFaceRecord {
    id: FontFaceId,
    source_index: usize,
    collection_index: u32,
    bytes: Arc<[u8]>,
    units_per_em: u16,
    ascent: i16,
    descent: i16,
    leading: i16,
    typo_ascent: Option<i16>,
    typo_descent: Option<i16>,
}

#[derive(Clone)]
pub(crate) struct HuoziFontManager {
    faces: Arc<[FontFaceRecord]>,
    face_indices: Arc<HashMap<FontFaceId, usize>>,
    descriptors: Arc<[ReplayableFontFaceDescriptor]>,
    descriptor_indices: Arc<HashMap<FontFaceId, usize>>,
    capability_report: Arc<FontBackendCapabilityReport>,
}

impl HuoziFontManager {
    pub(crate) fn from_sources(sources: Vec<FontSource>) -> Result<Self, String> {
        let mut faces = Vec::new();
        let mut descriptors = Vec::new();

        for (source_index, source) in sources.into_iter().enumerate() {
            let bytes: Arc<[u8]> = source.bytes.into();
            let collection_len = match skrifa::raw::FileRef::new(&bytes) {
                Ok(skrifa::raw::FileRef::Font(_)) => 1,
                Ok(skrifa::raw::FileRef::Collection(collection)) => collection.len(),
                Err(error) => {
                    warn!("skip invalid font source {source_index}: {error:?}");
                    continue;
                }
            };
            let source_label = source
                .alias
                .unwrap_or_else(|| format!("font-source-{source_index}"));

            for collection_index in 0..collection_len {
                let Ok(font) = SkrifaFontRef::from_index(&bytes, collection_index) else {
                    warn!("skip invalid font face {source_label}#{collection_index}");
                    continue;
                };
                let Ok(head) = font.head() else {
                    warn!("skip font face without head table {source_label}#{collection_index}");
                    continue;
                };
                let Ok(hhea) = font.hhea() else {
                    warn!("skip font face without hhea table {source_label}#{collection_index}");
                    continue;
                };
                let id = FontFaceId::new(
                    format!("{source_label}:{source_index}"),
                    collection_index,
                    FontVariationInstance::default(),
                );
                let os2 = font.os2().ok();
                let attributes = Attributes::new(&font);
                let record = FontFaceRecord {
                    id: id.clone(),
                    source_index,
                    collection_index,
                    bytes: bytes.clone(),
                    units_per_em: head.units_per_em(),
                    ascent: hhea.ascender().into(),
                    descent: hhea.descender().into(),
                    leading: hhea.line_gap().into(),
                    typo_ascent: os2.as_ref().map(|table| table.s_typo_ascender()),
                    typo_descent: os2.as_ref().map(|table| table.s_typo_descender()),
                };
                descriptors.push(
                    ReplayableFontFaceDescriptor::builder(
                        id,
                        HashSet::from([source_label.clone()]),
                        HashSet::from([
                            FontRole::CjkText,
                            FontRole::CjkPunctuation,
                            FontRole::LatinText,
                            FontRole::Symbol,
                            FontRole::Emoji,
                            FontRole::Unknown,
                        ]),
                        format!("{source_label}#{collection_index}"),
                    )
                    .weight(attributes.weight.value().round() as i32)
                    .italic(!matches!(attributes.style, Style::Normal))
                    .build(),
                );
                faces.push(record);
            }
        }

        if faces.is_empty() {
            return Err("no valid font faces".to_owned());
        }
        let face_indices = faces
            .iter()
            .enumerate()
            .map(|(index, face)| (face.id.clone(), index))
            .collect();
        let descriptor_indices = descriptors
            .iter()
            .enumerate()
            .map(|(index, descriptor)| (descriptor.id.clone(), index))
            .collect();
        let capability_report = FontBackendCapabilityReport::new(
            "HuoziHarfRustFontBackend".to_owned(),
            "controlled-font-bytes".to_owned(),
            descriptors.clone(),
        );
        Ok(Self {
            faces: faces.into(),
            face_indices: Arc::new(face_indices),
            descriptors: descriptors.into(),
            descriptor_indices: Arc::new(descriptor_indices),
            capability_report: Arc::new(capability_report),
        })
    }

    fn face_for_id(&self, id: &FontFaceId) -> &FontFaceRecord {
        &self.faces[*self
            .face_indices
            .get(id)
            .expect("HuoziFontManager received an unregistered FontFaceId")]
    }

    fn raw_metrics(&self, id: &FontFaceId, font_size: f32) -> RawFontMetrics {
        let face = self.face_for_id(id);
        let scale = font_size / face.units_per_em as f32;
        RawFontMetrics {
            ascent: face.ascent as f32 * scale,
            descent: -(face.descent as f32) * scale,
            leading: face.leading as f32 * scale,
            source: FontMetricSource::RawTables,
            typo_ascent: face.typo_ascent.map(|value| value as f32 * scale),
            typo_descent: face.typo_descent.map(|value| -(value as f32) * scale),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn glyph_ink_bounds(
        &self,
        id: &FontFaceId,
        glyph_id: u32,
        font_size: f32,
    ) -> Option<Rect> {
        let face = self.face_for_id(id);
        let font = SkrifaFontRef::from_index(&face.bytes, face.collection_index).ok()?;
        let scale = font_size / face.units_per_em as f32;
        glyph_ink_bounds(&font, glyph_id, scale)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn draw_outline<P: OutlinePen>(
        &self,
        id: &FontFaceId,
        glyph_id: u32,
        font_size: f32,
        pen: &mut P,
    ) -> Result<bool, DrawError> {
        let face = self.face_for_id(id);
        let font = SkrifaFontRef::from_index(&face.bytes, face.collection_index)
            .expect("registered face must remain readable by SkRifa");
        let Some(glyph) = font.outline_glyphs().get(GlyphId::new(glyph_id)) else {
            return Ok(false);
        };
        glyph.draw(
            DrawSettings::unhinted(
                Size::new(font_size),
                LocationRef::new(font.axes().location(Vec::<(&str, f32)>::new()).coords()),
            ),
            pen,
        )?;
        Ok(true)
    }

    pub(crate) fn has_color_glyph(&self, id: &FontFaceId, glyph_id: u32) -> bool {
        let face = self.face_for_id(id);
        let font = SkrifaFontRef::from_index(&face.bytes, face.collection_index)
            .expect("registered face must remain readable by SkRifa");
        font.color_glyphs().get(GlyphId::new(glyph_id)).is_some()
    }

    fn shape_one_face(
        &self,
        request: &FontBackendRequest,
        face: &FontFaceRecord,
    ) -> (ShapingResult, u32) {
        let font = HarfRustFontRef::from_index(&face.bytes, face.collection_index)
            .expect("registered face must remain readable by HarfRust");
        let data = ShaperData::new(&font);
        let shaper = data.shaper(&font).build();
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(request.display_text.as_str());
        buffer.guess_segment_properties();
        buffer.set_direction(Direction::LeftToRight);
        if request.role == FontRole::CjkPunctuation {
            buffer.set_script(harfrust::script::HAN);
        }
        buffer.set_language(
            request
                .style
                .locale
                .parse()
                .unwrap_or_else(|_| "c".parse().expect("fallback language must parse")),
        );
        let features: Vec<_> = request
            .open_type_features
            .iter()
            .map(|feature| {
                Feature::new(Tag::new(&feature_tag(feature)), feature_value(feature), ..)
            })
            .collect();
        let output = shaper.shape(buffer, harfrust::ShapeOptions::new().features(&features));
        let scale = request.style.font_size / face.units_per_em as f32;
        let mut pen_x = 0_i64;
        let glyphs: Vec<_> = output
            .glyph_infos()
            .iter()
            .zip(output.glyph_positions())
            .map(|(info, position)| {
                let glyph = Glyph::builder(
                    info.glyph_id,
                    request.range,
                    position.x_advance as f32 * scale,
                )
                .x((pen_x + i64::from(position.x_offset)) as f32 * scale)
                .y(-position.y_offset as f32 * scale)
                .render_font_face(Some(face.id.clone()))
                .build();
                pen_x += i64::from(position.x_advance);
                glyph
            })
            .collect();
        let advance = pen_x as f32 * scale;
        let missing_glyphs = glyphs.iter().filter(|glyph| glyph.id == 0).count() as u32;
        let source_text = Text::from(request.text.slice(request.range));
        let decision = ShapingDecisionInfo::builder(
            request.range,
            source_text.clone(),
            request.display_text.clone(),
            Some(face.id.clone()),
            glyphs.len() as i32,
            advance,
            format!("{:?}", ShapingSource::HarfBuzz),
            "HuoziFontManager:complete-shaping".to_owned(),
        )
        .glyphs_without_ink_bounds(glyphs.len() as i32)
        .missing_glyphs(missing_glyphs as i32)
        .language(Some(request.style.locale.clone()))
        .feature_evidence(
            (!request.open_type_features.is_empty()).then(|| request.open_type_features.join(",")),
        )
        .build();
        let cluster = Cluster::with_display_text(
            request.range,
            source_text,
            request.display_text.clone(),
            face.id.clone(),
            advance,
        );
        let run = GlyphRun::with_open_type_features(
            request.range,
            face.id.clone(),
            glyphs,
            advance,
            request.open_type_features.clone(),
        );
        (
            ShapingResult::with_decisions(vec![cluster], vec![run], vec![decision]),
            missing_glyphs,
        )
    }

    fn add_ink_bounds(&self, shaping: &mut ShapingResult, face: &FontFaceRecord, font_size: f32) {
        let font = SkrifaFontRef::from_index(&face.bytes, face.collection_index)
            .expect("registered face must remain readable by SkRifa");
        let scale = font_size / face.units_per_em as f32;
        let mut glyphs_without_ink_bounds = 0;
        for glyph in shaping
            .glyph_runs
            .iter_mut()
            .flat_map(|run| &mut run.glyphs)
        {
            glyph.bounds = glyph_ink_bounds(&font, glyph.id, scale);
            glyphs_without_ink_bounds += i32::from(glyph.bounds.is_none());
        }
        for decision in &mut shaping.decisions {
            decision.glyphs_without_ink_bounds = glyphs_without_ink_bounds;
        }
    }
}

impl ReplayableFontCatalog for HuoziFontManager {
    fn faces(&self) -> &[ReplayableFontFaceDescriptor] {
        &self.descriptors
    }

    fn capability_report(&self) -> &FontBackendCapabilityReport {
        &self.capability_report
    }

    fn face(&self, id: &FontFaceId) -> Option<&ReplayableFontFaceDescriptor> {
        self.descriptor_indices
            .get(id)
            .map(|index| &self.descriptors[*index])
    }
}

impl FontBackend for HuoziFontManager {
    fn shape(&self, request: &FontBackendRequest) -> FontBackendShapingResult {
        let mut attempts = Vec::new();
        let mut preferred = None;
        for face in self.faces.iter() {
            let (mut shaping, missing_glyphs) = self.shape_one_face(request, face);
            attempts.push(FontCandidateAttempt::new(
                format!("source-{}#{}", face.source_index, face.collection_index),
                face.id.clone(),
                missing_glyphs,
            ));
            if missing_glyphs == 0 {
                self.add_ink_bounds(&mut shaping, face, request.style.font_size);
                return FontBackendShapingResult::new(face.id.clone(), shaping, attempts);
            }
            if preferred.is_none() {
                preferred = Some((face.id.clone(), shaping));
                continue;
            }
        }
        let (face, mut shaping) =
            preferred.expect("HuoziFontManager must contain at least one face");
        self.add_ink_bounds(
            &mut shaping,
            self.face_for_id(&face),
            request.style.font_size,
        );
        FontBackendShapingResult::new(face, shaping, attempts)
    }

    fn metrics(&self, request: &FontMetricsRequest) -> RawFontMetrics {
        self.raw_metrics(&request.face, request.font_size)
    }
}

fn feature_tag(feature: &str) -> [u8; 4] {
    let mut tag = [b' '; 4];
    for (slot, byte) in tag.iter_mut().zip(feature.bytes()) {
        if byte == b'=' {
            break;
        }
        *slot = byte;
    }
    tag
}

fn feature_value(feature: &str) -> u32 {
    feature
        .split_once('=')
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiqian::core::geometry::text_range;
    use tiqian::core::text::Text;
    use tiqian::core::text_model::TextStyle;

    const FIRA_CODE: &[u8] = include_bytes!("../examples/assets/FiraCode-VF.ttf");
    const SOURCE_HAN_SANS: &[u8] = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");

    fn request(text: &str) -> FontBackendRequest {
        let text = Text::from(text);
        FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            TextStyle::default(),
            FontRole::CjkText,
        )
    }

    #[test]
    fn selects_the_first_complete_candidate() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned()),
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned()),
        ])
        .unwrap();

        let result = manager.shape(&request("A"));

        assert_eq!(result.attempts.len(), 1);
        assert_eq!(result.selected_attempt().candidate_key, "source-0#0");
        assert_eq!(result.selected_attempt().missing_glyphs, 0);
        assert!(
            result
                .shaping
                .glyph_runs
                .iter()
                .flat_map(|run| &run.glyphs)
                .all(|glyph| {
                    glyph.render_font_face.as_ref() == Some(&result.face) && glyph.bounds.is_some()
                })
        );
    }

    #[test]
    fn falls_back_after_complete_shaping_reports_missing_glyphs() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned()),
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned()),
        ])
        .unwrap();

        let result = manager.shape(&request("中文"));

        assert_eq!(result.attempts.len(), 2);
        assert_eq!(result.attempts[0].candidate_key, "source-0#0");
        assert!(result.attempts[0].has_missing_glyphs());
        assert_eq!(result.selected_attempt().candidate_key, "source-1#0");
        assert_eq!(result.selected_attempt().missing_glyphs, 0);
        assert!(
            result
                .shaping
                .glyph_runs
                .iter()
                .flat_map(|run| &run.glyphs)
                .all(|glyph| {
                    glyph.id != 0
                        && glyph.render_font_face.as_ref() == Some(&result.face)
                        && glyph.bounds.is_some()
                })
        );
    }

    #[test]
    fn retains_the_first_candidate_when_every_face_is_missing() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned()),
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned()),
        ])
        .unwrap();

        let result = manager.shape(&request("\u{10ffff}"));

        assert_eq!(result.attempts.len(), 2);
        assert!(
            result
                .attempts
                .iter()
                .all(FontCandidateAttempt::has_missing_glyphs)
        );
        assert_eq!(result.selected_attempt().candidate_key, "source-0#0");
        assert_eq!(result.face, result.attempts[0].face);
        assert!(
            result
                .shaping
                .glyph_runs
                .iter()
                .flat_map(|run| &run.glyphs)
                .all(|glyph| {
                    glyph.id == 0 && glyph.render_font_face.as_ref() == Some(&result.face)
                })
        );
    }
}

fn glyph_ink_bounds(font: &SkrifaFontRef<'_>, glyph_id: u32, scale: f32) -> Option<Rect> {
    let location = font.axes().location(Vec::<(&str, f32)>::new());
    if let Some(color_glyph) = font.color_glyphs().get(GlyphId::new(glyph_id))
        && let Some(bounds) =
            color_glyph.bounding_box(LocationRef::new(location.coords()), Size::unscaled())
    {
        return Some(Rect {
            left: bounds.x_min * scale,
            top: -bounds.y_max * scale,
            right: bounds.x_max * scale,
            bottom: -bounds.y_min * scale,
        });
    }
    let glyph = font.outline_glyphs().get(GlyphId::new(glyph_id))?;
    let mut pen = ControlBoundsPen::new();
    glyph
        .draw(
            skrifa::outline::DrawSettings::unhinted(
                Size::unscaled(),
                LocationRef::new(location.coords()),
            ),
            &mut pen,
        )
        .ok()?;
    let bounds = pen.bounding_box()?;
    Some(Rect {
        left: bounds.x_min * scale,
        top: -bounds.y_max * scale,
        right: bounds.x_max * scale,
        bottom: -bounds.y_min * scale,
    })
}
