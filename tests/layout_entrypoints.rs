use huozi::{
    FontSource, Huozi,
    glyph_vertices::{TextVertices, UnitVertices},
    layout::{ColorSpace, LayoutStyle, ParagraphAlignment},
    parser::{Segment, SegmentId, TextRun, TextSpan, TextStyle},
};

const TEST_FONT: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");

fn engine() -> Huozi {
    Huozi::new(vec![FontSource::new(TEST_FONT.to_vec())]).unwrap()
}

/// 取出文字变体的顶点；这些用例只断言普通文字。
fn text(element: &UnitVertices) -> &TextVertices {
    match element {
        UnitVertices::Text(vertices) => vertices,
        other => panic!("expected a text element, got {other:?}"),
    }
}

#[test]
fn layout_parse_uses_tiqian_for_rich_text_and_source_spans() {
    let segments = vec![Segment {
        id: Some(SegmentId::Lite(3)),
        content: "[size=48]中[/size]".into(),
    }];
    let style = TextStyle {
        font_size: 32.0,
        ..Default::default()
    };

    let output = engine()
        .layout_parse(
            &segments,
            &LayoutStyle::default(),
            &style,
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    assert_eq!(output.glyphs.len(), 1);
    assert_eq!(text(&output.glyphs[0]).scale_ratio, 0.5);
    assert_eq!(output.segment_glyph_spans.len(), 1);
    assert_eq!(output.segment_glyph_spans[0].segment_id, SegmentId::Lite(3));
    assert_eq!(output.segment_glyph_spans[0].glyph_range, 0..1);
}

#[test]
fn layout_plain_does_not_draw_notdef_for_space() {
    let output = engine()
        .layout_plain(
            &vec![Segment::dummy("A B")],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
        )
        .unwrap();

    assert_eq!(output.glyphs.len(), 2);
}

#[test]
fn layout_plain_positions_short_line_for_each_align_value() {
    let text_style = TextStyle::default();
    let layout_style = |align| LayoutStyle {
        box_width: Some(160.0),
        align,
        ..LayoutStyle::default()
    };
    let plain_position_for = |align| {
        let output = engine()
            .layout_plain(
                &vec![Segment::dummy("中")],
                &layout_style(align),
                &text_style,
                ColorSpace::SRGB,
            )
            .unwrap();
        text(&output.glyphs[0]).fill[0].position[0]
    };

    let start = plain_position_for(ParagraphAlignment::Start);
    let center = plain_position_for(ParagraphAlignment::Center);
    let end = plain_position_for(ParagraphAlignment::End);

    assert!(start < center);
    assert!(center < end);

    let text_spans = vec![TextSpan {
        runs: vec![TextRun {
            text: "中".to_owned(),
            style: text_style.clone(),
            ..TextRun::default()
        }],
        span_id: None,
    }];
    let output = engine().layout(
        &layout_style(ParagraphAlignment::Center),
        &text_spans,
        ColorSpace::SRGB,
    );
    assert_eq!(text(&output.glyphs[0]).fill[0].position[0], center);

    let output = engine()
        .layout_parse(
            &vec![Segment::dummy("中")],
            &layout_style(ParagraphAlignment::End),
            &text_style,
            ColorSpace::SRGB,
            None,
        )
        .unwrap();
    assert_eq!(text(&output.glyphs[0]).fill[0].position[0], end);

    let output = engine()
        .layout_parse_with::<'{', '}'>(
            &vec![Segment::dummy("中")],
            &layout_style(ParagraphAlignment::Center),
            &text_style,
            ColorSpace::SRGB,
            None,
        )
        .unwrap();
    assert_eq!(text(&output.glyphs[0]).fill[0].position[0], center);
}

#[test]
fn layout_plain_degrades_unbounded_center_alignment_to_start() {
    let text_style = TextStyle::default();
    let position_for = |align| {
        let output = engine()
            .layout_plain(
                &vec![Segment::dummy("中")],
                &LayoutStyle {
                    align,
                    ..LayoutStyle::default()
                },
                &text_style,
                ColorSpace::SRGB,
            )
            .unwrap();
        text(&output.glyphs[0]).fill[0].position[0]
    };

    let start = position_for(ParagraphAlignment::Start);
    let center = position_for(ParagraphAlignment::Center);

    assert!(start.is_finite());
    assert_eq!(center, start);
}

#[test]
fn layout_plain_keeps_auto_wrapped_lines_unshifted() {
    let text_style = TextStyle::default();
    let glyphs_for = |align| {
        engine()
            .layout_plain(
                &vec![Segment::dummy(
                    "这是一个用于验证自动换行对齐行为的中文段落。",
                )],
                &LayoutStyle {
                    box_width: Some(160.0),
                    align,
                    ..LayoutStyle::default()
                },
                &text_style,
                ColorSpace::SRGB,
            )
            .unwrap()
            .glyphs
    };

    let start = glyphs_for(ParagraphAlignment::Start);
    let center = glyphs_for(ParagraphAlignment::Center);
    let end = glyphs_for(ParagraphAlignment::End);

    assert!(text(start.last().unwrap()).row > text(&start[0]).row);
    assert_eq!(
        text(&center[0]).fill[0].position[0],
        text(&start[0]).fill[0].position[0]
    );
    assert_eq!(
        text(&end[0]).fill[0].position[0],
        text(&start[0]).fill[0].position[0]
    );
    assert!(
        text(start.last().unwrap()).fill[0].position[0]
            < text(center.last().unwrap()).fill[0].position[0]
    );
    assert!(
        text(center.last().unwrap()).fill[0].position[0]
            < text(end.last().unwrap()).fill[0].position[0]
    );
}

#[test]
fn layout_parse_outputs_one_link_area_per_positioned_cluster() {
    let output = engine()
        .layout_parse(
            &vec![Segment::dummy(
                "[link id=entry-42 target=\"https://example.com/42\"]甲乙[/link]",
            )],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    assert_eq!(output.interactions.len(), 1);
    let interaction = &output.interactions[0];
    assert_eq!(interaction.id, "entry-42");
    assert_eq!(interaction.areas.len(), 2);
    assert_eq!(interaction.areas[0].row, text(&output.glyphs[0]).row);
    assert_eq!(interaction.areas[0].col, text(&output.glyphs[0]).col);
    assert_eq!(interaction.areas[1].row, text(&output.glyphs[1]).row);
    assert_eq!(interaction.areas[1].col, text(&output.glyphs[1]).col);
    assert!(interaction.areas.iter().all(|area| area.rect.width() > 0.0));
    assert!(
        interaction
            .areas
            .iter()
            .all(|area| area.rect.height() > 0.0)
    );
}

#[test]
fn layout_parse_outputs_link_and_object_interactions_in_input_order() {
    let output = engine()
        .layout_parse(
            &vec![Segment::dummy(
                "[link id=link-1 target=\"https://example.com\"]甲乙[/link][object id=object-1 alt=图 width=12 ascent=9 descent=3 /]",
            )],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    assert_eq!(output.interactions.len(), 2);
    assert_eq!(output.interactions[0].id, "link-1");
    assert_eq!(output.interactions[0].areas.len(), 2);
    assert_eq!(output.interactions[1].id, "object-1");
    assert_eq!(output.interactions[1].areas.len(), 1);
    assert_eq!(
        output.interactions[1].areas[0].row,
        output.interactions[0].areas[1].row
    );
    assert_eq!(
        output.interactions[1].areas[0].col,
        output.interactions[0].areas[1].col + 1
    );
}

#[test]
fn layout_parse_orders_nested_object_before_outer_link() {
    let output = engine()
        .layout_parse(
            &vec![Segment::dummy(
                "[link id=link-1 target=\"https://example.com\"][object id=object-1 alt=图 width=12 ascent=9 descent=3 /][/link]",
            )],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    assert_eq!(output.interactions.len(), 2);
    assert_eq!(output.interactions[0].id, "object-1");
    assert_eq!(output.interactions[1].id, "link-1");
    assert_eq!(output.interactions[0].areas, output.interactions[1].areas);
}

#[test]
fn layout_parse_omits_missing_or_empty_ids_and_keeps_duplicate_ids_separate() {
    let without_ids = engine()
        .layout_parse(
            &vec![Segment::dummy(
                "[link target=\"https://example.com\"]甲[/link][link id='' target=\"https://example.com\"]乙[/link][object alt=图 width=12 ascent=9 descent=3 /][object id='' alt=标 width=12 ascent=9 descent=3 /]",
            )],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();
    assert!(without_ids.interactions.is_empty());

    let duplicates = engine()
        .layout_parse(
            &vec![Segment::dummy(
                "[link id=same target=\"https://example.com\"]甲[/link][link id=same target=\"https://example.com\"]乙[/link]",
            )],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();
    assert_eq!(duplicates.interactions.len(), 2);
    assert_eq!(duplicates.interactions[0].id, "same");
    assert_eq!(duplicates.interactions[1].id, "same");
    assert_eq!(duplicates.interactions[0].areas.len(), 1);
    assert_eq!(duplicates.interactions[1].areas.len(), 1);
}
