//! 空白模式、换行控制与 Tab stop 在 Huozi 配置层的接入行为。

use huozi::{
    FontSource, Huozi,
    glyph_vertices::{TextVertices, UnitVertices},
    layout::{ColorSpace, LayoutStyle, TabSize, TextWrapMode, WhiteSpaceMode},
    parser::{Segment, TextStyle},
};

const CJK_FONT: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");

fn engine() -> Huozi {
    Huozi::new(vec![FontSource::new(CJK_FONT.to_vec())]).unwrap()
}

fn layout_plain(text: &str, layout_style: &LayoutStyle) -> huozi::layout::RichTextLayoutOutput {
    engine()
        .layout_plain(
            &vec![Segment::dummy(text)],
            layout_style,
            &TextStyle::default(),
            ColorSpace::SRGB,
        )
        .unwrap()
}

fn layout_parsed(text: &str, layout_style: &LayoutStyle) -> huozi::layout::RichTextLayoutOutput {
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

/// 返回文字元素在指定行列上的横向起点。
fn text_left(output: &huozi::layout::RichTextLayoutOutput, row: u32, col: u32) -> Option<u32> {
    output.glyphs.iter().find_map(|element| match element {
        UnitVertices::Text(TextVertices {
            row: element_row,
            col: element_col,
            x,
            ..
        }) if *element_row == row && *element_col == col => Some(*x),
        _ => None,
    })
}

/// 返回指定行号的文字元素数量。
fn text_count_in_row(output: &huozi::layout::RichTextLayoutOutput, row: u32) -> usize {
    output
        .glyphs
        .iter()
        .filter(|element| matches!(element, UnitVertices::Text(text) if text.row == row))
        .count()
}

/// 返回出现过文字元素的行数。
fn text_row_count(output: &huozi::layout::RichTextLayoutOutput) -> u32 {
    output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Text(text) => Some(text.row),
            _ => None,
        })
        .max()
        .map_or(0, |max| max + 1)
}

#[test]
fn default_style_matches_tiqian_paragraph_defaults() {
    let style = LayoutStyle::default();
    assert_eq!(style.white_space, WhiteSpaceMode::PreserveBreaks);
    assert_eq!(style.text_wrap, TextWrapMode::Wrap);
    assert_eq!(style.tab_size, TabSize::SpaceAdvanceMultiple(8.0));
}

#[test]
fn preserve_modes_keep_trailing_spaces_in_width() {
    // 同一文本在折叠模式下尾空格不占宽，在保留模式下占宽，配置确实到达了 tiqian。
    let text = "甲  ";
    let collapsed = layout_plain(
        text,
        &LayoutStyle {
            white_space: WhiteSpaceMode::Collapse,
            ..LayoutStyle::default()
        },
    );
    let preserved = layout_plain(
        text,
        &LayoutStyle {
            white_space: WhiteSpaceMode::Preserve,
            ..LayoutStyle::default()
        },
    );

    assert!(
        preserved.width > collapsed.width,
        "preserved width {} should exceed collapsed width {}",
        preserved.width,
        collapsed.width
    );
}

#[test]
fn no_wrap_keeps_single_line_beyond_box_width() {
    let text = "甲乙丙丁戊己庚辛";
    let wrapping = layout_plain(
        text,
        &LayoutStyle {
            box_width: Some(64.0),
            ..LayoutStyle::default()
        },
    );
    let no_wrap = layout_plain(
        text,
        &LayoutStyle {
            box_width: Some(64.0),
            text_wrap: TextWrapMode::NoWrap,
            ..LayoutStyle::default()
        },
    );

    assert_eq!(1, text_row_count(&no_wrap));
    assert!(text_row_count(&wrapping) > 1);
}

#[test]
fn tab_size_multiple_changes_following_glyph_position() {
    let text = "甲\t乙";
    let narrow = layout_plain(
        text,
        &LayoutStyle {
            white_space: WhiteSpaceMode::Preserve,
            tab_size: TabSize::SpaceAdvanceMultiple(1.0),
            ..LayoutStyle::default()
        },
    );
    let wide = layout_plain(
        text,
        &LayoutStyle {
            white_space: WhiteSpaceMode::Preserve,
            tab_size: TabSize::SpaceAdvanceMultiple(8.0),
            ..LayoutStyle::default()
        },
    );

    let narrow_left = text_left(&narrow, 0, 2).expect("乙 should be drawn in narrow tab mode");
    let wide_left = text_left(&wide, 0, 2).expect("乙 should be drawn in wide tab mode");
    assert!(
        wide_left > narrow_left,
        "wide tab left {} should exceed narrow tab left {}",
        wide_left,
        narrow_left
    );
}

#[test]
fn br_attributes_override_following_paragraph() {
    // 第二段用 Preserve 保留尾空格，第一段按默认折叠。
    let output = layout_parsed(
        "甲[br whiteSpace=preserve /]乙  ",
        &LayoutStyle::default(),
    );

    assert_eq!(2, text_row_count(&output));
    assert_eq!(1, text_count_in_row(&output, 0));
    assert_eq!(1, text_count_in_row(&output, 1));
}

#[test]
fn invalid_br_attributes_keep_previous_configuration() {
    let output = layout_parsed(
        "甲[br whiteSpace=unknown textWrap=unknown tabSize=oops /]乙",
        &LayoutStyle::default(),
    );

    // 非法取值只告警，不影响段落布局继续产出。
    assert_eq!(2, text_row_count(&output));
}

#[test]
fn br_tab_size_override_accepts_numeric_value() {
    let output = layout_parsed("甲[br tabSize=4 /]\t乙", &LayoutStyle::default());
    assert_eq!(2, text_row_count(&output));
}

#[test]
fn no_wrap_br_attribute_keeps_single_line() {
    // `[br]` 只影响后续段落，所以被测长文本放在标签之后。
    let output = layout_parsed(
        "甲[br textWrap=noWrap /]乙丙丁戊己庚辛",
        &LayoutStyle {
            box_width: Some(64.0),
            ..LayoutStyle::default()
        },
    );

    assert_eq!(2, text_row_count(&output));
}
