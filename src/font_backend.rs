use harfrust::{
    Direction, Feature, FontRef as HarfRustFontRef, ShaperData, ShaperInstance, Tag, UnicodeBuffer,
};
use log::warn;
use lru::LruCache;
use skrifa::attribute::{Attributes, Style};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::pen::ControlBoundsPen;
use skrifa::outline::{DrawError, DrawSettings, OutlinePen};
use skrifa::raw::TableProvider;
use skrifa::string::StringId;
use skrifa::{FontRef as SkrifaFontRef, GlyphId, MetadataProvider};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tiqian::common::HashSet;
use tiqian::core::font_face::{
    FontFaceId, FontSynthesisInstance, FontVariationInstance, FontVariationSetting,
};
use tiqian::core::geometry::Rect;
use tiqian::core::layout_model::{Cluster, Glyph, GlyphRun, ShapingDecisionInfo};
use tiqian::core::text::Text;
use tiqian::core::text_model::FontSynthesis;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontSourceKind {
    Cjk,
    Western,
}

#[derive(Clone, Debug)]
pub struct FontSource {
    pub bytes: Vec<u8>,
    pub alias: Option<String>,
    pub kind: Option<FontSourceKind>,
}

impl FontSource {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            alias: None,
            kind: None,
        }
    }

    pub fn with_alias(bytes: Vec<u8>, alias: String) -> Self {
        Self {
            bytes,
            alias: Some(alias),
            kind: None,
        }
    }

    pub fn with_kind(mut self, kind: FontSourceKind) -> Self {
        self.kind = Some(kind);
        self
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

struct FontFaceRecord {
    id: FontFaceId,
    source_index: usize,
    collection_index: u32,
    kind: Option<FontSourceKind>,
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
    shaper_data: ShaperData,
}

const INK_BOUNDS_CACHE_CAPACITY: usize = 1024;
const SYNTHETIC_BOLD_WEIGHT_THRESHOLD: i32 = 600;
const SYNTHETIC_EMBOLDEN_EM: f32 = 1. / 60.;
const SYNTHETIC_OBLIQUE_DEGREES: f32 = 14.0;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct InkBoundsKey {
    face: FontFaceId,
    glyph_id: u32,
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
    ink_bounds_cache: Arc<Mutex<LruCache<InkBoundsKey, Option<Rect>>>>,
}

impl HuoziFontManager {
    pub(crate) fn from_sources(sources: Vec<FontSource>) -> Result<Self, String> {
        let mut faces = Vec::new();
        let mut descriptors = Vec::new();

        for (source_index, source) in sources.into_iter().enumerate() {
            let bytes: Arc<[u8]> = match decode_font_source(source.bytes) {
                Ok(bytes) => bytes.into(),
                Err(error) => {
                    warn!("skip invalid compressed font source {source_index}: {error}");
                    continue;
                }
            };
            let collection_len = match skrifa::raw::FileRef::new(&bytes) {
                Ok(skrifa::raw::FileRef::Font(_)) => 1,
                Ok(skrifa::raw::FileRef::Collection(collection)) => collection.len(),
                Err(error) => {
                    warn!("skip invalid font source {source_index}: {error:?}");
                    continue;
                }
            };
            let source_alias = source.alias;
            let source_kind = source.kind;
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
                let harfrust_font = HarfRustFontRef::from_index(&bytes, collection_index)
                    .expect("SkRifa 可读取的已注册字体应当也可供 HarfRust 读取");
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
                    kind: source_kind,
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
                    shaper_data: ShaperData::new(&harfrust_font),
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
        let ink_bounds_cache = LruCache::new(
            NonZeroUsize::new(INK_BOUNDS_CACHE_CAPACITY).expect("ink bounds 缓存容量必须大于零"),
        );
        Ok(Self {
            faces: faces.into(),
            face_indices: Arc::new(face_indices),
            family_indices: Arc::new(family_indices),
            fallback_families: fallback_families.into(),
            descriptors: descriptors.into(),
            descriptor_indices: Arc::new(descriptor_indices),
            capability_report: Arc::new(capability_report),
            ink_bounds_cache: Arc::new(Mutex::new(ink_bounds_cache)),
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
        let target_kind = match request.role {
            FontRole::CjkText | FontRole::CjkPunctuation => Some(FontSourceKind::Cjk),
            FontRole::LatinText => Some(FontSourceKind::Western),
            FontRole::Symbol | FontRole::Emoji | FontRole::Unknown => None,
        };

        for pass in 0..if target_kind.is_some() { 2 } else { 1 } {
            for group in &family_groups {
                let mut style_buckets: [Vec<FontCandidate<'_>>; 3] =
                    std::array::from_fn(|_| Vec::new());
                for &index in group {
                    let face = &self.faces[index];
                    if let Some(kind) = target_kind
                        && (pass == 0) != (face.kind == Some(kind))
                    {
                        continue;
                    }
                    let candidate = font_candidate(
                        face,
                        requested_weight,
                        request.style.italic,
                        request.style.font_synthesis,
                    );
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
        let key = InkBoundsKey {
            face: id.clone(),
            glyph_id,
        };
        let mut cache = self
            .ink_bounds_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(bounds) = cache.get(&key) {
            return bounds.map(|bounds| self.scale_ink_bounds(id, bounds, font_size));
        }
        let bounds = self.glyph_ink_bounds_uncached(id, glyph_id);
        cache.put(key, bounds);
        bounds.map(|bounds| self.scale_ink_bounds(id, bounds, font_size))
    }

    fn glyph_ink_bounds_uncached(&self, id: &FontFaceId, glyph_id: u32) -> Option<Rect> {
        let face = self.face_for_id(id);
        let font = SkrifaFontRef::from_index(&face.bytes, face.collection_index).ok()?;
        glyph_ink_bounds(&font, id, glyph_id, 1.0)
    }

    fn scale_ink_bounds(&self, id: &FontFaceId, bounds: Rect, font_size: f32) -> Rect {
        let scale = font_size / self.face_for_id(id).units_per_em as f32;
        let mut bounds = Rect {
            left: bounds.left * scale,
            top: bounds.top * scale,
            right: bounds.right * scale,
            bottom: bounds.bottom * scale,
        };
        if let Some(embolden_em) = id.synthesis().embolden_em() {
            let expansion = embolden_em * font_size;
            bounds.left -= expansion;
            bounds.top -= expansion;
            bounds.right += expansion;
            bounds.bottom += expansion;
        }
        bounds
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
        let location = variation_location(&font, id.variation_instance());
        let settings =
            DrawSettings::unhinted(Size::new(font_size), LocationRef::new(location.coords()));
        if let Some(oblique_degrees) = id.synthesis().oblique_degrees() {
            let mut shearing_pen = ShearingPen::new(pen, oblique_degrees);
            glyph.draw(settings, &mut shearing_pen)?;
        } else {
            glyph.draw(settings, pen)?;
        }
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
        let variation_settings: Vec<_> = candidate
            .id
            .variation_instance()
            .settings()
            .iter()
            .map(|setting| (Tag::new(&feature_tag(setting.tag())), setting.value()))
            .collect();
        let instance = ShaperInstance::from_variations(&font, variation_settings);
        let shaper = face
            .shaper_data
            .shaper(&font)
            .instance(Some(&instance))
            .build();
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

    fn add_ink_bounds(&self, shaping: &mut ShapingResult, id: &FontFaceId, font_size: f32) {
        let mut cache = self
            .ink_bounds_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut glyphs_without_ink_bounds = 0;
        for glyph in shaping
            .glyph_runs
            .iter_mut()
            .flat_map(|run| &mut run.glyphs)
        {
            let key = InkBoundsKey {
                face: id.clone(),
                glyph_id: glyph.id,
            };
            glyph.bounds = if let Some(bounds) = cache.get(&key) {
                bounds.map(|bounds| self.scale_ink_bounds(id, bounds, font_size))
            } else {
                let bounds = self.glyph_ink_bounds_uncached(id, glyph.id);
                cache.put(key, bounds);
                bounds.map(|bounds| self.scale_ink_bounds(id, bounds, font_size))
            };
            glyphs_without_ink_bounds += i32::from(glyph.bounds.is_none());
        }
        for decision in &mut shaping.decisions {
            decision.glyphs_without_ink_bounds = glyphs_without_ink_bounds;
        }
    }
}

#[cfg(feature = "woff")]
fn decode_font_source(bytes: Vec<u8>) -> Result<Vec<u8>, wuff::WuffErr> {
    match bytes.get(..4) {
        Some(b"wOFF") => wuff::decompress_woff1(&bytes),
        Some(b"wOF2") => wuff::decompress_woff2(&bytes),
        _ => Ok(bytes),
    }
}

#[cfg(not(feature = "woff"))]
fn decode_font_source(bytes: Vec<u8>) -> Result<Vec<u8>, std::convert::Infallible> {
    Ok(bytes)
}

impl ReplayableFontCatalog for HuoziFontManager {
    fn faces(&self) -> &[ReplayableFontFaceDescriptor] {
        &self.descriptors
    }

    fn capability_report(&self) -> &FontBackendCapabilityReport {
        &self.capability_report
    }

    fn face(&self, id: &FontFaceId) -> Option<&ReplayableFontFaceDescriptor> {
        let face_index = self
            .face_indices
            .get(id.resource_id())
            .and_then(|indices| indices.get(&id.collection_index()))?;
        self.descriptor_indices
            .get(&self.faces[*face_index].id)
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
                self.add_ink_bounds(&mut shaping, &candidate.id, request.style.font_size);
                return FontBackendShapingResult::new(candidate.id, shaping, attempts);
            }
            if preferred.is_none() {
                preferred = Some((candidate.id, candidate.face, shaping));
                continue;
            }
        }
        let (face_id, _, mut shaping) =
            preferred.expect("HuoziFontManager must contain at least one face");
        self.add_ink_bounds(&mut shaping, &face_id, request.style.font_size);
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

fn font_candidate(
    face: &FontFaceRecord,
    requested_weight: i32,
    italic: bool,
    font_synthesis: FontSynthesis,
) -> FontCandidate<'_> {
    let mut settings = Vec::new();
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
            if value.to_bits() != axis.default.to_bits() {
                FontFaceStyle::Italic
            } else {
                face.style
            }
        } else if let Some(axis) = face.variation_axes.slant {
            let value = (-14.0_f32).clamp(axis.min, axis.max);
            push_non_default_variation(&mut settings, "slnt", value, axis.default);
            if value.to_bits() != axis.default.to_bits() {
                FontFaceStyle::Oblique
            } else {
                face.style
            }
        } else {
            face.style
        }
    } else {
        face.style
    };
    let physical_id = if settings.is_empty() {
        face.id.clone()
    } else {
        FontFaceId::new(
            face.id.resource_id().to_owned(),
            face.collection_index,
            FontVariationInstance::new(settings),
        )
    };
    let needs_synthetic_weight = requested_weight >= SYNTHETIC_BOLD_WEIGHT_THRESHOLD
        && weight < SYNTHETIC_BOLD_WEIGHT_THRESHOLD
        && font_synthesis.contains(FontSynthesis::WEIGHT);
    let needs_synthetic_style =
        italic && style == FontFaceStyle::Normal && font_synthesis.contains(FontSynthesis::STYLE);
    let id = if needs_synthetic_weight || needs_synthetic_style {
        physical_id.with_synthesis(FontSynthesisInstance::new(
            needs_synthetic_weight.then_some(SYNTHETIC_EMBOLDEN_EM),
            needs_synthetic_style.then_some(SYNTHETIC_OBLIQUE_DEGREES),
        ))
    } else {
        physical_id
    };
    FontCandidate {
        face,
        id,
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
    mut candidates: Vec<FontCandidate<'a>>,
    requested_weight: i32,
) {
    candidates.sort_by_key(|candidate| {
        let weight = candidate.weight.clamp(1, 1000);
        if requested_weight < 400 {
            if weight <= requested_weight {
                (0, requested_weight - weight)
            } else {
                (1, weight - requested_weight)
            }
        } else if requested_weight <= 500 {
            if weight >= requested_weight && weight <= 500 {
                (0, weight - requested_weight)
            } else if weight < requested_weight {
                (1, requested_weight - weight)
            } else {
                (2, weight - 501)
            }
        } else if weight >= requested_weight {
            (0, weight - requested_weight)
        } else {
            (1, requested_weight - weight)
        }
    });
    output.append(&mut candidates);
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
    use std::collections::HashMap as StdHashMap;
    use tiqian::core::font_face::FontSynthesisInstance;
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
    #[cfg(feature = "woff")]
    const SOURCE_HAN_SERIF_WOFF2: &[u8] =
        include_bytes!("../resources/fonts/SourceHanSerif-VF.otf.woff2");

    fn request(text: &str) -> FontBackendRequest {
        request_with_style(text, TextStyle::default())
    }

    fn request_with_style(text: &str, style: TextStyle) -> FontBackendRequest {
        request_with_role_and_style(text, FontRole::CjkText, style)
    }

    fn request_with_role(text: &str, role: FontRole) -> FontBackendRequest {
        request_with_role_and_style(text, role, TextStyle::default())
    }

    fn request_with_role_and_style(
        text: &str,
        role: FontRole,
        style: TextStyle,
    ) -> FontBackendRequest {
        let text = Text::from(text);
        FontBackendRequest::new(
            text.clone(),
            text_range(0, text.scalar_len().value()),
            style,
            role,
        )
    }

    fn variation_value(face: &FontFaceId, tag: &str) -> Option<f32> {
        face.variation_instance()
            .settings()
            .iter()
            .find(|setting| setting.tag() == tag)
            .map(FontVariationSetting::value)
    }

    #[derive(Default)]
    struct RecordingPen {
        points: Vec<(f32, f32)>,
        close_count: usize,
    }

    impl OutlinePen for RecordingPen {
        fn move_to(&mut self, x: f32, y: f32) {
            self.points.push((x, y));
        }

        fn line_to(&mut self, x: f32, y: f32) {
            self.points.push((x, y));
        }

        fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
            self.points.extend([(cx0, cy0), (x, y)]);
        }

        fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
            self.points.extend([(cx0, cy0), (cx1, cy1), (x, y)]);
        }

        fn close(&mut self) {
            self.close_count += 1;
        }
    }

    #[test]
    fn shearing_pen_transforms_endpoints_and_bezier_controls() {
        let mut recording = RecordingPen::default();
        let mut pen = ShearingPen::new(&mut recording, 45.0);

        pen.move_to(1.0, 2.0);
        pen.line_to(3.0, 4.0);
        pen.quad_to(5.0, 6.0, 7.0, 8.0);
        pen.curve_to(9.0, 10.0, 11.0, 12.0, 13.0, 14.0);
        pen.close();

        assert_eq!(
            recording.points,
            vec![
                (3.0, 2.0),
                (7.0, 4.0),
                (11.0, 6.0),
                (15.0, 8.0),
                (19.0, 10.0),
                (23.0, 12.0),
                (27.0, 14.0),
            ]
        );
        assert_eq!(recording.close_count, 1);
    }

    #[cfg(feature = "woff")]
    #[test]
    fn loads_woff2_font_source() {
        let manager =
            HuoziFontManager::from_sources(vec![FontSource::new(SOURCE_HAN_SERIF_WOFF2.to_vec())])
                .unwrap();

        assert!(!manager.faces.is_empty());
        assert!(manager.faces.iter().any(|face| {
            face.family_names
                .iter()
                .any(|name| name.contains("Source Han Serif"))
        }));
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
    fn prioritizes_cjk_kind_for_cjk_text_and_punctuation() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned())
                .with_kind(FontSourceKind::Western),
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned())
                .with_kind(FontSourceKind::Cjk),
        ])
        .unwrap();

        let text = manager.shape(&request_with_role("A", FontRole::CjkText));
        let punctuation = manager.shape(&request_with_role("“", FontRole::CjkPunctuation));

        assert_eq!(text.face.resource_id(), "source-han:1");
        assert_eq!(punctuation.face.resource_id(), "source-han:1");
        assert_eq!(text.attempts.len(), 1);
        assert_eq!(punctuation.attempts.len(), 1);
    }

    #[test]
    fn prioritizes_western_kind_for_latin_text() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned())
                .with_kind(FontSourceKind::Cjk),
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned())
                .with_kind(FontSourceKind::Western),
        ])
        .unwrap();

        let result = manager.shape(&request_with_role("A", FontRole::LatinText));

        assert_eq!(result.face.resource_id(), "fira:1");
        assert_eq!(result.attempts.len(), 1);
    }

    #[test]
    fn prioritizes_kind_within_explicit_family_list() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned())
                .with_kind(FontSourceKind::Western),
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned())
                .with_kind(FontSourceKind::Cjk),
        ])
        .unwrap();
        let cjk_style = TextStyle::builder()
            .font_families(vec!["fira".to_owned(), "source-han".to_owned()])
            .build();
        let latin_style = TextStyle::builder()
            .font_families(vec!["source-han".to_owned(), "fira".to_owned()])
            .build();

        let cjk = manager.shape(&request_with_role_and_style(
            "A",
            FontRole::CjkText,
            cjk_style,
        ));
        let latin = manager.shape(&request_with_role_and_style(
            "A",
            FontRole::LatinText,
            latin_style,
        ));

        assert_eq!(cjk.face.resource_id(), "source-han:1");
        assert_eq!(latin.face.resource_id(), "fira:0");
    }

    #[test]
    fn keeps_explicit_single_family_from_leaking_to_other_families() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned())
                .with_kind(FontSourceKind::Western),
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned())
                .with_kind(FontSourceKind::Cjk),
        ])
        .unwrap();
        let style = TextStyle::builder()
            .font_families(vec!["fira".to_owned()])
            .build();

        let result = manager.shape(&request_with_role_and_style("中", FontRole::CjkText, style));

        assert_eq!(result.face.resource_id(), "fira:0");
        assert_eq!(result.attempts.len(), 1);
        assert!(result.attempts[0].has_missing_glyphs());
    }

    #[test]
    fn keeps_base_order_for_roles_without_a_target_kind() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned())
                .with_kind(FontSourceKind::Western),
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned())
                .with_kind(FontSourceKind::Cjk),
        ])
        .unwrap();

        let result = manager.shape(&request_with_role("A", FontRole::Emoji));

        assert_eq!(result.face.resource_id(), "fira:0");
        assert_eq!(result.attempts.len(), 1);
    }

    #[test]
    fn falls_back_once_after_preferred_kind_is_missing() {
        let manager = HuoziFontManager::from_sources(vec![
            FontSource::with_alias(FIRA_CODE.to_vec(), "fira".to_owned())
                .with_kind(FontSourceKind::Cjk),
            FontSource::with_alias(SOURCE_HAN_SANS.to_vec(), "source-han".to_owned()),
        ])
        .unwrap();

        let result = manager.shape(&request_with_role("中", FontRole::CjkText));

        assert_eq!(result.face.resource_id(), "source-han:1");
        assert_eq!(result.attempts.len(), 2);
        assert!(result.attempts[0].candidate_key.starts_with("source-0#0@"));
        assert!(result.attempts[0].has_missing_glyphs());
        assert!(result.attempts[1].candidate_key.starts_with("source-1#0@"));
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
    fn optimized_weight_order_matches_css_range_order() {
        let weights = [1, 100, 250, 399, 400, 450, 500, 501, 650, 900, 1000];
        for requested_weight in 1..=1000 {
            let mut optimized = weights;
            optimized.sort_by_key(|weight| {
                if requested_weight < 400 {
                    if *weight <= requested_weight {
                        (0, requested_weight - *weight)
                    } else {
                        (1, *weight - requested_weight)
                    }
                } else if requested_weight <= 500 {
                    if *weight >= requested_weight && *weight <= 500 {
                        (0, *weight - requested_weight)
                    } else if *weight < requested_weight {
                        (1, requested_weight - *weight)
                    } else {
                        (2, *weight - 501)
                    }
                } else if *weight >= requested_weight {
                    (0, *weight - requested_weight)
                } else {
                    (1, requested_weight - *weight)
                }
            });

            let mut by_weight: StdHashMap<i32, i32> =
                StdHashMap::from_iter(weights.map(|weight| (weight, weight)));
            let ranges: Vec<Box<dyn Iterator<Item = i32>>> = if requested_weight < 400 {
                vec![
                    Box::new((1..=requested_weight).rev()),
                    Box::new(requested_weight + 1..=1000),
                ]
            } else if requested_weight <= 500 {
                vec![
                    Box::new(requested_weight..=500),
                    Box::new((1..requested_weight).rev()),
                    Box::new(501..=1000),
                ]
            } else {
                vec![
                    Box::new(requested_weight..=1000),
                    Box::new((1..requested_weight).rev()),
                ]
            };
            let expected = ranges
                .into_iter()
                .flatten()
                .filter_map(|weight| by_weight.remove(&weight))
                .collect::<Vec<_>>();

            assert_eq!(optimized.as_slice(), expected.as_slice());
        }
    }

    #[test]
    fn ink_bounds_cache_evicts_at_fixed_capacity() {
        let manager =
            HuoziFontManager::from_sources(vec![FontSource::new(INTER.to_vec())]).unwrap();
        let face = manager.faces[0].id.clone();
        let mut cache = manager
            .ink_bounds_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for glyph_id in 0..=INK_BOUNDS_CACHE_CAPACITY as u32 {
            cache.put(
                InkBoundsKey {
                    face: face.clone(),
                    glyph_id,
                },
                None,
            );
        }

        assert_eq!(cache.len(), INK_BOUNDS_CACHE_CAPACITY);
    }

    #[test]
    fn ink_bounds_cache_reuses_unscaled_bounds_across_font_sizes() {
        let manager =
            HuoziFontManager::from_sources(vec![FontSource::new(INTER.to_vec())]).unwrap();
        let face = manager.faces[0].id.clone();
        let glyph_id = 36;

        let small = manager.glyph_ink_bounds(&face, glyph_id, 24.0).unwrap();
        let large = manager.glyph_ink_bounds(&face, glyph_id, 48.0).unwrap();

        assert_eq!(large.left, small.left * 2.0);
        assert_eq!(large.top, small.top * 2.0);
        assert_eq!(large.right, small.right * 2.0);
        assert_eq!(large.bottom, small.bottom * 2.0);
        assert_eq!(
            manager
                .ink_bounds_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len(),
            1
        );
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
        assert_eq!(
            bold_result.face.synthesis(),
            &FontSynthesisInstance::default()
        );
    }

    #[test]
    fn replays_synthetic_identity_with_its_physical_face_descriptor() {
        let manager = HuoziFontManager::from_sources(vec![FontSource::with_alias(
            INTER.to_vec(),
            "inter".to_owned(),
        )])
        .unwrap();
        let physical = manager.faces[0].id.clone();
        let synthetic = physical.with_synthesis(FontSynthesisInstance::new(
            Some(SYNTHETIC_EMBOLDEN_EM),
            Some(14.0),
        ));

        let descriptor = manager
            .face(&synthetic)
            .expect("synthetic identity must resolve its physical descriptor");

        assert_eq!(descriptor.id, physical);
    }

    #[test]
    fn synthetic_bold_expands_ink_bounds_without_changing_advance() {
        let manager = HuoziFontManager::from_sources(vec![FontSource::with_alias(
            SOURCE_HAN_SERIF_REGULAR.to_vec(),
            "serif".to_owned(),
        )])
        .unwrap();
        let normal = manager.shape(&request_with_style(
            "中",
            TextStyle::builder().font_size(96.0).build(),
        ));
        let bold = manager.shape(&request_with_style(
            "中",
            TextStyle::builder()
                .font_size(96.0)
                .font_weight(700)
                .build(),
        ));
        let normal_glyph = &normal.shaping.glyph_runs[0].glyphs[0];
        let bold_glyph = &bold.shaping.glyph_runs[0].glyphs[0];
        let normal_bounds = normal_glyph
            .bounds
            .expect("normal glyph must have ink bounds");
        let bold_bounds = bold_glyph
            .bounds
            .expect("synthetic glyph must have ink bounds");

        let expansion = SYNTHETIC_EMBOLDEN_EM * 96.0;

        assert_eq!(
            bold.face.synthesis().embolden_em(),
            Some(SYNTHETIC_EMBOLDEN_EM)
        );
        assert_eq!(bold_glyph.advance, normal_glyph.advance);
        assert_eq!(bold_bounds.left, normal_bounds.left - expansion);
        assert_eq!(bold_bounds.top, normal_bounds.top - expansion);
        assert_eq!(bold_bounds.right, normal_bounds.right + expansion);
        assert_eq!(bold_bounds.bottom, normal_bounds.bottom + expansion);
    }

    #[test]
    fn combines_synthetic_weight_and_style_when_both_capabilities_are_missing() {
        let manager = HuoziFontManager::from_sources(vec![FontSource::with_alias(
            SOURCE_HAN_SERIF_REGULAR.to_vec(),
            "serif".to_owned(),
        )])
        .unwrap();
        let result = manager.shape(&request_with_style(
            "中",
            TextStyle::builder().font_weight(700).italic(true).build(),
        ));

        assert_eq!(
            result.face.synthesis().embolden_em(),
            Some(SYNTHETIC_EMBOLDEN_EM)
        );
        assert_eq!(result.face.synthesis().oblique_degrees(), Some(14.0));
        assert_eq!(result.attempts.len(), 1);
    }

    #[test]
    fn synthesizes_oblique_when_italic_capability_is_missing() {
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

        assert_ne!(italic.face, normal.face);
        assert_eq!(italic.face.synthesis().oblique_degrees(), Some(14.0));
        assert_eq!(italic.attempts.len(), 1);
    }

    #[test]
    fn disables_synthetic_styles_when_font_synthesis_is_none() {
        let manager = HuoziFontManager::from_sources(vec![FontSource::with_alias(
            INTER.to_vec(),
            "inter".to_owned(),
        )])
        .unwrap();
        let normal = manager.shape(&request("A"));
        let italic = manager.shape(&request_with_style(
            "A",
            TextStyle::builder()
                .italic(true)
                .font_synthesis(FontSynthesis::NONE)
                .build(),
        ));

        assert_eq!(italic.face, normal.face);
        assert_eq!(italic.face.synthesis(), &FontSynthesisInstance::default());
    }

    #[test]
    fn disables_synthetic_weight_when_font_synthesis_excludes_weight() {
        let manager = HuoziFontManager::from_sources(vec![FontSource::with_alias(
            SOURCE_HAN_SERIF_REGULAR.to_vec(),
            "serif".to_owned(),
        )])
        .unwrap();
        let bold = manager.shape(&request_with_style(
            "中",
            TextStyle::builder()
                .font_weight(700)
                .font_synthesis(FontSynthesis::STYLE)
                .build(),
        ));

        assert_eq!(bold.face.synthesis(), &FontSynthesisInstance::default());
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

        let candidate = font_candidate(&face, 400, true, FontSynthesis::ALL);

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

        let candidate = font_candidate(&face, 400, true, FontSynthesis::ALL);

        assert_eq!(variation_value(&candidate.id, "slnt"), Some(-10.0));
        assert_eq!(candidate.style, FontFaceStyle::Oblique);
    }

    fn test_face_with_axes(variation_axes: FontVariationAxes) -> FontFaceRecord {
        FontFaceRecord {
            id: FontFaceId::with_resource_id("test-face"),
            source_index: 0,
            collection_index: 0,
            kind: None,
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
            shaper_data: ShaperData::new(
                &HarfRustFontRef::from_index(INTER, 0).expect("测试字体必须有效"),
            ),
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
    id: &FontFaceId,
    glyph_id: u32,
    scale: f32,
) -> Option<Rect> {
    let location = variation_location(font, id.variation_instance());
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
    let settings = skrifa::outline::DrawSettings::unhinted(
        Size::unscaled(),
        LocationRef::new(location.coords()),
    );
    if let Some(oblique_degrees) = id.synthesis().oblique_degrees() {
        let mut shearing_pen = ShearingPen::new(&mut pen, oblique_degrees);
        glyph.draw(settings, &mut shearing_pen).ok()?;
    } else {
        glyph.draw(settings, &mut pen).ok()?;
    }
    let bounds = pen.bounding_box()?;
    Some(Rect {
        left: bounds.x_min * scale,
        top: -bounds.y_max * scale,
        right: bounds.x_max * scale,
        bottom: -bounds.y_min * scale,
    })
}

struct ShearingPen<'a, P> {
    pen: &'a mut P,
    tangent: f32,
}

impl<'a, P> ShearingPen<'a, P> {
    fn new(pen: &'a mut P, oblique_degrees: f32) -> Self {
        Self {
            pen,
            tangent: oblique_degrees.to_radians().tan(),
        }
    }

    fn transform(&self, x: f32, y: f32) -> (f32, f32) {
        (x + self.tangent * y, y)
    }
}

impl<P: OutlinePen> OutlinePen for ShearingPen<'_, P> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.transform(x, y);
        self.pen.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.transform(x, y);
        self.pen.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (cx0, cy0) = self.transform(cx0, cy0);
        let (x, y) = self.transform(x, y);
        self.pen.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (cx0, cy0) = self.transform(cx0, cy0);
        let (cx1, cy1) = self.transform(cx1, cy1);
        let (x, y) = self.transform(x, y);
        self.pen.curve_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.pen.close();
    }
}
