use huozi::{
    FontSource, Huozi,
    glyph_vertices::{TextVertices, UnitVertices},
    layout::{ColorSpace, LayoutStyle},
    parser::{Segment, TextStyle},
};

const TEST_FONT: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");

/// 取出文字变体的顶点；这些用例只断言普通文字。
fn text(element: &UnitVertices) -> &TextVertices {
    match element {
        UnitVertices::Text(vertices) => vertices,
        other => panic!("expected a text element, got {other:?}"),
    }
}

#[test]
fn layout_parse_with_uses_custom_tag_symbols() {
    let mut huozi = Huozi::new(vec![FontSource::new(TEST_FONT.to_vec())]).unwrap();
    let segments = vec![Segment::dummy("【size=48】中【/size】")];

    let output = huozi
        .layout_parse_with::<'【', '】'>(
            &segments,
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    assert_eq!(output.glyphs.len(), 1);
    assert_eq!(text(&output.glyphs[0]).scale_ratio, 0.5);
}
