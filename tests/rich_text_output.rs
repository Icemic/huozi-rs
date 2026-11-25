//! 富文本布局输出的公开行为：逐字编号、连续范围、多段组合与来源 span。

use huozi::{
    FontSource, Huozi,
    glyph_vertices::{DecorationShape, TextRole, UnitVertices},
    layout::{ColorSpace, LayoutStyle},
    parser::{Segment, SegmentId, TextStyle},
};
use std::collections::HashMap;

const CJK_FONT: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");

fn engine() -> Huozi {
    Huozi::new(vec![FontSource::new(CJK_FONT.to_vec())]).unwrap()
}

fn layout(text: &str, layout_style: &LayoutStyle) -> huozi::layout::RichTextLayoutOutput {
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

fn layout_with_id(id: SegmentId, text: &str) -> huozi::layout::RichTextLayoutOutput {
    engine()
        .layout_parse(
            &vec![Segment {
                id: Some(id),
                content: text.into(),
            }],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap()
}

fn position(element: &UnitVertices) -> (u32, u32) {
    match element {
        UnitVertices::Text(text) => (text.row, text.col),
        UnitVertices::Background(background) => (background.row, background.col),
        UnitVertices::Line(line) => (line.row, line.col),
        UnitVertices::Decoration(decoration) => (decoration.row, decoration.col),
        UnitVertices::InlineObject(object) => (object.row, object.col),
    }
}

fn backgrounds(
    output: &huozi::layout::RichTextLayoutOutput,
) -> Vec<&huozi::glyph_vertices::BackgroundVertices> {
    output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Background(background) => Some(background),
            _ => None,
        })
        .collect()
}

fn lines(
    output: &huozi::layout::RichTextLayoutOutput,
) -> Vec<&huozi::glyph_vertices::LineVertices> {
    output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Line(line) => Some(line),
            _ => None,
        })
        .collect()
}

fn decorations(
    output: &huozi::layout::RichTextLayoutOutput,
) -> Vec<&huozi::glyph_vertices::DecorationVertices> {
    output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Decoration(decoration) => Some(decoration),
            _ => None,
        })
        .collect()
}

fn text_positions(output: &huozi::layout::RichTextLayoutOutput) -> Vec<(u32, u32)> {
    output
        .glyphs
        .iter()
        .filter(|element| matches!(element, UnitVertices::Text(_)))
        .map(position)
        .collect()
}

/// 一段背景填充层的颜色；没有填充顶点时为 `None`。
fn fill_color(background: &huozi::glyph_vertices::BackgroundVertices) -> Option<[f32; 4]> {
    background.vertices.fill.first().map(|vertex| vertex.color)
}

/// 一段背景填充层的横向范围；没有填充顶点时为 `None`。
fn fill_span(background: &huozi::glyph_vertices::BackgroundVertices) -> Option<(f32, f32)> {
    let mut span: Option<(f32, f32)> = None;
    for vertex in &background.vertices.fill {
        let x = vertex.position[0];
        span = Some(match span {
            Some((left, right)) => (left.min(x), right.max(x)),
            None => (x, x),
        });
    }
    span
}

#[test]
fn col_advances_per_positioned_cluster() {
    let output = layout("甲乙", &LayoutStyle::default());

    assert_eq!(text_positions(&output), vec![(0, 0), (0, 1)]);
}

/// 输出必须严格按逐字显示顺序排列，使调用方取前缀就是"打印到第 N 个元素"的完整可见状态。
///
/// 顺序是 `(row, col)` 为主序，同一排版单元内按绘制层排列：背景在文字之下，线条与装饰在其上，
/// 行内对象最后。
#[test]
fn glyphs_are_ordered_by_display_sequence() {
    let output = layout(
        "[background color=\"#FFF3BF\" radius=4]甲乙[/background][underline color=\"#1677FF\" thickness=1]丙[/underline][object id=\"icon\" alt=\"图标\" width=16 ascent=12 descent=4 /]丁",
        &LayoutStyle {
            box_width: Some(64.0),
            ..LayoutStyle::default()
        },
    );

    // 同一 (row, col) 的绘制层次序，与固定 paint layer 一致。
    let rank = |element: &UnitVertices| match element {
        UnitVertices::Background(_) => 0,
        UnitVertices::Text(text) => match text.role {
            TextRole::Body => 1,
            TextRole::Ruby | TextRole::Bopomofo => 3,
        },
        UnitVertices::Line(_) | UnitVertices::Decoration(_) => 2,
        UnitVertices::InlineObject(_) => 4,
    };

    // 主序非递减；同一位置的元素必须相邻并按绘制层排列。
    for pair in output.glyphs.windows(2) {
        let (first, second) = (position(&pair[0]), position(&pair[1]));
        assert!(
            first <= second,
            "输出位置不是非递减的：{first:?} 之后是 {second:?}"
        );
        if first == second {
            assert!(
                rank(&pair[0]) <= rank(&pair[1]),
                "同一位置 {first:?} 的绘制层顺序不符：{} 之后是 {}",
                rank(&pair[0]),
                rank(&pair[1])
            );
        }
    }

    // 逐字推进时，一个字出现之前它的背景必定已经出现，不会留下悬空的文字。
    let end_of = |target: (u32, u32), background: bool| {
        output
            .glyphs
            .iter()
            .position(|element| {
                let same_kind = matches!(element, UnitVertices::Background(_)) == background;
                same_kind && position(element) == target
            })
            .map(|index| index + 1)
    };
    let background_end = end_of((0, 0), true).expect("第一个字的背景");
    let text_end = end_of((0, 0), false).expect("第一个字的正文");
    assert!(
        background_end <= text_end,
        "第一个字的背景在第 {background_end} 个元素，文字却在第 {text_end} 个元素"
    );
}

#[test]
fn background_fragments_share_the_cluster_position() {
    let output = layout(
        "[background color=\"#FFF3BF\" radius=4]甲乙[/background]丙",
        &LayoutStyle::default(),
    );
    let fragments = backgrounds(&output);
    let texts = text_positions(&output);

    assert_eq!(texts, vec![(0, 0), (0, 1), (0, 2)]);
    assert_eq!(fragments.len(), 2);
    // 每个背景片段与它所属 cluster 的文字共用同一组编号，后面的文字才递增。
    assert_eq!(position(&output.glyphs[0]), (0, 0));
    assert_eq!((fragments[0].row, fragments[0].col), (0, 0));
    assert_eq!((fragments[1].row, fragments[1].col), (0, 1));
}

#[test]
fn adjacent_ranges_do_not_merge() {
    let output = layout(
        "[background color=\"#FFF3BF\" radius=4]甲乙[/background][background color=\"#BAE6FD\" radius=4]丙丁[/background]",
        &LayoutStyle::default(),
    );
    let fragments = backgrounds(&output);

    assert_eq!(fragments.len(), 4);
    // 两个相邻但来源不同的范围不会合并：各段保留自己范围的填充色。
    let colors = fragments
        .iter()
        .map(|fragment| fill_color(fragment))
        .collect::<Vec<_>>();
    assert_eq!(colors[0], colors[1], "第一个范围的两段应同色");
    assert_eq!(colors[2], colors[3], "第二个范围的两段应同色");
    assert_ne!(colors[0], colors[2], "不同范围不应被当成同一个图形");
}

#[test]
fn nested_backgrounds_stay_separate() {
    let output = layout(
        "[background color=\"#FFF3BF\" radius=4]甲[background color=\"#BAE6FD\" radius=4]乙[/background]丙[/background]",
        &LayoutStyle::default(),
    );
    let fragments = backgrounds(&output);

    // 内层把外层切成前后两段，但两者仍是同一个 authored range；内层是另一个范围。
    assert_eq!(fragments.len(), 3);
    let colors = fragments
        .iter()
        .map(|fragment| fill_color(fragment))
        .collect::<Vec<_>>();
    assert_eq!(colors[0], colors[2], "外层被切成的两段应同色");
    assert_ne!(colors[0], colors[1], "内层不应与外层同色");
}

#[test]
fn a_range_crossing_lines_covers_each_line() {
    let style = LayoutStyle {
        box_width: Some(64.0),
        ..LayoutStyle::default()
    };
    let output = layout(
        "[background color=\"#FFF3BF\" radius=4]甲乙丙丁[/background]",
        &style,
    );
    let fragments = backgrounds(&output);

    // 字号 32、宽度 64 时每行两个 cluster。
    assert_eq!(fragments.len(), 4);
    let rows: std::collections::HashSet<_> =
        fragments.iter().map(|fragment| fragment.row).collect();
    assert_eq!(rows.len(), 2);
    // 每行的 col 从 0 重新开始。
    assert_eq!(
        fragments
            .iter()
            .map(|fragment| (fragment.row, fragment.col))
            .collect::<Vec<_>>(),
        vec![(0, 0), (0, 1), (1, 0), (1, 1)]
    );
    // 每行的两段拼起来是一条完整的行内背景：后一段从上一段结束处继续。
    for row in [0, 1] {
        let line: Vec<_> = fragments
            .iter()
            .filter(|fragment| fragment.row == row)
            .collect();
        let first = fill_span(line[0]).expect("第一段应有填充顶点");
        let second = fill_span(line[1]).expect("第二段应有填充顶点");
        assert!(
            (first.1 - second.0).abs() <= 0.01 || first.1 <= second.0,
            "第 {row} 行的两段应首尾相接：{first:?} 与 {second:?}"
        );
        assert!(
            first.0 < first.1 && second.0 < second.1,
            "每段应有宽度的顶点"
        );
    }
}

#[test]
fn line_fragments_reuse_the_resolved_center_line() {
    let output = layout(
        "[underline color=\"#1677FF\" thickness=1 pattern=dashed dashLength=3 gapLength=2]甲乙丙[/underline]",
        &LayoutStyle::default(),
    );
    let fragments = lines(&output);

    assert_eq!(fragments.len(), 3);
    assert!(
        fragments
            .iter()
            .all(|fragment| fragment.line_y == fragments[0].line_y)
    );
    // 片段平铺同一条连续线段，且各自都有顶点。
    assert_eq!(fragments[0].left, fragments[0].left.min(fragments[1].left));
    assert!(fragments[1].left >= fragments[0].right);
    assert!(fragments[2].left >= fragments[1].right);
    assert!(
        fragments
            .iter()
            .all(|fragment| !fragment.vertices.fill.is_empty()
                || !fragment.vertices.shadow.is_empty()),
        "每段都应有顶点"
    );
}

#[test]
fn decorations_use_tiqian_geometry() {
    let output = layout(
        "[emphasis]甲[/emphasis][properNoun]乙[/properNoun][bookTitle]丙[/bookTitle][mourning]丁[/mourning]",
        &LayoutStyle::default(),
    );
    let decorations = decorations(&output);

    let dots: Vec<_> = decorations
        .iter()
        .filter(|decoration| matches!(decoration.shape, DecorationShape::Dot { .. }))
        .collect();
    assert_eq!(dots.len(), 1);
    match dots[0].shape {
        DecorationShape::Dot { diameter, .. } => assert!(diameter > 0.0),
        _ => unreachable!(),
    }

    // 专名号与书名号是中心线，示亡号是完整矩形。
    assert!(decorations.iter().any(|decoration| matches!(
        decoration.shape,
        DecorationShape::Frame { top, bottom, .. } if top == bottom
    )));
    assert!(decorations.iter().any(|decoration| matches!(
        decoration.shape,
        DecorationShape::Frame { top, bottom, .. } if bottom > top
    )));
    // 书名号提供波浪参数，其余装饰为 `None`。
    assert_eq!(
        decorations
            .iter()
            .filter(|decoration| decoration.wave.is_some())
            .count(),
        1
    );
}

#[test]
fn inline_object_takes_one_col_and_carries_its_key() {
    let output = layout(
        "甲[object id=\"icon\" alt=\"图标\" width=16 ascent=12 descent=4 /]乙",
        &LayoutStyle::default(),
    );

    let object = output
        .glyphs
        .iter()
        .find_map(|element| match element {
            UnitVertices::InlineObject(object) => Some(object),
            _ => None,
        })
        .expect("rich text with an object must output an inline object");
    assert_eq!(object.id.as_deref(), Some("icon"));
    assert_eq!(object.alt, "图标");
    // 对象占一个 col，因此第二个文字在其之后。
    assert_eq!((object.row, object.col), (0, 1));
    assert_eq!(text_positions(&output), vec![(0, 0), (0, 2)]);
}

#[test]
fn object_without_id_still_lays_out() {
    let output = layout(
        "甲[object alt=\"图标\" width=16 ascent=12 descent=4 /]乙",
        &LayoutStyle::default(),
    );
    let object = output
        .glyphs
        .iter()
        .find_map(|element| match element {
            UnitVertices::InlineObject(object) => Some(object),
            _ => None,
        })
        .expect("object must still be output without an id");
    assert!(object.id.is_none());
    assert!(output.interactions.is_empty());
}

#[test]
fn ruby_and_bopomofo_follow_their_base_cluster() {
    let output = layout(
        "[ruby text=\"tí qiàn\"]提[/ruby][bopomofo text=\"ㄑㄧㄢˋ\"]椠[/bopomofo]",
        &LayoutStyle::default(),
    );

    let ruby: Vec<_> = output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Text(text) if text.role == huozi::glyph_vertices::TextRole::Ruby => {
                Some(text)
            }
            _ => None,
        })
        .collect();
    let bopomofo: Vec<_> = output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Text(text) if text.role == huozi::glyph_vertices::TextRole::Bopomofo => {
                Some(text)
            }
            _ => None,
        })
        .collect();

    assert!(!ruby.is_empty(), "ruby glyphs must be replayed");
    assert!(!bopomofo.is_empty(), "bopomofo glyphs must be replayed");
    assert!(ruby.iter().all(|text| (text.row, text.col) == (0, 0)));
    assert!(bopomofo.iter().all(|text| (text.row, text.col) == (0, 1)));
}

#[test]
fn multiple_paragraphs_accumulate_rows_and_offsets() {
    let output = layout("甲[br /]乙[br /]丙", &LayoutStyle::default());
    let text_positions: Vec<_> = output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Text(text) => Some((text.row, text.y, text.fill[0].position[1])),
            _ => None,
        })
        .collect();

    assert_eq!(text_positions.len(), 3);
    assert_eq!(text_positions[0].0, 0);
    assert_eq!(text_positions[1].0, 1);
    assert_eq!(text_positions[2].0, 2);
    assert!(text_positions[1].2 > text_positions[0].2);
    assert!(text_positions[2].2 > text_positions[1].2);
}

#[test]
fn consecutive_breaks_produce_a_blank_row() {
    let output = layout("甲[br /][br /]乙", &LayoutStyle::default());

    // 空段不产生元素，但占一行高度。
    assert_eq!(text_positions(&output), vec![(0, 0), (2, 0)]);
}

#[test]
fn output_stops_at_the_first_line_that_does_not_fit() {
    // 默认字号 32、行高倍率 1.5，因此一行高 48。
    let too_short = LayoutStyle {
        box_height: Some(40.0),
        ..LayoutStyle::default()
    };
    let empty = layout("甲[br /]乙", &too_short);
    assert!(empty.glyphs.is_empty());
    assert_eq!(empty.height, 0);

    // 放得下第一行、放不下第二行时只输出第一段。
    let one_line = LayoutStyle {
        box_height: Some(60.0),
        ..LayoutStyle::default()
    };
    let output = layout("甲[br /]乙", &one_line);
    assert_eq!(text_positions(&output), vec![(0, 0)]);
    assert!(output.height <= 60);
}

#[test]
fn segment_glyph_spans_cover_all_draw_elements() {
    let output = layout_with_id(
        SegmentId::Lite(5),
        "[background color=\"#FFF3BF\" radius=4]甲[/background][underline color=\"#1677FF\"]乙[/underline]",
    );

    assert_eq!(output.segment_glyph_spans.len(), 1);
    let span = &output.segment_glyph_spans[0];
    assert_eq!(span.segment_id, SegmentId::Lite(5));
    assert_eq!(span.glyph_range, 0..output.glyphs.len());
}

#[test]
fn a_segment_that_produces_no_elements_gets_no_span() {
    let output = engine()
        .layout_parse(
            &vec![
                Segment {
                    id: Some(SegmentId::Lite(1)),
                    content: "甲".into(),
                },
                Segment {
                    id: Some(SegmentId::Lite(2)),
                    content: String::new().into(),
                },
            ],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    assert_eq!(output.segment_glyph_spans.len(), 1);
    assert_eq!(output.segment_glyph_spans[0].segment_id, SegmentId::Lite(1));
}

#[test]
fn one_segment_id_appearing_twice_gets_two_spans() {
    let output = engine()
        .layout_parse(
            &vec![
                Segment {
                    id: Some(SegmentId::Lite(1)),
                    content: "甲".into(),
                },
                Segment {
                    id: Some(SegmentId::Lite(2)),
                    content: "乙".into(),
                },
                Segment {
                    id: Some(SegmentId::Lite(1)),
                    content: "丙".into(),
                },
            ],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap();

    // 同一个 SegmentId 在主序列中不连续时不合并，各自产生一个 span。
    assert_eq!(output.segment_glyph_spans.len(), 3);
    let ids = output
        .segment_glyph_spans
        .iter()
        .map(|span| span.segment_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![SegmentId::Lite(1), SegmentId::Lite(2), SegmentId::Lite(1)]
    );

    // span 平铺整条主序列：下标连续、首尾相接，且不重叠。
    let mut cursor = 0;
    for span in &output.segment_glyph_spans {
        assert_eq!(span.glyph_range.start, cursor);
        assert!(!span.glyph_range.is_empty());
        cursor = span.glyph_range.end;
    }
    assert_eq!(cursor, output.glyphs.len());
}

#[test]
fn link_areas_cover_every_positioned_cluster() {
    let output = layout(
        "[link id=\"a\" target=\"https://example.com\"]甲乙[/link]",
        &LayoutStyle::default(),
    );

    assert_eq!(output.interactions.len(), 1);
    let interaction = &output.interactions[0];
    assert_eq!(interaction.id, "a");
    assert_eq!(interaction.areas.len(), 2);
    for (area, element) in interaction.areas.iter().zip(output.glyphs.iter()) {
        assert_eq!((area.row, area.col), position(element));
        assert!(area.rect.width() > 0.0);
    }
}

#[test]
fn every_line_has_a_positive_height() {
    let output = layout("甲乙丙", &LayoutStyle::default());
    let text = match &output.glyphs[0] {
        UnitVertices::Text(text) => text,
        other => panic!("expected text, got {other:?}"),
    };
    assert!(text.height > 0);
    assert!(text.width > 0);
}

/// 同一批输入重复布局时，元素的变体分布与逐字位置必须稳定。
#[test]
fn repeated_layout_is_stable() {
    let text =
        "[background color=\"#FFF3BF\" radius=4]甲[/background][properNoun]乙[/properNoun]丙";
    let first = layout(text, &LayoutStyle::default());
    let second = layout(text, &LayoutStyle::default());

    let summarize = |output: &huozi::layout::RichTextLayoutOutput| {
        let mut counts = HashMap::new();
        for element in &output.glyphs {
            let name = match element {
                UnitVertices::Text(_) => "text",
                UnitVertices::Background(_) => "background",
                UnitVertices::Line(_) => "line",
                UnitVertices::Decoration(_) => "decoration",
                UnitVertices::InlineObject(_) => "object",
            };
            *counts.entry(name).or_insert(0_usize) += 1;
        }
        (
            counts,
            output.glyphs.iter().map(position).collect::<Vec<_>>(),
            output.width,
            output.height,
        )
    };
    assert_eq!(summarize(&first), summarize(&second));
}
