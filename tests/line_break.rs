//! 断行粒度的回归测试：拉丁文按词换行，CJK 按字换行。
//!
//! 这些用例锁住的是「排版原子」的行为，不是实现细节：shaper 返回的 cluster 粒度会被 Tiqian
//! 当作断行机会，一旦 cluster 被拆到字形级，拉丁词就会在词中被断开。用例的值来自改动前的
//! `Release huozi v0.27.0`，任何改动都应当保持它们不变。

use huozi::{
    FontSource, Huozi,
    glyph_vertices::{TextRole, UnitVertices},
    layout::{ColorSpace, LayoutStyle, RichTextLayoutOutput},
    parser::{Segment, TextStyle},
};

const CJK_FONT: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");

fn layout(text: &str, box_width: f64) -> RichTextLayoutOutput {
    Huozi::new(vec![FontSource::new(CJK_FONT.to_vec())])
        .unwrap()
        .layout_parse(
            &vec![Segment::dummy(text)],
            &LayoutStyle {
                box_width: Some(box_width),
                ..LayoutStyle::default()
            },
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap()
}

/// 每行正文字形数量，按行号升序；行尾连字符也算一个字形。
fn rows(output: &RichTextLayoutOutput) -> Vec<usize> {
    let mut counts: Vec<(u32, usize)> = Vec::new();
    for element in &output.glyphs {
        let UnitVertices::Text(text) = element else {
            continue;
        };
        if text.role != TextRole::Body {
            continue;
        }
        match counts.last_mut() {
            Some((row, count)) if *row == text.row => *count += 1,
            _ => counts.push((text.row, 1)),
        }
    }
    counts.into_iter().map(|(_, count)| count).collect()
}

fn body_glyphs(output: &RichTextLayoutOutput) -> usize {
    output
        .glyphs
        .iter()
        .filter(
            |element| matches!(element, UnitVertices::Text(text) if text.role == TextRole::Body),
        )
        .count()
}

/// 空间充裕时只在词边界换行：`typography` 必须整体落到第二行。
#[test]
fn latin_words_break_at_word_boundaries() {
    let output = layout("hello world typography", 200.0);
    assert_eq!(rows(&output), vec![10, 10]);
    assert_eq!(body_glyphs(&output), 20);
}

/// 长词能整行放下时不得在词中断开。
#[test]
fn latin_long_word_moves_to_the_next_line_as_a_whole() {
    let output = layout("aaaa bbbbbbbbbbbb ccc", 260.0);
    assert_eq!(rows(&output), vec![4, 12, 3]);
    assert_eq!(body_glyphs(&output), 19);
}

/// 词比一整行还宽时必须走连字符断词：20 个字母 + 4 个行尾连字符。
#[test]
fn latin_long_word_hyphenates_instead_of_breaking_silently() {
    let output = layout("internationalization", 100.0);
    assert_eq!(rows(&output), vec![6, 3, 7, 4, 4]);
    assert_eq!(body_glyphs(&output), 24);
}

/// CJK 按字换行，不受拉丁断行规则影响。
#[test]
fn cjk_breaks_between_ideographs() {
    let output = layout("中文排版测试换行", 100.0);
    assert_eq!(rows(&output), vec![3, 3, 2]);
    assert_eq!(body_glyphs(&output), 8);
}
