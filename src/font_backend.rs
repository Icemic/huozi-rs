use harfrust::{
    Direction, Feature, FontRef as HarfRustFontRef, ShaperData, ShaperInstance, Tag, UnicodeBuffer,
};
use log::warn;
use skrifa::attribute::{Attributes, Style};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::pen::ControlBoundsPen;
use skrifa::outline::{DrawError, DrawSettings, OutlinePen};
use skrifa::raw::TableProvider;
use skrifa::string::StringId;
use skrifa::{FontRef as SkrifaFontRef, GlyphId, MetadataProvider};
use std::collections::HashMap;
use std::sync::Arc;
use tiqian::common::HashSet;
use tiqian::core::font_face::{FontFaceId, FontVariationInstance, FontVariationSetting};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FontFaceStyle {
    Normal,
    Italic,
    Oblique,
}

impl From<Style> for FontFaceStyle {
    fn from(style: Style) -> Self {
        match style {
            Style::Normal => Self::Normal,
            Style::Italic => Self::Italic,
            Style::Oblique(_) => Self::Oblique,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FontVariationAxis {
    min: f32,
    default: f32,
    max: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct FontVariationAxes {
    weight: Option<FontVariationAxis>,
    italic: Option<FontVariationAxis>,
    slant: Option<FontVariationAxis>,
}

#[derive(Clone)]
struct FontFaceRecord {
    id: FontFaceId,
    source_index: usize,
    collection_index: u32,
    bytes: Arc<[u8]>,
    family_names: Vec<String>,
    weight: i32,
    style: FontFaceStyle,
    variation_axes: FontVariationAxes,
    units_per_em: u16,
    ascent: i16,
    descent: i16,
    leading: i16,
    typo_ascent: Option<i16>,
    typo_descent: Option<i16>,
}

struct FontCandidate<'a> {
    face: &'a FontFaceRecord,
    id: FontFaceId,
    weight: i32,
    style: FontFaceStyle,
}

#[derive(Clone)]
pub(crate) struct HuoziFontManager {
    faces: Arc<[FontFaceRecord]>,
    face_indices: Arc<HashMap<String, HashMap<u32, usize>>>,
    family_indices: Arc<HashMap<String, Vec<usize>>>,
    fallback_families: Arc<[Vec<usize>]>,
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
            let source_alias = source.alias;
            let source_label = source_alias
                .clone()
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
                let mut family_names = font_family_names(&font);
                if let Some(alias) = source_alias.as_deref() {
                    push_unique_family_name(&mut family_names, alias);
                }
                if family_names.is_empty() {
                    family_names.push(source_label.clone());
                }
                let weight = attributes.weight.value().round() as i32;
                let style = FontFaceStyle::from(attributes.style);
                let record = FontFaceRecord {
                    id: id.clone(),
                    source_index,
                    collection_index,
                    bytes: bytes.clone(),
                    family_names: family_names.clone(),
                    weight,
                    style,
                    variation_axes: font_variation_axes(&font),
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
                        family_names.into_iter().collect(),
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
                    .weight(weight)
                    .italic(style != FontFaceStyle::Normal)
                    .build(),
                );
                faces.push(record);
            }
        }

        if faces.is_empty() {
            return Err("no valid font faces".to_owned());
        }
        let (family_indices, fallback_families) = build_family_indices(&faces);
        let mut face_indices: HashMap<String, HashMap<u32, usize>> = HashMap::new();
        for (index, face) in faces.iter().enumerate() {
            face_indices
                .entry(face.id.resource_id().to_owned())
                .or_default()
                .insert(face.collection_index, index);
        }
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
            family_indices: Arc::new(family_indices),
            fallback_families: fallback_families.into(),
            descriptors: descriptors.into(),
            descriptor_indices: Arc::new(descriptor_indices),
            capability_report: Arc::new(capability_report),
        })
    }

    fn face_for_id(&self, id: &FontFaceId) -> &FontFaceRecord {
        &self.faces[*self
            .face_indices
            .get(id.resource_id())
            .and_then(|indices| indices.get(&id.collection_index()))
            .expect("HuoziFontManager received an unregistered FontFaceId")]
    }

    fn candidates(&self, request: &FontBackendRequest) -> Vec<FontCandidate<'_>> {
        let requested_weight = request.style.font_weight.clamp(1, 1000);
        let mut family_groups = Vec::new();
        let mut selected_faces = vec![false; self.faces.len()];

        for requested_family in &request.style.font_families {
            let key = normalize_family_name(requested_family);
            let Some(indices) = self.family_indices.get(&key) else {
                continue;
            };
            let mut group = Vec::with_capacity(indices.len());
            for &index in indices {
                if !selected_faces[index] {
                    selected_faces[index] = true;
                    group.push(index);
                }
            }
            if !group.is_empty() {
                family_groups.push(group);
            }
        }

        if family_groups.is_empty() {
            family_groups.extend(self.fallback_families.iter().cloned());
        }

        let style_order = if request.style.italic {
            [
                FontFaceStyle::Italic,
                FontFaceStyle::Oblique,
                FontFaceStyle::Normal,
            ]
        } else {
            [
                FontFaceStyle::Normal,
                FontFaceStyle::Oblique,
                FontFaceStyle::Italic,
            ]
        };
        let mut candidates = Vec::new();
        for group in family_groups {
            let mut style_buckets: [Vec<FontCandidate<'_>>; 3] =
                std::array::from_fn(|_| Vec::new());
            for index in group {
                let candidate =
                    font_candidate(&self.faces[index], requested_weight, request.style.italic);
                let style_index = style_order
                    .iter()
                    .position(|style| *style == candidate.style)
                    .expect("candidate style must belong to a style tier");
                style_buckets[style_index].push(candidate);
            }
            for bucket in style_buckets {
                append_candidates_by_weight(&mut candidates, bucket, requested_weight);
            }
        }
        candidates
    }

    fn raw_metrics(&self, id: &FontFaceId, font_size: f32) -> RawFontMetrics {
        let face = self.face_for_id(id);
        let scale = font_size / face.units_per_em as f32;
        let font = SkrifaFontRef::from_index(&face.bytes, face.collection_index)
            .expect("registered face must remain readable by SkRifa");
        let location = variation_location(&font, id.variation_instance());
        let instance_metrics = font.metrics(Size::new(font_size), &location);
        let default_metrics = font.metrics(Size::new(font_size), LocationRef::default());
        let ascent_delta = instance_metrics.ascent - default_metrics.ascent;
        let descent_delta = instance_metrics.descent - default_metrics.descent;
        let leading_delta = instance_metrics.leading - default_metrics.leading;
        RawFontMetrics {
            ascent: face.ascent as f32 * scale + ascent_delta,
            descent: -(face.descent as f32 * scale + descent_delta),
            leading: face.leading as f32 * scale + leading_delta,
            source: FontMetricSource::RawTables,
            typo_ascent: face
                .typo_ascent
                .map(|value| value as f32 * scale + ascent_delta),
            typo_descent: face
                .typo_descent
                .map(|value| -(value as f32 * scale + descent_delta)),
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
        glyph_ink_bounds(&font, id.variation_instance(), glyph_id, scale)
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
                LocationRef::new(variation_location(&font, id.variation_instance()).coords()),
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
        candidate: &FontCandidate<'_>,
    ) -> (ShapingResult, u32) {
        let face = candidate.face;
        let font = HarfRustFontRef::from_index(&face.bytes, face.collection_index)
            .expect("registered face must remain readable by HarfRust");
        let data = ShaperData::new(&font);
        let variation_settings: Vec<_> = candidate
            .id
            .variation_instance()
            .settings()
            .iter()
            .map(|setting| (Tag::new(&feature_tag(setting.tag())), setting.value()))
            .collect();
        let instance = ShaperInstance::from_variations(&font, variation_settings);
        let shaper = data.shaper(&font).instance(Some(&instance)).build();
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
                .render_font_face(Some(candidate.id.clone()))
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
            Some(candidate.id.clone()),
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
            candidate.id.clone(),
            advance,
        );
        let run = GlyphRun::with_open_type_features(
            request.range,
            candidate.id.clone(),
            glyphs,
            advance,
            request.open_type_features.clone(),
        );
        (
            ShapingResult::with_decisions(vec![cluster], vec![run], vec![decision]),
            missing_glyphs,
        )
    }

    fn add_ink_bounds(
        &self,
        shaping: &mut ShapingResult,
        face: &FontFaceRecord,
        id: &FontFaceId,
        font_size: f32,
    ) {
        let font = SkrifaFontRef::from_index(&face.bytes, face.collection_index)
            .expect("registered face must remain readable by SkRifa");
        let scale = font_size / face.units_per_em as f32;
        let mut glyphs_without_ink_bounds = 0;
        for glyph in shaping
            .glyph_runs
            .iter_mut()
            .flat_map(|run| &mut run.glyphs)
        {
            glyph.bounds = glyph_ink_bounds(&font, id.variation_instance(), glyph.id, scale);
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
        for candidate in self.candidates(request) {
            let (mut shaping, missing_glyphs) = self.shape_one_face(request, &candidate);
            attempts.push(FontCandidateAttempt::new(
                format!(
                    "source-{}#{}@{}",
                    candidate.face.source_index,
                    candidate.face.collection_index,
                    candidate.id.variation_instance()
                ),
                candidate.id.clone(),
                missing_glyphs,
            ));
            if missing_glyphs == 0 {
                self.add_ink_bounds(
                    &mut shaping,
                    candidate.face,
                    &candidate.id,
                    request.style.font_size,
                );
                return FontBackendShapingResult::new(candidate.id, shaping, attempts);
            }
            if preferred.is_none() {
                preferred = Some((candidate.id, candidate.face, shaping));
                continue;
            }
        }
        let (face_id, face, mut shaping) =
            preferred.expect("HuoziFontManager must contain at least one face");
        self.add_ink_bounds(&mut shaping, face, &face_id, request.style.font_size);
        FontBackendShapingResult::new(face_id, shaping, attempts)
    }

    fn metrics(&self, request: &FontMetricsRequest) -> RawFontMetrics {
        self.raw_metrics(&request.face, request.font_size)
    }
}

fn font_family_names(font: &SkrifaFontRef<'_>) -> Vec<String> {
    const TYPOGRAPHIC_FAMILY_NAME_ID: u16 = 16;
    const FAMILY_NAME_ID: u16 = 1;
    const WWS_FAMILY_NAME_ID: u16 = 21;

    let mut names = Vec::new();
    for name_id in [
        TYPOGRAPHIC_FAMILY_NAME_ID,
        FAMILY_NAME_ID,
        WWS_FAMILY_NAME_ID,
    ] {
        for localized in font.localized_strings(StringId::new(name_id)) {
            push_unique_family_name(&mut names, &localized.to_string());
        }
    }
    names
}

fn build_family_indices(
    faces: &[FontFaceRecord],
) -> (HashMap<String, Vec<usize>>, Vec<Vec<usize>>) {
    let mut family_indices: HashMap<String, Vec<usize>> = HashMap::new();
    let mut fallback_families = Vec::new();
    let mut fallback_family_indices = HashMap::new();

    for (face_index, face) in faces.iter().enumerate() {
        for name in &face.family_names {
            family_indices
                .entry(normalize_family_name(name))
                .or_default()
                .push(face_index);
        }

        let primary_name = normalize_family_name(&face.family_names[0]);
        let family_index = *fallback_family_indices
            .entry(primary_name)
            .or_insert_with(|| {
                fallback_families.push(Vec::new());
                fallback_families.len() - 1
            });
        fallback_families[family_index].push(face_index);
    }
    (family_indices, fallback_families)
}

fn normalize_family_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn push_unique_family_name(names: &mut Vec<String>, candidate: &str) {
    let candidate = candidate.trim();
    if candidate.is_empty()
        || names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(candidate))
    {
        return;
    }
    names.push(candidate.to_owned());
}

fn font_variation_axes(font: &SkrifaFontRef<'_>) -> FontVariationAxes {
    let axes = font.axes();
    FontVariationAxes {
        weight: axes
            .get_by_tag(skrifa::raw::types::Tag::new(b"wght"))
            .map(font_variation_axis),
        italic: axes
            .get_by_tag(skrifa::raw::types::Tag::new(b"ital"))
            .map(font_variation_axis),
        slant: axes
            .get_by_tag(skrifa::raw::types::Tag::new(b"slnt"))
            .map(font_variation_axis),
    }
}

fn font_variation_axis(axis: skrifa::Axis) -> FontVariationAxis {
    FontVariationAxis {
        min: axis.min_value(),
        default: axis.default_value(),
        max: axis.max_value(),
    }
}

fn font_candidate(face: &FontFaceRecord, requested_weight: i32, italic: bool) -> FontCandidate<'_> {
    let mut settings = Vec::with_capacity(2);
    let weight = if let Some(axis) = face.variation_axes.weight {
        let value = (requested_weight as f32).clamp(axis.min, axis.max);
        push_non_default_variation(&mut settings, "wght", value, axis.default);
        value.round() as i32
    } else {
        face.weight.clamp(1, 1000)
    };
    let style = if italic {
        if let Some(axis) = face.variation_axes.italic {
            let value = 1.0_f32.clamp(axis.min, axis.max);
            push_non_default_variation(&mut settings, "ital", value, axis.default);
            FontFaceStyle::Italic
        } else if let Some(axis) = face.variation_axes.slant {
            let value = (-14.0_f32).clamp(axis.min, axis.max);
            push_non_default_variation(&mut settings, "slnt", value, axis.default);
            FontFaceStyle::Oblique
        } else {
            face.style
        }
    } else {
        face.style
    };
    let variation = FontVariationInstance::new(settings);
    FontCandidate {
        face,
        id: FontFaceId::new(
            face.id.resource_id().to_owned(),
            face.collection_index,
            variation,
        ),
        weight,
        style,
    }
}

fn push_non_default_variation(
    settings: &mut Vec<FontVariationSetting>,
    tag: &str,
    value: f32,
    default: f32,
) {
    if value.to_bits() != default.to_bits() {
        settings.push(FontVariationSetting::new(tag.to_owned(), value));
    }
}

fn append_candidates_by_weight<'a>(
    output: &mut Vec<FontCandidate<'a>>,
    candidates: Vec<FontCandidate<'a>>,
    requested_weight: i32,
) {
    let mut weights: Vec<Vec<FontCandidate<'a>>> = (0..=1000).map(|_| Vec::new()).collect();
    for candidate in candidates {
        weights[candidate.weight.clamp(1, 1000) as usize].push(candidate);
    }

    if requested_weight < 400 {
        append_weight_range(output, &mut weights, (1..=requested_weight).rev());
        append_weight_range(output, &mut weights, requested_weight + 1..=1000);
    } else if requested_weight <= 500 {
        append_weight_range(output, &mut weights, requested_weight..=500);
        append_weight_range(output, &mut weights, (1..requested_weight).rev());
        append_weight_range(output, &mut weights, 501..=1000);
    } else {
        append_weight_range(output, &mut weights, requested_weight..=1000);
        append_weight_range(output, &mut weights, (1..requested_weight).rev());
    }
}

fn append_weight_range<'a>(
    output: &mut Vec<FontCandidate<'a>>,
    weights: &mut [Vec<FontCandidate<'a>>],
    range: impl Iterator<Item = i32>,
) {
    for weight in range {
        output.append(&mut weights[weight as usize]);
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

    const FIRA_CODE: &[u8] = include_bytes!("../resources/fonts/FiraCode-VF.ttf");
    const INTER: &[u8] = include_bytes!("../resources/fonts/InterVariable.ttf");
    const INTER_ITALIC: &[u8] = include_bytes!("../resources/fonts/InterVariable-Italic.ttf");
    const SOURCE_HAN_SANS: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");
    const SOURCE_HAN_SERIF_REGULAR: &[u8] =
        include_bytes!("../resources/fonts/SourceHanSerifCN-Regular.otf");
    const SOURCE_HAN_SERIF_SEMIBOLD: &[u8] =
        include_bytes!("../resources/fonts/SourceHanSerifCN-SemiBold.otf");

    fn request(text: &str) -> FontBackendRequest {
        request_with_style(text, TextStyle::default())
    }

    fn request_with_style(text: &str, style: TextStyle) -> FontBackendRequest {
        let text = Text::from(text);
        FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            style,
            FontRole::CjkText,
        )
    }

    fn variation_value(face: &FontFaceId, tag: &str) -> Option<f32> {
        face.variation_instance()
            .settings()
            .iter()
            .find(|setting| setting.tag() == tag)
            .map(FontVariationSetting::value)
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
        assert!(
            result
                .selected_attempt()
                .unwrap()
                .candidate_key
                .starts_with("source-0#0@")
        );
        assert_eq!(result.selected_attempt().unwrap().missing_glyphs, 0);
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
        assert!(result.attempts[0].candidate_key.starts_with("source-0#0@"));
        assert!(result.attempts[0].has_missing_glyphs());
        assert!(
            result
                .selected_attempt()
                .unwrap()
                .candidate_key
                .starts_with("source-1#0@")
        );
        assert_eq!(result.selected_attempt().unwrap().missing_glyphs, 0);
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
        assert!(
            result
                .selected_attempt()
                .unwrap()
                .candidate_key
                .starts_with("source-0#0@")
        );
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

    #[test]
    fn follows_requested_family_order_and_ignores_ascii_case() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "Fira Alias".to_owned()),
            FontSource::with_alias(INTER.to_vec(), "Inter Alias".to_owned()),
        ])
        .unwrap();
        let style = TextStyle::builder()
            .font_families(vec![" inter alias ".to_owned(), "fira alias".to_owned()])
            .build();

        let result = manager.shape(&request_with_style("A", style));

        assert_eq!(result.face.resource_id(), "Inter Alias:1");
        assert_eq!(result.attempts.len(), 1);
    }

    #[test]
    fn selects_unaliased_font_by_name_table_family() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::new(FIRA_CODE.to_vec()),
            FontSource::new(INTER.to_vec()),
        ])
        .unwrap();
        let style = TextStyle::builder()
            .font_families(vec![" inter variable ".to_owned()])
            .build();

        let result = manager.shape(&request_with_style("A", style));

        assert_eq!(result.face.resource_id(), "font-source-1:1");
    }

    #[test]
    fn falls_back_to_registration_order_when_no_family_matches() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned()),
            FontSource::with_alias(INTER.to_vec(), "inter".to_owned()),
        ])
        .unwrap();
        let style = TextStyle::builder()
            .font_families(vec!["missing family".to_owned()])
            .build();

        let result = manager.shape(&request_with_style("A", style));

        assert_eq!(result.face.resource_id(), "fira:0");
    }

    #[test]
    fn selects_static_face_by_css_weight_order() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(SOURCE_HAN_SERIF_REGULAR.to_vec(), "serif".to_owned()),
            FontSource::with_alias(SOURCE_HAN_SERIF_SEMIBOLD.to_vec(), "serif".to_owned()),
        ])
        .unwrap();
        let style = TextStyle::builder()
            .font_families(vec!["serif".to_owned()])
            .font_weight(700)
            .build();

        let result = manager.shape(&request_with_style("中", style));

        assert_eq!(result.face.resource_id(), "serif:1");
        assert!(result.face.variation_instance().settings().is_empty());
    }

    #[test]
    fn creates_distinct_variable_weight_instances() {
        let manager = HuoziFontManager::from_sources(vec![FontSource::with_alias(
            INTER.to_vec(),
            "inter".to_owned(),
        )])
        .unwrap();
        let light = TextStyle::builder().font_weight(300).build();
        let bold = TextStyle::builder().font_weight(700).build();

        let light_result = manager.shape(&request_with_style("A", light));
        let bold_result = manager.shape(&request_with_style("A", bold));

        assert_eq!(variation_value(&light_result.face, "wght"), Some(300.0));
        assert_eq!(variation_value(&bold_result.face, "wght"), Some(700.0));
        assert_ne!(light_result.face, bold_result.face);
    }

    #[test]
    fn keeps_non_synthetic_face_when_italic_capability_is_missing() {
        let manager = HuoziFontManager::from_sources(vec![FontSource::with_alias(
            INTER.to_vec(),
            "inter".to_owned(),
        )])
        .unwrap();
        let normal = manager.shape(&request("A"));
        let italic = manager.shape(&request_with_style(
            "A",
            TextStyle::builder().italic(true).build(),
        ));

        assert_eq!(italic.face, normal.face);
    }

    #[test]
    fn selects_italic_face_and_applies_variable_weight() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(INTER.to_vec(), "inter".to_owned()),
            FontSource::with_alias(INTER_ITALIC.to_vec(), "inter".to_owned()),
        ])
        .unwrap();
        let normal = manager.shape(&request_with_style(
            "A",
            TextStyle::builder()
                .font_families(vec!["inter".to_owned()])
                .font_weight(700)
                .build(),
        ));
        let italic = manager.shape(&request_with_style(
            "A",
            TextStyle::builder()
                .font_families(vec!["inter".to_owned()])
                .font_weight(700)
                .italic(true)
                .build(),
        ));

        assert_eq!(normal.face.resource_id(), "inter:0");
        assert_eq!(italic.face.resource_id(), "inter:1");
        assert_eq!(variation_value(&italic.face, "wght"), Some(700.0));
        assert_ne!(normal.face, italic.face);
    }

    #[test]
    fn prefers_ital_axis_and_leaves_slant_at_default() {
        let face = test_face_with_axes(FontVariationAxes {
            weight: None,
            italic: Some(FontVariationAxis {
                min: 0.0,
                default: 0.0,
                max: 1.0,
            }),
            slant: Some(FontVariationAxis {
                min: -10.0,
                default: 0.0,
                max: 0.0,
            }),
        });

        let candidate = font_candidate(&face, 400, true);

        assert_eq!(variation_value(&candidate.id, "ital"), Some(1.0));
        assert_eq!(variation_value(&candidate.id, "slnt"), None);
        assert_eq!(candidate.style, FontFaceStyle::Italic);
    }

    #[test]
    fn clamps_slant_axis_when_ital_axis_is_missing() {
        let face = test_face_with_axes(FontVariationAxes {
            weight: None,
            italic: None,
            slant: Some(FontVariationAxis {
                min: -10.0,
                default: 0.0,
                max: 0.0,
            }),
        });

        let candidate = font_candidate(&face, 400, true);

        assert_eq!(variation_value(&candidate.id, "slnt"), Some(-10.0));
        assert_eq!(candidate.style, FontFaceStyle::Oblique);
    }

    fn test_face_with_axes(variation_axes: FontVariationAxes) -> FontFaceRecord {
        FontFaceRecord {
            id: FontFaceId::with_resource_id("test-face"),
            source_index: 0,
            collection_index: 0,
            bytes: Arc::from([]),
            family_names: vec!["test".to_owned()],
            weight: 400,
            style: FontFaceStyle::Normal,
            variation_axes,
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            leading: 0,
            typo_ascent: Some(800),
            typo_descent: Some(-200),
        }
    }
}

fn variation_location<'a>(
    font: &SkrifaFontRef<'a>,
    variation: &FontVariationInstance,
) -> skrifa::instance::Location {
    font.axes().location(
        variation
            .settings()
            .iter()
            .map(|setting| (setting.tag(), setting.value())),
    )
}

fn glyph_ink_bounds(
    font: &SkrifaFontRef<'_>,
    variation: &FontVariationInstance,
    glyph_id: u32,
    scale: f32,
) -> Option<Rect> {
    let location = variation_location(font, variation);
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
