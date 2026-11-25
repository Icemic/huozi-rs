use huozi::{
    FontSource, Huozi,
    effects::Effect,
    glyph_vertices::{TextRole, UnitVertices},
    layout::{ColorSpace, LayoutStyle, RichTextLayoutOutput},
    parser::{Segment, SegmentId, TextStyle},
};

fn engine() -> Huozi {
    Huozi::new(vec![FontSource::new(
        include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf").to_vec(),
    )])
    .unwrap()
}

fn layout(engine: &mut Huozi, text: &str) -> RichTextLayoutOutput {
    engine
        .layout_parse(
            &vec![Segment {
                id: Some(SegmentId::Tag("effects".into())),
                content: text.into(),
            }],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap()
}

#[test]
fn clusters_follow_shaping_units_and_keep_elements_separated() {
    let output = layout(&mut engine(), "[wave]éffi e\u{301}x[/wave]");
    assert_eq!(output.text_length, 8);
    // 一次 shaping 请求一个簇：拉丁词整体成簇，空格与组合序列各有自己的簇。
    assert_eq!(
        output
            .clusters
            .iter()
            .map(|cluster| cluster.source.clone())
            .collect::<Vec<_>>(),
        vec![0..4, 4..5, 5..7, 7..8]
    );
    assert!(
        output
            .clusters
            .iter()
            .all(|cluster| *cluster.effects == [Effect::Wave]),
        "整段都处在 [wave] 作用域内"
    );
    // 空格有进宽但没有绘制元素。
    assert!(output.clusters[1].glyph_range.is_empty());
    assert_eq!(output.clusters.first().unwrap().glyph_range.start, 0);
    assert_eq!(
        output.clusters.last().unwrap().glyph_range.end,
        output.glyphs.len()
    );
    for adjacent in output.clusters.windows(2) {
        assert_eq!(adjacent[0].glyph_range.end, adjacent[1].glyph_range.start);
    }
    // 首个簇内是「é」与「ffi」两个字形，来源按比例分配；连字簇不可再分。
    assert_eq!(
        output.clusters[0].element_sources.as_deref(),
        Some(&[0..2, 2..4][..])
    );
    assert_eq!(output.clusters[2].element_sources, None);
    assert_eq!(output.clusters[2].element_source(0), 5..7);
}

#[test]
fn effect_tag_inside_a_ligature_preserves_shaping_and_segment_identity() {
    let mut engine = engine();
    let plain = layout(&mut engine, "ffi");
    let tagged = layout(&mut engine, "f[wave]fi[/wave]");
    assert_eq!(tagged.clusters.len(), 1);
    assert_eq!(tagged.clusters[0].source, 0..3);
    assert!(tagged.clusters[0].effects.is_empty());
    assert_eq!(tagged.glyphs.len(), plain.glyphs.len());
    assert_eq!((tagged.width, tagged.height), (plain.width, plain.height));
    assert_eq!(tagged.segment_glyph_spans.len(), 1);
    let UnitVertices::Text(actual) = &tagged.glyphs[0] else {
        panic!("预期正文文字")
    };
    let UnitVertices::Text(expected) = &plain.glyphs[0] else {
        panic!("预期正文文字")
    };
    assert_eq!(actual.fill[0].position, expected.fill[0].position);
    assert_eq!(actual.fill[2].position, expected.fill[2].position);
}

#[test]
fn cluster_ranges_count_decoded_text_across_paragraphs_and_objects() {
    let output = layout(
        &mut engine(),
        "[wave]A[[B[/wave][br /][br /][glitch]é[/glitch][object id=icon alt=图标 width=16 ascent=12 descent=4 /]C",
    );
    assert_eq!(output.text_length, 7);
    assert_eq!(
        output
            .clusters
            .iter()
            .map(|cluster| cluster.source.clone())
            .collect::<Vec<_>>(),
        vec![0..3, 3..4, 4..6, 6..7]
    );
    assert_eq!(&*output.clusters[1].effects, &[Effect::Glitch]);
    assert_eq!(
        output.clusters[0].element_sources.as_deref(),
        Some(&[0..1, 1..2, 2..3][..])
    );
    // 行内对象按 `alt` 的 scalar 数占位，并作为所在簇的元素之一输出。
    assert!(
        output.glyphs[output.clusters[2].glyph_range.clone()]
            .iter()
            .any(|element| matches!(element, UnitVertices::InlineObject(_)))
    );
}

#[test]
fn cluster_groups_include_background_decoration_and_ruby() {
    let output = layout(
        &mut engine(),
        "[wave][background color=red][underline][ruby text=jiǎ]甲[/ruby][/underline][/background][/wave]乙",
    );
    assert_eq!(output.text_length, 2);
    assert_eq!(output.clusters.len(), 2);
    let first = &output.clusters[0];
    assert_eq!(&*first.effects, &[Effect::Wave]);
    let elements = &output.glyphs[first.glyph_range.clone()];
    assert!(
        elements
            .iter()
            .any(|element| matches!(element, UnitVertices::Background(_)))
    );
    assert!(
        elements
            .iter()
            .any(|element| matches!(element, UnitVertices::Line(_)))
    );
    assert!(
        elements.iter().any(
            |element| matches!(element, UnitVertices::Text(text) if text.role == TextRole::Ruby)
        )
    );
    assert!(output.clusters[1].effects.is_empty());
}

#[test]
fn height_clipping_keeps_submitted_text_length_and_visible_cluster_ranges() {
    let output = engine()
        .layout_parse(
            &vec![Segment::dummy("[wave]甲乙丙丁[/wave][br /]戊己")],
            &LayoutStyle {
                box_width: Some(64.0),
                box_height: Some(48.0),
                ..LayoutStyle::default()
            },
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();
    assert_eq!(output.text_length, 4);
    assert_eq!(output.clusters.len(), 2);
    assert_eq!(output.clusters.last().unwrap().source.end, 2);
}

#[test]
fn glow_color_uses_the_requested_output_color_space() {
    let output = engine()
        .layout_parse(
            &vec![Segment::dummy("[glow color=\"#80402080\"]A[/glow]")],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::Linear,
            None,
        )
        .unwrap();
    let expected: csscolorparser::Color = "#80402080".parse().unwrap();
    assert_eq!(
        &*output.clusters[0].effects,
        &[Effect::Glow {
            color: expected.to_linear_rgba()
        }]
    );
}
