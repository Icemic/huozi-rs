use huozi::{
    FontSource, Huozi,
    layout::{ColorSpace, LayoutStyle, ParagraphAlignment},
    parser::{Segment, SegmentId, TextRun, TextSpan, TextStyle},
};

const TEST_FONT: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");

fn engine() -> Huozi {
    Huozi::new(vec![FontSource::new(TEST_FONT.to_vec())]).unwrap()
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

    let (glyphs, spans, _, _) = engine()
        .layout_parse(
            &segments,
            &LayoutStyle::default(),
            &style,
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    assert_eq!(glyphs.len(), 1);
    assert_eq!(glyphs[0].scale_ratio, 0.5);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].segment_id, SegmentId::Lite(3));
    assert_eq!(spans[0].glyph_range, 0..1);
}

#[test]
fn layout_plain_does_not_draw_notdef_for_space() {
    let (glyphs, _, _, _) = engine()
        .layout_plain(
            &vec![Segment::dummy("A B")],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
        )
        .unwrap();

    assert_eq!(glyphs.len(), 2);
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
        let (glyphs, _, _, _) = engine()
            .layout_plain(
                &vec![Segment::dummy("中")],
                &layout_style(align),
                &text_style,
                ColorSpace::SRGB,
            )
            .unwrap();
        glyphs[0].fill[0].position[0]
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
    let (glyphs, _, _, _) = engine().layout(
        &layout_style(ParagraphAlignment::Center),
        &text_spans,
        ColorSpace::SRGB,
    );
    assert_eq!(glyphs[0].fill[0].position[0], center);

    let (glyphs, _, _, _) = engine()
        .layout_parse(
            &vec![Segment::dummy("中")],
            &layout_style(ParagraphAlignment::End),
            &text_style,
            ColorSpace::SRGB,
            None,
        )
        .unwrap();
    assert_eq!(glyphs[0].fill[0].position[0], end);

    let (glyphs, _, _, _) = engine()
        .layout_parse_with::<'{', '}'>(
            &vec![Segment::dummy("中")],
            &layout_style(ParagraphAlignment::Center),
            &text_style,
            ColorSpace::SRGB,
            None,
        )
        .unwrap();
    assert_eq!(glyphs[0].fill[0].position[0], center);
}

#[test]
fn layout_plain_degrades_unbounded_center_alignment_to_start() {
    let text_style = TextStyle::default();
    let position_for = |align| {
        let (glyphs, _, _, _) = engine()
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
        glyphs[0].fill[0].position[0]
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
            .0
    };

    let start = glyphs_for(ParagraphAlignment::Start);
    let center = glyphs_for(ParagraphAlignment::Center);
    let end = glyphs_for(ParagraphAlignment::End);

    assert!(start.last().unwrap().row > start[0].row);
    assert_eq!(center[0].fill[0].position[0], start[0].fill[0].position[0]);
    assert_eq!(end[0].fill[0].position[0], start[0].fill[0].position[0]);
    assert!(start.last().unwrap().fill[0].position[0] < center.last().unwrap().fill[0].position[0]);
    assert!(center.last().unwrap().fill[0].position[0] < end.last().unwrap().fill[0].position[0]);
}
