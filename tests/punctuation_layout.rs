use huozi::{
    Huozi,
    layout::{ColorSpace, LayoutStyle},
    parser::{Segment, SegmentId, SourceRange, TextRun, TextSpan, TextStyle},
};

const TEST_FONT: &[u8] = include_bytes!("../examples/assets/SourceHanSansSC-Regular.otf");

fn engine() -> Huozi {
    Huozi::new(TEST_FONT.to_vec())
}

fn text_style() -> TextStyle {
    TextStyle {
        font_size: 32.0,
        line_height: 1.0,
        ..Default::default()
    }
}

fn layout_style(width_in_em: f64) -> LayoutStyle {
    LayoutStyle {
        box_width: width_in_em * 32.0,
        box_height: 320.0,
        glyph_grid_size: 32.0,
        ..Default::default()
    }
}

#[test]
fn punctuation_compression_is_opt_in() {
    let segments = vec![Segment::dummy("，。")];
    let text_style = text_style();

    let (plain, _, plain_width, _) = engine()
        .layout_plain(
            &segments,
            &layout_style(10.0),
            &text_style,
            ColorSpace::SRGB,
        )
        .unwrap();

    let mut adjusted_style = layout_style(10.0);
    adjusted_style.punctuation.compression = true;
    let (adjusted, _, adjusted_width, _) = engine()
        .layout_plain(&segments, &adjusted_style, &text_style, ColorSpace::SRGB)
        .unwrap();

    assert_eq!(plain[1].x - plain[0].x, plain[0].width);
    assert_eq!(adjusted[1].x - adjusted[0].x, adjusted[0].width / 2);
    assert_eq!(plain_width, 64);
    assert_eq!(adjusted_width, 48);
}

#[test]
fn punctuation_compression_crosses_rich_text_runs_and_segments() {
    let style = text_style();
    let spans = vec![TextSpan {
        span_id: None,
        runs: vec![
            TextRun {
                text: "」".to_string(),
                style: style.clone(),
                source_range: SourceRange {
                    segment_id: Some(SegmentId::Lite(1)),
                    start: 0,
                    end: 3,
                },
            },
            TextRun {
                text: "「".to_string(),
                style,
                source_range: SourceRange {
                    segment_id: Some(SegmentId::Lite(2)),
                    start: 0,
                    end: 3,
                },
            },
        ],
    }];
    let mut layout_style = layout_style(10.0);
    layout_style.punctuation.compression = true;

    let (glyphs, segment_spans, total_width, _) =
        engine().layout(&layout_style, &spans, ColorSpace::SRGB);

    assert_eq!(glyphs[1].x - glyphs[0].x, glyphs[0].width / 2);
    assert_eq!(segment_spans.len(), 2);
    assert_eq!(segment_spans[0].glyph_range, 0..1);
    assert_eq!(segment_spans[1].glyph_range, 1..2);
    assert_eq!(total_width, 48);
}

#[test]
fn line_end_punctuation_hangs_by_at_most_half_an_em() {
    let segments = vec![Segment::dummy("中文。")];
    let text_style = text_style();

    let (wrapped, _, _, _) = engine()
        .layout_plain(&segments, &layout_style(2.5), &text_style, ColorSpace::SRGB)
        .unwrap();

    let mut hanging_style = layout_style(2.5);
    hanging_style.punctuation.hanging = true;
    let (hanging, _, total_width, _) = engine()
        .layout_plain(&segments, &hanging_style, &text_style, ColorSpace::SRGB)
        .unwrap();

    assert_eq!(wrapped[2].row, 1);
    assert_eq!(hanging[2].row, 0);
    assert_eq!(hanging[2].x + hanging[2].width, 3 * hanging[2].width);
    assert_eq!(total_width, 96);
}

#[test]
fn exact_fit_stays_on_line_and_repeated_punctuation_does_not_stack_hanging() {
    let text_style = text_style();
    let exact_fit = vec![Segment::dummy("中文")];
    let (glyphs, _, total_width, _) = engine()
        .layout_plain(
            &exact_fit,
            &layout_style(2.0),
            &text_style,
            ColorSpace::SRGB,
        )
        .unwrap();

    assert_eq!(glyphs[1].row, 0);
    assert_eq!(total_width, 64);

    let repeated = vec![Segment::dummy("中。。。。")];
    let mut adjusted_style = layout_style(2.5);
    adjusted_style.punctuation.compression = true;
    adjusted_style.punctuation.hanging = true;
    let (glyphs, _, _, _) = engine()
        .layout_plain(&repeated, &adjusted_style, &text_style, ColorSpace::SRGB)
        .unwrap();

    assert_eq!(glyphs[3].row, 0);
    assert_eq!(glyphs[4].row, 1);
}
