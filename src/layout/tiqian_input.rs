use csscolorparser::Color;
use tiqian::api::{
    InlineBoxStyle, InlineObjectMetrics, ParagraphBuilder, RubyAnnotation, TextStyleOverride,
};
use tiqian::core::geometry::{LayoutConstraints, ScalarOffset, TextRange};
use tiqian::core::text_model::{
    DecorationKind as TiqianDecorationKind, InlineAttachment as TiqianInlineAttachment,
    InlineBoxOuterSpacing, InlineObjectBoundaryAdjustment,
    LastLineAlignment as TiqianLastLineAlignment, LayoutInput, LineLengthGrid, ParagraphStyle,
    RichTextBackgroundMetricPolicy, RichTextBackgroundPaint, RichTextLinePaint,
    RichTextLinePattern, RichTextPaint, RubyLineHeightMode as TiqianRubyLineHeightMode,
    TextStyle as TiqianTextStyle,
};
use tiqian::core::units::Ic;

use crate::layout::LayoutStyle;
use crate::parser::{
    BackgroundMetricPolicy, DecorationKind, InlineBoxSpacing, InlineNode, InlineObject,
    InlineScopeKind, LastLineAlignment, LinePattern, LineStyle, ParagraphStyleOverride,
    ParsedParagraph, RubyKind, RubyLineHeightMode, SourceRange, TextSpan, TextStyle,
};

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

    pub(crate) fn adapt_paragraph(
        paragraph: &ParsedParagraph,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
    ) -> HuoziTiqianInput {
        let max_width = layout_style
            .box_width
            .map_or(f32::INFINITY, |width| width as f32);
        let constraints = match layout_style.box_height {
            Some(max_height) => LayoutConstraints::with_max_height(max_width, max_height as f32),
            None => LayoutConstraints::with_defaults(max_width),
        };
        let mut builder = ParagraphBuilder::new(constraints);
        builder
            .text_style(tiqian_text_style(initial_text_style))
            .paragraph_style(paragraph_style(
                layout_style,
                initial_text_style,
                &paragraph.paragraph_style,
            ));

        let mut source_map_entries = Vec::new();
        let mut display_offset = 0_i32;
        append_nodes(
            &mut builder,
            &paragraph.nodes,
            &mut source_map_entries,
            &mut display_offset,
            false,
        );

        HuoziTiqianInput {
            layout_input: builder
                .build()
                .expect("Huozi 结构化输入不会留下未关闭的 tiqian builder scope"),
            source_map: HuoziSourceMap {
                entries: source_map_entries,
            },
        }
    }
}

fn append_nodes(
    builder: &mut ParagraphBuilder,
    nodes: &[InlineNode],
    source_map_entries: &mut Vec<HuoziSourceMapEntry>,
    display_offset: &mut i32,
    inside_ruby: bool,
) {
    for node in nodes {
        match node {
            InlineNode::Text(run) => {
                let length = run.text.chars().count() as i32;
                let range = TextRange::new(
                    ScalarOffset::new(*display_offset),
                    ScalarOffset::new(*display_offset + length),
                );
                let style = tiqian_text_style_override(&run.style);
                let paints = tiqian_paints(&run.style);
                builder.with_text_style(style, |builder| {
                    builder.with_paints(&paints, |builder| builder.push(&run.text));
                });
                source_map_entries.push(HuoziSourceMapEntry {
                    display_range: range,
                    source_range: run.source_range.clone(),
                });
                *display_offset += length;
            }
            InlineNode::Object(object) if inside_ruby => {
                log::warn!("inline object inside ruby ignored");
            }
            InlineNode::Object(object) => {
                append_object(builder, object, source_map_entries, display_offset)
            }
            InlineNode::Scope { kind, children } if children.is_empty() => {
                log::warn!("empty rich-text scope ignored");
            }
            InlineNode::Scope { kind, children } => match kind {
                InlineScopeKind::Background(style) => {
                    let background = tiqian_background(style);
                    let paints = tiqian_paints_from_parts(
                        &style.fill_color,
                        style.stroke.as_ref(),
                        style.shadow.as_ref(),
                    );
                    builder.with_background(background, &paints, |builder| {
                        append_nodes(
                            builder,
                            children,
                            source_map_entries,
                            display_offset,
                            inside_ruby,
                        )
                    });
                }
                InlineScopeKind::Underline(style) => {
                    let paints = tiqian_paints_from_parts(
                        &style.fill_color,
                        style.stroke.as_ref(),
                        style.shadow.as_ref(),
                    );
                    builder.with_paints(&paints, |builder| {
                        builder.with_underline(tiqian_line(style), |builder| {
                            append_nodes(
                                builder,
                                children,
                                source_map_entries,
                                display_offset,
                                inside_ruby,
                            )
                        });
                    });
                }
                InlineScopeKind::LineThrough(style) => {
                    let paints = tiqian_paints_from_parts(
                        &style.fill_color,
                        style.stroke.as_ref(),
                        style.shadow.as_ref(),
                    );
                    builder.with_paints(&paints, |builder| {
                        builder.with_line_through(tiqian_line(style), |builder| {
                            append_nodes(
                                builder,
                                children,
                                source_map_entries,
                                display_offset,
                                inside_ruby,
                            )
                        });
                    });
                }
                InlineScopeKind::Ruby(style) => {
                    let annotation = match style.kind {
                        RubyKind::Pinyin => RubyAnnotation::builder(&style.text),
                        RubyKind::Bopomofo => RubyAnnotation::builder(&style.text)
                            .kind(tiqian::core::text_model::RubyKind::Bopomofo),
                    }
                    .font_families(style.font_families.clone())
                    .locale(style.locale.clone())
                    .build();
                    builder.with_ruby(annotation, |builder| {
                        append_nodes(builder, children, source_map_entries, display_offset, true)
                    });
                }
                InlineScopeKind::Decoration(kind) => {
                    builder.with_decoration(tiqian_decoration(*kind), |builder| {
                        append_nodes(
                            builder,
                            children,
                            source_map_entries,
                            display_offset,
                            inside_ruby,
                        )
                    });
                }
                InlineScopeKind::Link { target } => {
                    builder.with_link(target.clone(), |builder| {
                        append_nodes(
                            builder,
                            children,
                            source_map_entries,
                            display_offset,
                            inside_ruby,
                        )
                    });
                }
                InlineScopeKind::Technical => {
                    builder.with_technical(|builder| {
                        append_nodes(
                            builder,
                            children,
                            source_map_entries,
                            display_offset,
                            inside_ruby,
                        )
                    });
                }
                InlineScopeKind::InlineCode(style) => {
                    let paints = tiqian_paints_from_parts(
                        &style.background.fill_color,
                        style.background.stroke.as_ref(),
                        style.background.shadow.as_ref(),
                    );
                    builder.with_paints(&paints, |builder| {
                        builder.with_inline_code(
                            tiqian_text_style_override(&style.text_style),
                            tiqian_background(&style.background),
                            |builder| {
                                append_nodes(
                                    builder,
                                    children,
                                    source_map_entries,
                                    display_offset,
                                    inside_ruby,
                                )
                            },
                        );
                    });
                }
                InlineScopeKind::AutoSpaceSuppressed => {
                    builder.with_auto_space_suppressed(|builder| {
                        append_nodes(
                            builder,
                            children,
                            source_map_entries,
                            display_offset,
                            inside_ruby,
                        )
                    });
                }
                InlineScopeKind::InlineBox(style) => {
                    builder.with_inline_box(
                        InlineBoxStyle::with_all(
                            style.start,
                            style.end,
                            match style.spacing {
                                InlineBoxSpacing::Narrow => InlineBoxOuterSpacing::Narrow,
                                InlineBoxSpacing::Source => InlineBoxOuterSpacing::Source,
                            },
                        ),
                        |builder| {
                            append_nodes(
                                builder,
                                children,
                                source_map_entries,
                                display_offset,
                                inside_ruby,
                            )
                        },
                    );
                }
            },
        }
    }
}

fn append_object(
    builder: &mut ParagraphBuilder,
    object: &InlineObject,
    source_map_entries: &mut Vec<HuoziSourceMapEntry>,
    display_offset: &mut i32,
) {
    let length = object.alt.chars().count() as i32;
    let range = TextRange::new(
        ScalarOffset::new(*display_offset),
        ScalarOffset::new(*display_offset + length),
    );
    let metrics = InlineObjectMetrics::builder(object.width, object.ascent, object.descent)
        .leading_boundary(InlineObjectBoundaryAdjustment::FIXED)
        .trailing_boundary(InlineObjectBoundaryAdjustment::FIXED)
        .build();
    if builder.inline_object(&object.alt, metrics).is_ok() {
        source_map_entries.push(HuoziSourceMapEntry {
            display_range: range,
            source_range: object.source_range.clone(),
        });
        *display_offset += length;
    } else {
        log::warn!("inline object ignored by tiqian builder");
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
        .font_families(style.font_families.clone())
        .font_size(style.font_size as f32)
        .locale(style.locale.clone())
        .font_weight(style.font_weight)
        .italic(style.italic)
        .baseline_shift(style.baseline_shift)
        .inline_attachment(match style.inline_attachment {
            crate::parser::InlineAttachment::None => TiqianInlineAttachment::None,
            crate::parser::InlineAttachment::Previous => TiqianInlineAttachment::Previous,
        })
        .build()
}

fn tiqian_text_style_override(style: &TextStyle) -> TextStyleOverride {
    TextStyleOverride::builder()
        .font_families(style.font_families.clone())
        .font_size(style.font_size as f32)
        .locale(style.locale.clone())
        .font_weight(style.font_weight)
        .italic(style.italic)
        .baseline_shift(style.baseline_shift)
        .inline_attachment(match style.inline_attachment {
            crate::parser::InlineAttachment::None => TiqianInlineAttachment::None,
            crate::parser::InlineAttachment::Previous => TiqianInlineAttachment::Previous,
        })
        .build()
}

fn paragraph_style(
    layout_style: &LayoutStyle,
    initial_text_style: &TextStyle,
    override_style: &ParagraphStyleOverride,
) -> ParagraphStyle {
    ParagraphStyle::builder()
        .line_height(override_style.line_height.or(Some(
            (initial_text_style.font_size * layout_style.line_height) as f32,
        )))
        .first_line_indent(Some(Ic {
            count: override_style.indent.unwrap_or(layout_style.indent as f32),
        }))
        .block_indent(Ic {
            count: override_style.block_indent.unwrap_or(0.0),
        })
        .last_line_alignment(match override_style.last_line_alignment {
            Some(LastLineAlignment::Start) | None => TiqianLastLineAlignment::Start,
            Some(LastLineAlignment::Center) => TiqianLastLineAlignment::Center,
            Some(LastLineAlignment::End) => TiqianLastLineAlignment::End,
        })
        .line_length_grid(LineLengthGrid::with_enabled(
            override_style.line_length_grid.unwrap_or(true),
        ))
        .ruby_line_height_mode(match override_style.ruby_line_height_mode {
            Some(RubyLineHeightMode::UniformParagraph) => {
                TiqianRubyLineHeightMode::UniformParagraph
            }
            Some(RubyLineHeightMode::PerLine) | None => TiqianRubyLineHeightMode::PerLine,
        })
        .inline_object_minimum_clearance_em(
            override_style
                .inline_object_minimum_clearance
                .unwrap_or(0.1),
        )
        .emphasis_dot_gap_em(override_style.emphasis_dot_gap.unwrap_or(0.1))
        .build()
}

fn tiqian_background(style: &crate::parser::BackgroundStyle) -> RichTextBackgroundPaint {
    let mut builder = RichTextBackgroundPaint::builder()
        .horizontal_padding(style.padding_x)
        .vertical_padding(style.padding_y)
        .corner_radius(style.radius)
        .metric_policy(match style.metric_policy {
            BackgroundMetricPolicy::MarkedFaces => RichTextBackgroundMetricPolicy::MarkedFaces,
            BackgroundMetricPolicy::UniformTextStyle => {
                RichTextBackgroundMetricPolicy::UniformTextStyle
            }
            BackgroundMetricPolicy::UniformParagraphStyle => {
                RichTextBackgroundMetricPolicy::UniformParagraphStyle
            }
        })
        .adjacent_same_style_clearance(style.clearance);
    if let Some(radius) = style.continuation_radius {
        builder = builder.continuation_corner_radius(radius);
    }
    builder.build()
}

fn tiqian_line(style: &LineStyle) -> RichTextLinePaint {
    RichTextLinePaint {
        thickness: style.thickness,
        pattern: match &style.pattern {
            LinePattern::Solid => RichTextLinePattern::Solid,
            LinePattern::Dashed {
                dash_length,
                gap_length,
            } => RichTextLinePattern::Dashed {
                dash_length: *dash_length,
                gap_length: *gap_length,
            },
            LinePattern::Dotted { gap_length } => RichTextLinePattern::Dotted {
                gap_length: *gap_length,
            },
        },
        adjacent_same_style_clearance: style.clearance,
    }
}

fn tiqian_decoration(kind: DecorationKind) -> TiqianDecorationKind {
    match kind {
        DecorationKind::Emphasis => TiqianDecorationKind::Emphasis,
        DecorationKind::Mourning => TiqianDecorationKind::Mourning,
        DecorationKind::ProperNoun => TiqianDecorationKind::ProperNoun,
        DecorationKind::BookTitle => TiqianDecorationKind::BookTitle,
    }
}

fn tiqian_paints(style: &TextStyle) -> Vec<RichTextPaint> {
    tiqian_paints_from_parts(
        &style.fill_color,
        style.stroke.as_ref(),
        style.shadow.as_ref(),
    )
}

fn tiqian_paints_from_parts(
    fill_color: &Color,
    stroke: Option<&crate::parser::StrokeStyle>,
    shadow: Option<&crate::parser::ShadowStyle>,
) -> Vec<RichTextPaint> {
    let mut paints = vec![RichTextPaint::Fill {
        argb: color_to_argb(fill_color),
    }];

    if let Some(stroke) = stroke {
        paints.push(RichTextPaint::Stroke {
            argb: color_to_argb(&stroke.stroke_color),
            width: stroke.stroke_width,
        });
    }

    if let Some(shadow) = shadow {
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
    use crate::parser::{
        BackgroundStyle, InlineNode, InlineObject, InlineObjectBoundary, InlineScopeKind,
        ParsedParagraph, ScalarOffset as HuoziScalarOffset, SegmentId, TextRun,
    };

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

    #[test]
    fn adapts_structured_scope_object_and_source_ranges() {
        let text_run = TextRun {
            text: "甲".to_string(),
            style: TextStyle::default(),
            source_range: SourceRange {
                segment_id: Some(SegmentId::Lite(1)),
                start: HuoziScalarOffset(5),
                end: HuoziScalarOffset(6),
            },
        };
        let paragraph = ParsedParagraph {
            nodes: vec![
                InlineNode::Scope {
                    kind: InlineScopeKind::Background(BackgroundStyle::default()),
                    children: vec![InlineNode::Scope {
                        kind: InlineScopeKind::Link {
                            target: "https://example.com".to_string(),
                        },
                        children: vec![InlineNode::Text(text_run)],
                    }],
                },
                InlineNode::Object(InlineObject {
                    alt: "锚".to_string(),
                    width: 12.0,
                    ascent: 9.0,
                    descent: 3.0,
                    leading_boundary: InlineObjectBoundary::Fixed,
                    trailing_boundary: InlineObjectBoundary::Fixed,
                    source_range: SourceRange {
                        segment_id: Some(SegmentId::Lite(2)),
                        start: HuoziScalarOffset(8),
                        end: HuoziScalarOffset(20),
                    },
                }),
            ],
            ..Default::default()
        };

        let input = HuoziTiqianInputAdapter::adapt_paragraph(
            &paragraph,
            &LayoutStyle::default(),
            &TextStyle::default(),
        );

        assert_eq!(input.layout_input.content.text.as_str(), "甲锚");
        assert_eq!(input.layout_input.inline_objects.len(), 1);
        assert!(input.layout_input.rich_text.iter().any(|span| {
            span.semantics.iter().any(|semantic| {
                matches!(
                    semantic,
                    tiqian::core::text_model::RichTextSemantic::Link { target }
                        if target == "https://example.com"
                )
            })
        }));
        assert_eq!(input.source_map.entries.len(), 2);
        assert_eq!(
            input.source_map.entries[0].display_range,
            TextRange::new(ScalarOffset::new(0), ScalarOffset::new(1))
        );
        assert_eq!(
            input.source_map.entries[1].source_range.segment_id,
            Some(SegmentId::Lite(2))
        );
    }
}
