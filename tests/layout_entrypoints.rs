use huozi::{
    FontSource, Huozi,
    layout::{ColorSpace, LayoutStyle},
    parser::{Segment, SegmentId, TextStyle},
};

const TEST_FONT: &[u8] = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");

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

