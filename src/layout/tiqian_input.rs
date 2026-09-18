use csscolorparser::Color;
use tiqian::api::{ParagraphBuilder, TextStyleOverride};
use tiqian::core::geometry::{LayoutConstraints, ScalarOffset, TextRange};
use tiqian::core::text_model::{
    LayoutInput, ParagraphStyle, RichTextPaint, TextStyle as TiqianTextStyle,
};
use tiqian::core::units::Ic;

use crate::layout::LayoutStyle;
use crate::parser::{SourceRange, TextSpan, TextStyle};

pub(crate) struct HuoziTiqianInput {
    pub(crate) layout_input: LayoutInput,
    pub(crate) source_map: HuoziSourceMap,
}

pub(crate) struct HuoziTiqianInputAdapter;

impl HuoziTiqianInputAdapter {
    pub(crate) fn adapt(
        text_spans: &[TextSpan],
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
    ) -> HuoziTiqianInput {
        let mut source_map_entries = Vec::new();
        let mut display_offset = 0_i32;

        let max_width = layout_style
            .box_width
            .map_or(f32::INFINITY, |width| width as f32);
        let constraints = match layout_style.box_height {
            Some(max_height) => LayoutConstraints::with_max_height(max_width, max_height as f32),
            None => LayoutConstraints::with_defaults(max_width),
        };
        let paragraph_style = ParagraphStyle::builder()
            .line_height(Some(
                (initial_text_style.font_size * layout_style.line_height) as f32,
            ))
            .first_line_indent(Some(Ic {
                count: layout_style.indent as f32,
            }))
            .build();
        let mut builder = ParagraphBuilder::new(constraints);
        builder
            .text_style(tiqian_text_style(initial_text_style))
            .paragraph_style(paragraph_style);

        for text_span in text_spans {
            for text_run in &text_span.runs {
                let run_length = text_run.text.chars().count() as i32;
                let range = TextRange::new(
                    ScalarOffset::new(display_offset),
                    ScalarOffset::new(display_offset + run_length),
                );
                let style = TextStyleOverride::builder()
                    .font_size(text_run.style.font_size as f32)
                    .build();
                let paints = tiqian_paints(&text_run.style);

                builder.with_text_style(style, |builder| {
                    builder.with_paints(&paints, |builder| builder.push(&text_run.text));
                });
                source_map_entries.push(HuoziSourceMapEntry {
                    display_range: range,
                    source_range: text_run.source_range.clone(),
                });
                display_offset += run_length;
            }
        }

        HuoziTiqianInput {
            layout_input: builder
                .build()
                .expect("Huozi 输入转换不会留下未关闭的 tiqian builder scope"),
            source_map: HuoziSourceMap {
                entries: source_map_entries,
            },
        }
    }
}

pub(crate) struct HuoziSourceMap {
    pub(crate) entries: Vec<HuoziSourceMapEntry>,
}

pub(crate) struct HuoziSourceMapEntry {
    pub(crate) display_range: TextRange,
    pub(crate) source_range: SourceRange,
}

fn tiqian_text_style(style: &TextStyle) -> TiqianTextStyle {
    TiqianTextStyle::builder()
        .font_size(style.font_size as f32)
        .build()
}

fn tiqian_paints(style: &TextStyle) -> Vec<RichTextPaint> {
    let mut paints = vec![RichTextPaint::Fill {
        argb: color_to_argb(&style.fill_color),
    }];

    if let Some(stroke) = &style.stroke {
        paints.push(RichTextPaint::Stroke {
            argb: color_to_argb(&stroke.stroke_color),
            width: stroke.stroke_width,
        });
    }

    if let Some(shadow) = &style.shadow {
        paints.push(RichTextPaint::Shadow {
            argb: color_to_argb(&shadow.shadow_color),
            offset_x: shadow.shadow_offset_x,
            offset_y: shadow.shadow_offset_y,
            blur_radius: shadow.shadow_blur,
            spread_radius: shadow.shadow_width,
        });
    }

    paints
}

fn color_to_argb(color: &Color) -> i32 {
    let [red, green, blue, alpha] = color.to_rgba8();
    u32::from_be_bytes([alpha, red, green, blue]) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ScalarOffset as HuoziScalarOffset, SegmentId, TextRun};

    #[test]
    fn preserves_run_geometry_paint_and_source_identity() {
        let first_run = TextRun {
            text: "你".to_string(),
            style: TextStyle::default(),
            source_range: SourceRange {
                segment_id: Some(SegmentId::Lite(1)),
                start: HuoziScalarOffset(3),
                end: HuoziScalarOffset(4),
            },
        };
        let second_run = TextRun {
            text: "好".to_string(),
            style: TextStyle {
                font_size: 48.0,
                stroke: Some(crate::parser::StrokeStyle {
                    stroke_color: Color::from_rgba8(255, 0, 0, 255),
                    stroke_width: 2.0,
                }),
                ..TextStyle::default()
            },
            source_range: SourceRange {
                segment_id: Some(SegmentId::Lite(2)),
                start: HuoziScalarOffset(5),
                end: HuoziScalarOffset(6),
            },
        };
        let input = HuoziTiqianInputAdapter::adapt(
            &[TextSpan {
                runs: vec![first_run, second_run],
                span_id: None,
            }],
            &LayoutStyle::default(),
            &TextStyle::default(),
        );

        assert_eq!(input.layout_input.content.text.as_str(), "你好");
        assert_eq!(input.layout_input.content.spans.len(), 1);
        assert_eq!(input.layout_input.content.spans[0].style.font_size, 48.0);
        assert_eq!(input.layout_input.rich_text.len(), 2);
        assert_eq!(input.source_map.entries.len(), 2);
        assert_eq!(
            input.source_map.entries[0].display_range,
            TextRange::new(ScalarOffset::new(0), ScalarOffset::new(1))
        );
        assert_eq!(
            input.source_map.entries[1].display_range,
            TextRange::new(ScalarOffset::new(1), ScalarOffset::new(2))
        );
        assert_eq!(
            input.source_map.entries[0].source_range.segment_id,
            Some(SegmentId::Lite(1))
        );
        assert_eq!(
            input.source_map.entries[0].source_range.start,
            HuoziScalarOffset(3)
        );
        assert_eq!(
            input.source_map.entries[0].source_range.end,
            HuoziScalarOffset(4)
        );
        assert_eq!(
            input.source_map.entries[1].source_range.segment_id,
            Some(SegmentId::Lite(2))
        );
        assert_eq!(
            input.source_map.entries[1].source_range.start,
            HuoziScalarOffset(5)
        );
        assert_eq!(
            input.source_map.entries[1].source_range.end,
            HuoziScalarOffset(6)
        );
        assert_eq!(
            input.layout_input.rich_text[1].range,
            TextRange::new(ScalarOffset::new(1), ScalarOffset::new(2))
        );

        let paints = &input.layout_input.rich_text[1].layers[0].paints;
        assert_eq!(
            paints[0],
            RichTextPaint::Fill {
                argb: 0xFF000000_u32 as i32,
            }
        );
        assert_eq!(
            paints[1],
            RichTextPaint::Stroke {
                argb: 0xFFFF0000_u32 as i32,
                width: 2.0,
            }
        );
    }

    #[test]
    fn coalesces_adjacent_runs_with_identical_tiqian_style_and_paint() {
        let first_run = TextRun {
            text: "你".to_string(),
            style: TextStyle::default(),
            source_range: SourceRange::default(),
        };
        let second_run = TextRun {
            text: "好".to_string(),
            style: TextStyle::default(),
            source_range: SourceRange::default(),
        };
        let input = HuoziTiqianInputAdapter::adapt(
            &[TextSpan {
                runs: vec![first_run, second_run],
                span_id: None,
            }],
            &LayoutStyle::default(),
            &TextStyle::default(),
        );

        assert_eq!(input.layout_input.content.text.as_str(), "你好");
        assert!(input.layout_input.content.spans.is_empty());
        assert_eq!(input.layout_input.rich_text.len(), 1);
        assert_eq!(
            input.layout_input.rich_text[0].range,
            TextRange::new(ScalarOffset::new(0), ScalarOffset::new(2))
        );
        assert_eq!(input.source_map.entries.len(), 2);
        assert_eq!(
            input.source_map.entries[0].display_range,
            TextRange::new(ScalarOffset::new(0), ScalarOffset::new(1))
        );
        assert_eq!(
            input.source_map.entries[1].display_range,
            TextRange::new(ScalarOffset::new(1), ScalarOffset::new(2))
        );
    }

    #[test]
    fn converts_colors_to_argb() {
        assert_eq!(
            color_to_argb(&Color::from_rgba8(0x12, 0x34, 0x56, 0x78)),
            0x7812_3456_u32 as i32
        );
    }
}
