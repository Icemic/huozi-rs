use std::collections::HashMap;
use std::str::FromStr;

use crate::layout::ParagraphAlignment;
use crate::parser::{
    Attribute, BackgroundMetricPolicy, BackgroundStyle, DecorationKind, Element, FontSynthesis,
    InlineAttachment, InlineBoxSpacing, InlineBoxStyle, InlineCodeStyle, InlineNode, InlineObject,
    InlineObjectBoundary, InlineScopeKind, LinePattern, LineStyle, ParagraphStyleOverride,
    ParsedParagraph, ParsedText, RubyKind, RubyLineHeightMode, RubyStyle, ShadowStyle, SourceRange,
    StrokeStyle, TextRun, TextStyle,
};

pub(crate) fn lower_elements(
    elements: Vec<Element>,
    initial_style: &TextStyle,
    style_prefabs: Option<&HashMap<String, TextStyle>>,
) -> ParsedText {
    ParsedText {
        paragraphs: lower_sequence(
            &elements,
            initial_style,
            ParagraphStyleOverride::default(),
            style_prefabs,
        ),
    }
}

fn lower_sequence(
    elements: &[Element],
    current_style: &TextStyle,
    initial_paragraph_style: ParagraphStyleOverride,
    style_prefabs: Option<&HashMap<String, TextStyle>>,
) -> Vec<ParsedParagraph> {
    let mut paragraphs = vec![ParsedParagraph {
        nodes: Vec::new(),
        paragraph_style: initial_paragraph_style,
    }];

    for element in elements {
        match element {
            Element::Text {
                start,
                end,
                content,
                segment_id,
            } => paragraphs
                .last_mut()
                .expect("段落列表始终非空")
                .nodes
                .push(InlineNode::Text(TextRun {
                    text: content.clone(),
                    style: current_style.clone(),
                    source_range: SourceRange {
                        segment_id: segment_id.clone(),
                        start: *start,
                        end: *end,
                    },
                })),
            Element::Block {
                inner,
                tag,
                attributes,
                ..
            } => {
                let inherited_paragraph_style = paragraphs
                    .last()
                    .expect("段落列表始终非空")
                    .paragraph_style
                    .clone();
                let mut child_paragraphs = if is_text_style_tag(tag) {
                    let mut style = current_style.clone();
                    apply_text_attributes(&mut style, tag, attributes);
                    lower_sequence(inner, &style, inherited_paragraph_style, style_prefabs)
                } else if tag == "code" {
                    let style = code_text_style(attributes, current_style);
                    let scope = InlineScopeKind::InlineCode(InlineCodeStyle {
                        text_style: style.clone(),
                        background: background_style(attributes),
                    });
                    lower_sequence(inner, &style, inherited_paragraph_style, style_prefabs)
                        .into_iter()
                        .map(|mut paragraph| {
                            paragraph.nodes = vec![InlineNode::Scope {
                                kind: scope.clone(),
                                children: paragraph.nodes,
                            }];
                            paragraph
                        })
                        .collect()
                } else if let Some(scope) = scope_kind(tag, attributes, current_style) {
                    lower_sequence(
                        inner,
                        current_style,
                        inherited_paragraph_style,
                        style_prefabs,
                    )
                    .into_iter()
                    .map(|mut paragraph| {
                        paragraph.nodes = vec![InlineNode::Scope {
                            kind: scope.clone(),
                            children: paragraph.nodes,
                        }];
                        paragraph
                    })
                    .collect()
                } else if let Some(prefab) = style_prefabs.and_then(|prefabs| prefabs.get(tag)) {
                    lower_sequence(inner, prefab, inherited_paragraph_style, style_prefabs)
                } else {
                    if !tag.is_empty() {
                        log::warn!("unrecognized rich-text tag `{tag}`, preserving its content");
                    }
                    lower_sequence(
                        inner,
                        current_style,
                        inherited_paragraph_style,
                        style_prefabs,
                    )
                };
                append_paragraphs(&mut paragraphs, &mut child_paragraphs);
            }
            Element::SelfClosing {
                tag,
                attributes,
                start,
                end,
                segment_id,
            } if tag == "br" => {
                let mut next_style = paragraphs
                    .last()
                    .expect("段落列表始终非空")
                    .paragraph_style
                    .clone();
                apply_paragraph_attributes(&mut next_style, attributes);
                paragraphs.push(ParsedParagraph {
                    nodes: Vec::new(),
                    paragraph_style: next_style,
                });
            }
            Element::SelfClosing {
                tag,
                attributes,
                start,
                end,
                segment_id,
            } if tag == "object" => {
                if let Some(object) = inline_object(attributes, *start, *end, segment_id.clone()) {
                    paragraphs
                        .last_mut()
                        .expect("段落列表始终非空")
                        .nodes
                        .push(InlineNode::Object(object));
                }
            }
            Element::SelfClosing { tag, .. } => {
                log::warn!("unrecognized self-closing rich-text tag `{tag}`, ignored");
            }
        }
    }

    paragraphs
}

fn append_paragraphs(target: &mut Vec<ParsedParagraph>, source: &mut Vec<ParsedParagraph>) {
    let Some(first) = source.first_mut() else {
        return;
    };
    target
        .last_mut()
        .expect("段落列表始终非空")
        .nodes
        .append(&mut first.nodes);
    target.extend(source.drain(1..));
}

fn is_text_style_tag(tag: &str) -> bool {
    matches!(
        tag,
        "span"
            | "size"
            | "color"
            | "fillColor"
            | "stroke"
            | "strokeColor"
            | "strokeWidth"
            | "shadow"
            | "shadowColor"
            | "shadowOffsetX"
            | "shadowOffsetY"
            | "shadowBlur"
            | "shadowWidth"
            | "font"
            | "weight"
            | "bold"
            | "italic"
            | "fontSynthesis"
            | "locale"
            | "baseline"
            | "attach"
    )
}

fn apply_text_attributes(style: &mut TextStyle, tag: &str, attributes: &[Attribute]) {
    if tag == "bold" && attributes.is_empty() {
        style.font_weight = 700;
    }
    if tag == "italic" && attributes.is_empty() {
        style.italic = true;
    }

    for attribute in attributes {
        match attribute.name.as_str() {
            "size" => update_value(&attribute.value, &mut style.font_size, "font size"),
            "color" | "fillColor" if matches!(tag, "span" | "color" | "fillColor") => {
                update_value(&attribute.value, &mut style.fill_color, "fill color")
            }
            "stroke" => {
                style.stroke = parse_optional(&attribute.value, style.stroke.as_ref(), "stroke")
            }
            "strokeColor" => {
                let stroke = style.stroke.get_or_insert_with(StrokeStyle::default);
                update_value(&attribute.value, &mut stroke.stroke_color, "stroke color");
            }
            "color" if tag == "stroke" => {
                let stroke = style.stroke.get_or_insert_with(StrokeStyle::default);
                update_value(&attribute.value, &mut stroke.stroke_color, "stroke color");
            }
            "strokeWidth" | "width" if matches!(tag, "span" | "stroke" | "strokeWidth") => {
                let stroke = style.stroke.get_or_insert_with(StrokeStyle::default);
                update_value(&attribute.value, &mut stroke.stroke_width, "stroke width");
            }
            "shadow" => {
                style.shadow = parse_optional(&attribute.value, style.shadow.as_ref(), "shadow")
            }
            "shadowColor" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(&attribute.value, &mut shadow.shadow_color, "shadow color");
            }
            "color" if tag == "shadow" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(&attribute.value, &mut shadow.shadow_color, "shadow color");
            }
            "shadowOffsetX" | "offsetX" if matches!(tag, "span" | "shadow" | "shadowOffsetX") => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_offset_x,
                    "shadow offset x",
                );
            }
            "shadowOffsetY" | "offsetY" if matches!(tag, "span" | "shadow" | "shadowOffsetY") => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_offset_y,
                    "shadow offset y",
                );
            }
            "shadowBlur" | "blur" if matches!(tag, "span" | "shadow" | "shadowBlur") => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(&attribute.value, &mut shadow.shadow_blur, "shadow blur");
            }
            "shadowWidth" | "width" if matches!(tag, "span" | "shadow" | "shadowWidth") => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(&attribute.value, &mut shadow.shadow_width, "shadow width");
            }
            "font" | "family" => {
                let families = attribute
                    .value
                    .split(',')
                    .map(str::trim)
                    .filter(|family| !family.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                if families.is_empty() {
                    log::warn!("invalid empty font family list");
                } else {
                    style.font_families = families;
                }
            }
            "weight" => update_value(&attribute.value, &mut style.font_weight, "font weight"),
            "fontSynthesis" => match attribute.value.as_str() {
                "none" => style.font_synthesis = FontSynthesis::None,
                "weight" => style.font_synthesis = FontSynthesis::Weight,
                "style" => style.font_synthesis = FontSynthesis::Style,
                "all" => style.font_synthesis = FontSynthesis::All,
                _ => log::warn!("invalid font synthesis `{}`", attribute.value),
            },
            "italic" | "enabled" if tag == "italic" => {
                update_value(&attribute.value, &mut style.italic, "italic flag")
            }
            "locale" => style.locale = attribute.value.clone(),
            "baseline" => update_value(
                &attribute.value,
                &mut style.baseline_shift,
                "baseline shift",
            ),
            "attach" => match attribute.value.as_str() {
                "none" => style.inline_attachment = InlineAttachment::None,
                "previous" => style.inline_attachment = InlineAttachment::Previous,
                _ => log::warn!("invalid inline attachment `{}`", attribute.value),
            },
            _ => log::warn!(
                "unrecognized text attribute `{}` on tag `{tag}`",
                attribute.name
            ),
        }
    }
}

fn scope_kind(
    tag: &str,
    attributes: &[Attribute],
    current_style: &TextStyle,
) -> Option<InlineScopeKind> {
    match tag {
        "background" => Some(InlineScopeKind::Background(background_style(attributes))),
        "underline" => Some(InlineScopeKind::Underline(line_style(attributes))),
        "lineThrough" => Some(InlineScopeKind::LineThrough(line_style(attributes))),
        "ruby" | "bopomofo" => ruby_style(tag, attributes).map(InlineScopeKind::Ruby),
        "emphasis" => Some(InlineScopeKind::Decoration(DecorationKind::Emphasis)),
        "mourning" => Some(InlineScopeKind::Decoration(DecorationKind::Mourning)),
        "properNoun" => Some(InlineScopeKind::Decoration(DecorationKind::ProperNoun)),
        "bookTitle" => Some(InlineScopeKind::Decoration(DecorationKind::BookTitle)),
        "link" => attribute_value(attributes, "target")
            .cloned()
            .map(|target| InlineScopeKind::Link { target }),
        "technical" => Some(InlineScopeKind::Technical),
        "code" => Some(InlineScopeKind::InlineCode(InlineCodeStyle {
            text_style: code_text_style(attributes, current_style),
            background: background_style(attributes),
        })),
        "noAutoSpace" => Some(InlineScopeKind::AutoSpaceSuppressed),
        "box" => inline_box_style(attributes).map(InlineScopeKind::InlineBox),
        _ => None,
    }
}

fn code_text_style(attributes: &[Attribute], current_style: &TextStyle) -> TextStyle {
    let mut style = current_style.clone();
    apply_text_attributes(&mut style, "span", attributes);
    style
}

fn background_style(attributes: &[Attribute]) -> BackgroundStyle {
    let mut style = BackgroundStyle::default();
    for attribute in attributes {
        match attribute.name.as_str() {
            "color" => update_value(&attribute.value, &mut style.fill_color, "background color"),
            "strokeColor" => {
                let stroke = style.stroke.get_or_insert_with(StrokeStyle::default);
                update_value(
                    &attribute.value,
                    &mut stroke.stroke_color,
                    "background stroke color",
                );
            }
            "strokeWidth" => {
                let stroke = style.stroke.get_or_insert_with(StrokeStyle::default);
                update_value(
                    &attribute.value,
                    &mut stroke.stroke_width,
                    "background stroke width",
                );
            }
            "shadowColor" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_color,
                    "background shadow color",
                );
            }
            "shadowOffsetX" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_offset_x,
                    "background shadow offset x",
                );
            }
            "shadowOffsetY" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_offset_y,
                    "background shadow offset y",
                );
            }
            "shadowBlur" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_blur,
                    "background shadow blur",
                );
            }
            "shadowWidth" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_width,
                    "background shadow width",
                );
            }
            "paddingX" => update_value(
                &attribute.value,
                &mut style.padding_x,
                "background padding x",
            ),
            "paddingY" => update_value(
                &attribute.value,
                &mut style.padding_y,
                "background padding y",
            ),
            "radius" => update_value(&attribute.value, &mut style.radius, "background radius"),
            "continuationRadius" => update_optional_value(
                &attribute.value,
                &mut style.continuation_radius,
                "background continuation radius",
            ),
            "clearance" => update_value(
                &attribute.value,
                &mut style.clearance,
                "background clearance",
            ),
            "metricPolicy" => {
                style.metric_policy = match attribute.value.as_str() {
                    "markedFaces" => BackgroundMetricPolicy::MarkedFaces,
                    "uniformTextStyle" => BackgroundMetricPolicy::UniformTextStyle,
                    "uniformParagraphStyle" => BackgroundMetricPolicy::UniformParagraphStyle,
                    _ => {
                        log::warn!("invalid background metric policy `{}`", attribute.value);
                        style.metric_policy
                    }
                }
            }
            _ => log::warn!("unrecognized background attribute `{}`", attribute.name),
        }
    }
    style
}

fn line_style(attributes: &[Attribute]) -> LineStyle {
    let mut style = LineStyle::default();
    let mut dash_length = None;
    let mut gap_length = None;
    for attribute in attributes {
        match attribute.name.as_str() {
            "color" => update_value(&attribute.value, &mut style.fill_color, "line color"),
            "strokeColor" => {
                let stroke = style.stroke.get_or_insert_with(StrokeStyle::default);
                update_value(
                    &attribute.value,
                    &mut stroke.stroke_color,
                    "line stroke color",
                );
            }
            "strokeWidth" => {
                let stroke = style.stroke.get_or_insert_with(StrokeStyle::default);
                update_value(
                    &attribute.value,
                    &mut stroke.stroke_width,
                    "line stroke width",
                );
            }
            "shadowColor" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_color,
                    "line shadow color",
                );
            }
            "shadowOffsetX" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_offset_x,
                    "line shadow offset x",
                );
            }
            "shadowOffsetY" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_offset_y,
                    "line shadow offset y",
                );
            }
            "shadowBlur" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_blur,
                    "line shadow blur",
                );
            }
            "shadowWidth" => {
                let shadow = style.shadow.get_or_insert_with(ShadowStyle::default);
                update_value(
                    &attribute.value,
                    &mut shadow.shadow_width,
                    "line shadow width",
                );
            }
            "thickness" => update_value(&attribute.value, &mut style.thickness, "line thickness"),
            "dashLength" => {
                update_optional_value(&attribute.value, &mut dash_length, "dash length")
            }
            "gapLength" => update_optional_value(&attribute.value, &mut gap_length, "gap length"),
            "clearance" => update_value(&attribute.value, &mut style.clearance, "line clearance"),
            "pattern" => {
                style.pattern = match attribute.value.as_str() {
                    "solid" => LinePattern::Solid,
                    "dashed" => LinePattern::Dashed {
                        dash_length: dash_length.unwrap_or(1.0),
                        gap_length: gap_length.unwrap_or(1.0),
                    },
                    "dotted" => LinePattern::Dotted {
                        gap_length: gap_length.unwrap_or(1.0),
                    },
                    _ => {
                        log::warn!("invalid line pattern `{}`", attribute.value);
                        style.pattern
                    }
                }
            }
            _ => log::warn!("unrecognized line attribute `{}`", attribute.name),
        }
    }
    match &mut style.pattern {
        LinePattern::Dashed {
            dash_length: dash,
            gap_length: gap,
        } => {
            *dash = dash_length.unwrap_or(*dash);
            *gap = gap_length.unwrap_or(*gap);
        }
        LinePattern::Dotted { gap_length: gap } => *gap = gap_length.unwrap_or(*gap),
        LinePattern::Solid => {}
    }
    style
}

fn ruby_style(tag: &str, attributes: &[Attribute]) -> Option<RubyStyle> {
    let text = attribute_value(attributes, "text")?.clone();
    if text.is_empty() {
        log::warn!("empty ruby annotation ignored");
        return None;
    }
    let font_families = attribute_value(attributes, "font")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Some(RubyStyle {
        text,
        font_families,
        locale: attribute_value(attributes, "locale").cloned(),
        kind: if tag == "bopomofo" {
            RubyKind::Bopomofo
        } else {
            RubyKind::Pinyin
        },
    })
}

fn inline_box_style(attributes: &[Attribute]) -> Option<InlineBoxStyle> {
    let mut style = InlineBoxStyle {
        start: 0.0,
        end: 0.0,
        spacing: InlineBoxSpacing::Narrow,
    };
    for attribute in attributes {
        match attribute.name.as_str() {
            "start" => update_value(&attribute.value, &mut style.start, "box start"),
            "end" => update_value(&attribute.value, &mut style.end, "box end"),
            "spacing" => {
                style.spacing = match attribute.value.as_str() {
                    "narrow" => InlineBoxSpacing::Narrow,
                    "source" => InlineBoxSpacing::Source,
                    _ => {
                        log::warn!("invalid box spacing `{}`", attribute.value);
                        style.spacing
                    }
                }
            }
            _ => log::warn!("unrecognized box attribute `{}`", attribute.name),
        }
    }
    Some(style)
}

fn inline_object(
    attributes: &[Attribute],
    start: crate::parser::ScalarOffset,
    end: crate::parser::ScalarOffset,
    segment_id: Option<crate::parser::SegmentId>,
) -> Option<InlineObject> {
    let alt = attribute_value(attributes, "alt")?.clone();
    if alt.is_empty() {
        log::warn!("empty inline object replacement text ignored");
        return None;
    }
    let width = parse_required(attribute_value(attributes, "width"), "object width")?;
    let ascent = parse_required(attribute_value(attributes, "ascent"), "object ascent")?;
    let descent = parse_required(attribute_value(attributes, "descent"), "object descent")?;
    Some(InlineObject {
        alt,
        width,
        ascent,
        descent,
        leading_boundary: InlineObjectBoundary::Fixed,
        trailing_boundary: InlineObjectBoundary::Fixed,
        source_range: SourceRange {
            segment_id,
            start,
            end,
        },
    })
}

fn apply_paragraph_attributes(style: &mut ParagraphStyleOverride, attributes: &[Attribute]) {
    for attribute in attributes {
        match attribute.name.as_str() {
            "indent" => {
                update_optional_value(&attribute.value, &mut style.indent, "paragraph indent")
            }
            "lineHeight" => update_optional_value(
                &attribute.value,
                &mut style.line_height,
                "paragraph line height",
            ),
            "blockIndent" => update_optional_value(
                &attribute.value,
                &mut style.block_indent,
                "paragraph block indent",
            ),
            "align" => {
                style.last_line_alignment = match attribute.value.as_str() {
                    "start" => Some(ParagraphAlignment::Start),
                    "center" => Some(ParagraphAlignment::Center),
                    "end" => Some(ParagraphAlignment::End),
                    _ => {
                        log::warn!("invalid paragraph align `{}`", attribute.value);
                        style.last_line_alignment
                    }
                }
            }
            "lineLengthGrid" => update_optional_value(
                &attribute.value,
                &mut style.line_length_grid,
                "line length grid",
            ),
            "rubyLineHeightMode" => {
                style.ruby_line_height_mode = match attribute.value.as_str() {
                    "perLine" => Some(RubyLineHeightMode::PerLine),
                    "uniformParagraph" => Some(RubyLineHeightMode::UniformParagraph),
                    _ => {
                        log::warn!("invalid ruby line height mode `{}`", attribute.value);
                        style.ruby_line_height_mode
                    }
                }
            }
            "inlineObjectMinimumClearance" => update_optional_value(
                &attribute.value,
                &mut style.inline_object_minimum_clearance,
                "inline object clearance",
            ),
            "emphasisDotGap" => update_optional_value(
                &attribute.value,
                &mut style.emphasis_dot_gap,
                "emphasis dot gap",
            ),
            _ => log::warn!("unrecognized paragraph attribute `{}`", attribute.name),
        }
    }
}

fn attribute_value<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a String> {
    attributes
        .iter()
        .rev()
        .find(|attribute| attribute.name == name)
        .map(|attribute| &attribute.value)
}

fn parse_required<T: FromStr>(value: Option<&String>, name: &str) -> Option<T> {
    let value = value?;
    value
        .parse()
        .map_err(|_| log::warn!("invalid {name} `{value}`"))
        .ok()
}

fn update_value<T: FromStr>(value: &str, target: &mut T, name: &str) {
    match value.parse() {
        Ok(value) => *target = value,
        Err(_) => log::warn!("invalid {name} `{value}`"),
    }
}

fn update_optional_value<T: FromStr>(value: &str, target: &mut Option<T>, name: &str) {
    match value.parse() {
        Ok(value) => *target = Some(value),
        Err(_) => log::warn!("invalid {name} `{value}`"),
    }
}

fn parse_optional<T: FromStr + Clone>(value: &str, fallback: Option<&T>, name: &str) -> Option<T> {
    value.parse().map(Some).unwrap_or_else(|_| {
        log::warn!("invalid {name} `{value}`");
        fallback.cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ParagraphAlignment;
    use crate::parser::{Element, Segment, parse};

    #[test]
    fn span_applies_text_and_paint_attributes_through_one_path() {
        let document = lower_elements(
            parse(&Segment::dummy(
                "[span size=24 color=#112233 strokeColor=#445566 strokeWidth=2 shadowColor=#778899 shadowOffsetX=1 shadowOffsetY=2 shadowBlur=3 shadowWidth=4]文[/span]",
            ))
            .unwrap(),
            &TextStyle::default(),
            None,
        );
        let InlineNode::Text(run) = &document.paragraphs[0].nodes[0] else {
            panic!("expected text node");
        };
        assert_eq!(run.style.font_size, 24.0);
        assert_eq!(run.style.fill_color.to_rgba8(), [0x11, 0x22, 0x33, 0xFF]);
        let stroke = run.style.stroke.as_ref().expect("expected stroke");
        assert_eq!(stroke.stroke_color.to_rgba8(), [0x44, 0x55, 0x66, 0xFF]);
        assert_eq!(stroke.stroke_width, 2.0);
        let shadow = run.style.shadow.as_ref().expect("expected shadow");
        assert_eq!(shadow.shadow_color.to_rgba8(), [0x77, 0x88, 0x99, 0xFF]);
        assert_eq!(shadow.shadow_offset_x, 1.0);
        assert_eq!(shadow.shadow_offset_y, 2.0);
        assert_eq!(shadow.shadow_blur, 3.0);
        assert_eq!(shadow.shadow_width, 4.0);
    }

    #[test]
    fn font_tag_preserves_declared_family_order() {
        let document = lower_elements(
            parse(&Segment::dummy("[font=\"latin, cjk\"]A[/font]")).unwrap(),
            &TextStyle::default(),
            None,
        );
        let InlineNode::Text(run) = &document.paragraphs[0].nodes[0] else {
            panic!("expected text node");
        };

        assert_eq!(
            run.style.font_families,
            vec!["latin".to_owned(), "cjk".to_owned()]
        );
    }

    #[test]
    fn font_synthesis_tag_and_span_attribute_override_inherited_style() {
        let document = lower_elements(
            parse(&Segment::dummy(
                "[fontSynthesis=style]外层[span fontSynthesis=weight]内层[/span][/fontSynthesis]",
            ))
            .unwrap(),
            &TextStyle::default(),
            None,
        );
        let InlineNode::Text(outer) = &document.paragraphs[0].nodes[0] else {
            panic!("expected outer text node");
        };
        let InlineNode::Text(inner) = &document.paragraphs[0].nodes[1] else {
            panic!("expected inner text node");
        };

        assert_eq!(outer.style.font_synthesis, FontSynthesis::Style);
        assert_eq!(inner.style.font_synthesis, FontSynthesis::Weight);
    }

    #[test]
    fn br_splits_paragraphs_and_continues_enclosing_scope() {
        let document = lower_elements(
            parse(&Segment::dummy(
                "[background color=#FFFF00]甲[br indent=2 align=center/]乙[/background]",
            ))
            .unwrap(),
            &TextStyle::default(),
            None,
        );
        assert_eq!(document.paragraphs.len(), 2);
        assert_eq!(document.paragraphs[1].paragraph_style.indent, Some(2.0));
        assert_eq!(
            document.paragraphs[1].paragraph_style.last_line_alignment,
            Some(ParagraphAlignment::Center)
        );
        for paragraph in &document.paragraphs {
            let InlineNode::Scope {
                kind: InlineScopeKind::Background(_),
                children,
            } = &paragraph.nodes[0]
            else {
                panic!("expected background scope");
            };
            assert!(matches!(&children[0], InlineNode::Text(_)));
        }
    }

    #[test]
    fn br_rejects_legacy_last_line_alignment_attribute() {
        let document = lower_elements(
            parse(&Segment::dummy(
                "甲[br align=end/]乙[br lastLineAlignment=center/]丙",
            ))
            .unwrap(),
            &TextStyle::default(),
            None,
        );

        assert_eq!(
            document.paragraphs[1].paragraph_style.last_line_alignment,
            Some(ParagraphAlignment::End)
        );
        assert_eq!(
            document.paragraphs[2].paragraph_style.last_line_alignment,
            Some(ParagraphAlignment::End)
        );
    }

    #[test]
    fn object_requires_nonempty_alt_and_metrics() {
        let document = lower_elements(
            vec![Element::SelfClosing {
                start: crate::parser::ScalarOffset(0),
                end: crate::parser::ScalarOffset(1),
                tag: "object".to_string(),
                attributes: Vec::new(),
                segment_id: None,
            }],
            &TextStyle::default(),
            None,
        );
        assert!(document.paragraphs[0].nodes.is_empty());
    }
}
