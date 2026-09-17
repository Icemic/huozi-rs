use huozi::{
    FontSource, Huozi,
    layout::{ColorSpace, LayoutStyle},
    parser::{Segment, TextStyle},
};

const TEST_FONT: &[u8] = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");

#[test]
fn layout_parse_with_uses_custom_tag_symbols() {
    let mut huozi = Huozi::new(vec![FontSource::new(TEST_FONT.to_vec())]).unwrap();
    let segments = vec![Segment::dummy("【size=48】中【/size】")];

    let (glyphs, _, _, _) = huozi
        .layout_parse_with::<'【', '】'>(
            &segments,
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    assert_eq!(glyphs.len(), 1);
    assert_eq!(glyphs[0].scale_ratio, 0.5);
}