use huozi::font_backend::{FontSource, HuoziFontManager};
use tiqian::core::{geometry::text_range, text::Text, text_model::TextStyle};
use tiqian::font::font_policy::FontRole;
use tiqian::shaping::font_backend::{FontBackend, FontBackendRequest};
use tiqian::shaping::replayable_font_backend::ReplayableFontCatalog;

#[test]
fn source_clusters_are_scalar_ranges_not_whole_request() {
    let manager = HuoziFontManager::from_sources(vec![FontSource::new(
        include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf").to_vec(),
    )])
    .unwrap();
    let text = Text::from("éffi e\u{301}x");
    let result = manager.shape(&FontBackendRequest::new(
        text.clone(),
        text_range(0, text.scalar_len().value()),
        TextStyle::builder().font_size(24.0).build(),
        FontRole::LatinText,
    ));
    let ranges: Vec<_> = result
        .shaping
        .glyph_runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .map(|glyph| {
            (
                glyph.cluster_range.start().value(),
                glyph.cluster_range.end().value(),
            )
        })
        .collect();
    assert_eq!(ranges.first(), Some(&(0, 1)));
    assert!(
        ranges.contains(&(1, 4)),
        "ffi must retain its ligature source: {ranges:?}"
    );
    assert!(
        ranges.contains(&(5, 7)),
        "combining sequence must stay atomic: {ranges:?}"
    );
    assert_eq!(ranges.last(), Some(&(7, 8)));
}

#[test]
fn display_substitution_keeps_glyph_positions_within_one_source_cluster() {
    let manager = HuoziFontManager::from_sources(vec![FontSource::new(
        include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf").to_vec(),
    )])
    .unwrap();
    let request = FontBackendRequest::builder(
        Text::from("前x后"),
        text_range(1, 2),
        TextStyle::builder().font_size(24.0).build(),
        FontRole::LatinText,
    )
    .display_text(Text::from("HH"))
    .build();
    let result = manager.shape(&request);
    assert_eq!(result.shaping.clusters.len(), 1);
    assert_eq!(result.shaping.clusters[0].range, text_range(1, 2));
    let glyphs = &result.shaping.glyph_runs[0].glyphs;
    assert_eq!(glyphs.len(), 2);
    assert!(
        glyphs
            .iter()
            .all(|glyph| { glyph.id != 0 && glyph.cluster_range == text_range(1, 2) })
    );
    assert_eq!(glyphs[0].x, 0.0);
    assert!(glyphs[0].advance > 0.0);
    assert!(
        (glyphs[1].x - glyphs[0].advance).abs() < 0.001,
        "替换后的第二个字形应紧接第一个字形，而不是回到来源簇的起点：{glyphs:?}"
    );
}

#[test]
fn raster_pixels_are_owned_and_baseline_positioned() {
    let manager = HuoziFontManager::from_sources(vec![FontSource::new(
        include_bytes!("../resources/fonts/InterVariable.ttf").to_vec(),
    )])
    .unwrap();
    let request = FontBackendRequest::new(
        Text::from("A"),
        text_range(0, 1),
        TextStyle::builder().font_size(24.0).build(),
        FontRole::LatinText,
    );
    let result = manager.shape_glyphs(&request);
    let glyph = &result.glyphs[0];
    assert_eq!(manager.glyph_id(&glyph.face, 'A'), Some(glyph.glyph_id));
    assert!(
        (manager.glyph_advance(&glyph.face, glyph.glyph_id, 24.0) - glyph.advance).abs() < 0.01
    );
    assert!(manager.font_metrics(&glyph.face, 24.0).ascent > 0.0);
    assert!(manager.supports_character(&glyph.face, 'A'));
    assert!(!manager.supports_character(&glyph.face, '\u{10ffff}'));
    assert!(manager.supported_characters(&glyph.face).contains('A'));
    assert!(
        !manager
            .supported_characters(&glyph.face)
            .contains('\u{10ffff}')
    );
    assert!(manager.glyph_count(&glyph.face) > glyph.glyph_id);
    assert_eq!(manager.face_count(), 1);
    let sdf = manager
        .rasterize_glyph(&glyph.face, glyph.glyph_id, 24.0)
        .unwrap()
        .unwrap();
    assert_eq!(sdf.pixels.len(), (sdf.width * sdf.height) as usize);
    assert!(sdf.pixels.iter().any(|pixel| *pixel > 128));
    assert!(sdf.top < 0.0);
}

#[test]
fn chinese_clusters_keep_nonzero_scalar_source_offset() {
    let manager = HuoziFontManager::from_sources(vec![FontSource::new(
        include_bytes!("../resources/fonts/LXGWWenKaiLite-Regular.ttf").to_vec(),
    )])
    .unwrap();
    let text = Text::from("前中文后");
    let request = FontBackendRequest::new(
        text,
        text_range(1, 3),
        TextStyle::builder().font_size(24.0).build(),
        FontRole::CjkText,
    );
    let result = manager.shape_glyphs(&request);
    assert_eq!(
        result
            .glyphs
            .iter()
            .map(|glyph| glyph.source.clone())
            .collect::<Vec<_>>(),
        vec![1..2, 2..3]
    );
    assert!(result.glyphs.iter().all(|glyph| glyph.glyph_id != 0));
}

#[test]
fn exact_face_shaping_bypasses_family_fallback() {
    let manager = HuoziFontManager::from_sources(vec![
        FontSource::new(include_bytes!("../resources/fonts/InterVariable.ttf").to_vec()),
        FontSource::new(include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf").to_vec()),
    ])
    .unwrap();
    let face = manager.faces()[1].id.clone();
    let request = FontBackendRequest::new(
        Text::from("A"),
        text_range(0, 1),
        TextStyle::builder().font_size(24.0).build(),
        FontRole::LatinText,
    );
    let shaped = manager.shape_glyphs_for_face(&face, &request);
    assert_eq!(shaped.glyphs[0].face, face);
    assert_eq!(manager.face_count(), 2);
}
