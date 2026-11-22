//! 段落样式字段在配置层与富文本标签上的接入行为。

use huozi::{
    FontSource, Huozi,
    glyph_vertices::{TextVertices, UnitVertices},
    layout::{
        ColorSpace, FirstLineIndent, LayoutStyle, LineLengthGrid, ParagraphAlignment,
        RichTextLayoutOutput,
    },
    parser::{Segment, TextStyle},
};

const CJK_FONT: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");

fn engine() -> Huozi {
    Huozi::new(vec![FontSource::new(CJK_FONT.to_vec())]).unwrap()
}

fn layout_plain(text: &str, layout_style: &LayoutStyle) -> RichTextLayoutOutput {
    engine()
        .layout_plain(
            &vec![Segment::dummy(text)],
            layout_style,
            &TextStyle::default(),
            ColorSpace::SRGB,
        )
        .unwrap()
}

fn layout_parsed(text: &str, layout_style: &LayoutStyle) -> RichTextLayoutOutput {
    engine()
        .layout_parse(
            &vec![Segment::dummy(text)],
            layout_style,
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap()
}

/// 返回指定行最左侧文字元素的横向起点。
fn row_left(output: &RichTextLayoutOutput, row: u32) -> Option<u32> {
    output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Text(TextVertices {
                row: element_row,
                x,
                ..
            }) if *element_row == row => Some(*x),
            _ => None,
        })
        .min()
}

#[test]
fn block_indent_shifts_every_line() {
    let text = "甲乙丙丁戊己庚辛";
    let plain = layout_plain(
        text,
        &LayoutStyle {
            box_width: Some(64.0),
            ..LayoutStyle::default()
        },
    );
    let indented = layout_plain(
        text,
        &LayoutStyle {
            box_width: Some(64.0),
            block_indent: 1.0,
            ..LayoutStyle::default()
        },
    );

    let plain_first = row_left(&plain, 0).expect("第一行应有文字");
    let plain_second = row_left(&plain, 1).expect("第二行应有文字");
    let indented_first = row_left(&indented, 0).expect("缩进后第一行应有文字");
    let indented_second = row_left(&indented, 1).expect("缩进后第二行应有文字");

    assert!(indented_first > plain_first);
    assert!(indented_second > plain_second);
    assert_eq!(indented_first - plain_first, indented_second - plain_second);
}

#[test]
fn auto_indent_follows_measure_width_while_fixed_ignores_it() {
    let indent_at = |width: f64, indent: FirstLineIndent| {
        let output = layout_plain(
            "甲乙丙",
            &LayoutStyle {
                box_width: Some(width),
                indent,
                ..LayoutStyle::default()
            },
        );
        row_left(&output, 0).expect("应有文字")
    };

    // 字号 32：窄行（< 14 字）缩 1 字，宽行缩 2 字。
    assert_eq!(indent_at(320.0, FirstLineIndent::Auto), 32);
    assert_eq!(indent_at(960.0, FirstLineIndent::Auto), 64);
    assert_eq!(
        indent_at(320.0, FirstLineIndent::Fixed(2.0)),
        indent_at(960.0, FirstLineIndent::Fixed(2.0))
    );
}

#[test]
fn line_length_grid_body_alignment_moves_quantized_slack() {
    let style = |grid: LineLengthGrid| LayoutStyle {
        box_width: Some(300.0),
        line_length_grid: grid,
        ..LayoutStyle::default()
    };

    let start = layout_plain("甲", &style(LineLengthGrid::with_enabled(true)));
    let end = layout_plain(
        "甲",
        &style(LineLengthGrid::new(true, Some(ParagraphAlignment::End))),
    );
    let disabled = layout_plain(
        "甲",
        &style(LineLengthGrid::new(false, Some(ParagraphAlignment::End))),
    );

    let start_left = row_left(&start, 0).expect("起点对齐应有文字");
    let end_left = row_left(&end, 0).expect("末端对齐应有文字");
    let disabled_left = row_left(&disabled, 0).expect("关闭量化应有文字");

    assert!(end_left > start_left);
    assert_eq!(disabled_left, start_left);
}

#[test]
fn br_overrides_apply_to_following_paragraphs() {
    // 第二段改用自适应缩进并加整段缩进；宽行下自动缩进为 2 字。
    let output = layout_parsed(
        "甲[br indent=auto blockIndent=1 /]乙",
        &LayoutStyle {
            box_width: Some(960.0),
            ..LayoutStyle::default()
        },
    );

    let first_left = row_left(&output, 0).expect("第一段应有文字");
    let second_left = row_left(&output, 1).expect("第二段应有文字");
    assert_eq!(first_left, 0);
    assert_eq!(second_left, 64 + 32);
}

#[test]
fn br_line_length_grid_body_alignment_default_resets_inherited_value() {
    let output = layout_parsed(
        "甲[br lineLengthGridBodyAlignment=end /]乙[br lineLengthGridBodyAlignment=default /]丙",
        &LayoutStyle {
            box_width: Some(300.0),
            ..LayoutStyle::default()
        },
    );

    let first_left = row_left(&output, 0).expect("第一段应有文字");
    let end_left = row_left(&output, 1).expect("第二段应有文字");
    let reset_left = row_left(&output, 2).expect("第三段应有文字");

    assert!(end_left > first_left);
    assert_eq!(reset_left, first_left);
}
