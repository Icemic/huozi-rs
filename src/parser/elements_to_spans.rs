use std::str::FromStr;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::Result;

use crate::parser::*;

pub(crate) fn to_spans(
    elements: Vec<Element>,
    current_style: &TextStyle,
    style_prefabs: Option<&HashMap<String, TextStyle>>,
) -> Result<Vec<TextSpan>, String> {
    let mut spans = vec![];
    let mut current_runs = vec![];

    // (elements iterator, current style, is_span)
    let mut stack: Vec<(Rc<RefCell<std::vec::IntoIter<Element>>>, TextStyle, bool)> = vec![];
    let mut current_style = current_style.clone();
    let mut elements = Rc::new(RefCell::new(elements.into_iter()));

    loop {
        let elements_remaining = elements.borrow_mut().len();
        if elements_remaining == 0 {
            if !stack.is_empty() {
                let (next_elements, next_style, is_span) = stack.pop().unwrap();
                elements = next_elements;
                current_style = next_style;

                if is_span && !current_runs.is_empty() {
                    let runs = current_runs.drain(..).collect();
                    let span = TextSpan {
                        runs,
                        span_id: Some(SpanId::Lite(0)),
                    };
                    spans.push(span);
                }

                continue;
            } else {
                break;
            }
        }

        let element = elements.borrow_mut().next().unwrap();

        match element {
            Element::Text {
                start,
                end,
                content,
            } => {
                current_runs.push(TextRun {
                    text: content,
                    style: current_style.clone(),
                    source_range: SourceRange {
                        segment_id: Some(SegmentId::Lite(0)),
                        start,
                        end,
                    },
                });
            }
            Element::Block {
                start: _,
                end: _,
                inner,
                tag,
                value,
            } => {
                if tag.as_str() != "span" && value.is_some() {
                    stack.push((elements.clone(), current_style.clone(), false));
                    elements = Rc::new(RefCell::new(inner.into_iter()));

                    let value = value.as_ref().unwrap();
                    match tag.as_str() {
                        "size" => {
                            current_style.font_size = parse_str(value, &current_style.font_size);
                        }
                        "color" | "fillColor" => {
                            current_style.fill_color = parse_str(value, &current_style.fill_color);
                        }
                        "lineHeight" => {
                            current_style.line_height =
                                parse_str(value, &current_style.line_height);
                        }
                        "indent" => {
                            current_style.indent = parse_str(value, &current_style.indent);
                        }
                        "stroke" => {
                            current_style.stroke =
                                parse_str_optional(value, current_style.stroke.as_ref());
                        }
                        "strokeColor" => {
                            if current_style.stroke.is_none() {
                                current_style.stroke = Some(StrokeStyle::default());
                            }
                            let stroke = current_style.stroke.as_mut().unwrap();
                            stroke.stroke_color = parse_str(value, &stroke.stroke_color);
                        }
                        "strokeWidth" => {
                            if current_style.stroke.is_none() {
                                current_style.stroke = Some(StrokeStyle::default());
                            }
                            let stroke = current_style.stroke.as_mut().unwrap();
                            stroke.stroke_width = parse_str(value, &stroke.stroke_width);
                        }
                        "shadow" => {
                            current_style.shadow =
                                parse_str_optional(value, current_style.shadow.as_ref());
                        }
                        "shadowOffsetX" => {
                            if current_style.shadow.is_none() {
                                current_style.shadow = Some(ShadowStyle::default());
                            }
                            let shadow = current_style.shadow.as_mut().unwrap();
                            shadow.shadow_offset_x = parse_str(value, &shadow.shadow_offset_x);
                        }
                        "shadowOffsetY" => {
                            if current_style.shadow.is_none() {
                                current_style.shadow = Some(ShadowStyle::default());
                            }
                            let shadow = current_style.shadow.as_mut().unwrap();
                            shadow.shadow_offset_y = parse_str(value, &shadow.shadow_offset_y);
                        }
                        "shadowWidth" => {
                            if current_style.shadow.is_none() {
                                current_style.shadow = Some(ShadowStyle::default());
                            }
                            let shadow = current_style.shadow.as_mut().unwrap();
                            shadow.shadow_width = parse_str(value, &shadow.shadow_width);
                        }
                        "shadowBlur" => {
                            if current_style.shadow.is_none() {
                                current_style.shadow = Some(ShadowStyle::default());
                            }
                            let shadow = current_style.shadow.as_mut().unwrap();
                            shadow.shadow_blur = parse_str(value, &shadow.shadow_blur);
                        }
                        "shadowColor" => {
                            if current_style.shadow.is_none() {
                                current_style.shadow = Some(ShadowStyle::default());
                            }
                            let shadow = current_style.shadow.as_mut().unwrap();
                            shadow.shadow_color = parse_str(value, &shadow.shadow_color);
                        }
                        _ => {
                            log::warn!("unrecognized style tag `{}`, ignored.", tag);
                        }
                    };
                } else {
                    if let Some(style_prefabs) = style_prefabs {
                        if let Some(style_prefab) = style_prefabs.get(&tag) {
                            stack.push((elements.clone(), current_style.clone(), false));
                            elements = Rc::new(RefCell::new(inner.into_iter()));

                            current_style = style_prefab.clone();
                            continue;
                        }
                    }

                    if tag.as_str() != "span" && !tag.is_empty() {
                        log::warn!("unrecognized prefab tag `{}`, treated as normal span", tag);
                    }

                    if !current_runs.is_empty() {
                        let runs = current_runs.drain(..).collect();
                        let span = TextSpan {
                            runs,
                            span_id: Some(SpanId::Lite(0)),
                        };
                        spans.push(span);
                    }

                    stack.push((elements.clone(), current_style.clone(), true));
                    elements = Rc::new(RefCell::new(inner.into_iter()));
                }
            }
        }
    }

    if !current_runs.is_empty() {
        let runs = current_runs.drain(..).collect();
        let span = TextSpan {
            runs,
            span_id: Some(SpanId::Lite(0)),
        };
        spans.push(span);
    }

    Ok(spans)
}

fn parse_str<T: FromStr + Clone>(str: &str, fallback: &T) -> T {
    str.parse::<T>().unwrap_or_else(|_| {
        log::warn!(
            "cannot parse string value `{}` to type `{}`.",
            str,
            std::any::type_name::<T>()
        );
        (*fallback).clone()
    })
}

fn parse_str_optional<T: FromStr + Clone>(str: &str, fallback: Option<&T>) -> Option<T> {
    str.parse::<T>()
        .and_then(|v| Ok(Some(v)))
        .unwrap_or_else(|_| {
            log::warn!(
                "cannot parse string value `{}` to type `{}`.",
                str,
                std::any::type_name::<T>()
            );
            fallback.cloned()
        })
}

#[cfg(test)]
mod tests {
    use super::elements_to_spans::to_spans;
    use crate::parser::*;

    /// Helper function to create a default TextStyle
    fn default_style() -> TextStyle {
        TextStyle::default()
    }

    #[test]
    fn test_plain_text() {
        let input = "[span]Hello, World![/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs.len(), 1);
        assert_eq!(result[0].runs[0].text, "Hello, World!");

        // Check source range byte positions
        let source_range = &result[0].runs[0].source_range;
        assert_eq!(source_range.start, 6); // After "[span]"
        assert_eq!(source_range.end, 19); // Before "[/span]"
    }

    #[test]
    fn test_single_style_tag() {
        let input = "[span]Text with [size=48]large[/size] size[/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs.len(), 3);

        // First run: "Text with "
        assert_eq!(result[0].runs[0].text, "Text with ");
        assert_eq!(result[0].runs[0].style.font_size, 32.0);
        let sr = &result[0].runs[0].source_range;
        assert_eq!(sr.start, 6);
        assert_eq!(sr.end, 16);

        // Second run: "large" with size=48
        assert_eq!(result[0].runs[1].text, "large");
        assert_eq!(result[0].runs[1].style.font_size, 48.0);
        let sr = &result[0].runs[1].source_range;
        assert_eq!(sr.start, 25);
        assert_eq!(sr.end, 30);

        // Third run: " size"
        assert_eq!(result[0].runs[2].text, " size");
        assert_eq!(result[0].runs[2].style.font_size, 32.0);
        let sr = &result[0].runs[2].source_range;
        assert_eq!(sr.start, 37);
        assert_eq!(sr.end, 42);
    }

    #[test]
    fn test_single_span_tag() {
        let input = "[span]Before [span]inside[/span] after[/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        // Without style_prefabs, span tags create separate TextSpans
        assert_eq!(result.len(), 3);

        // First span: "Before "
        assert_eq!(result[0].runs.len(), 1);
        assert_eq!(result[0].runs[0].text, "Before ");

        // Second span: "inside"
        assert_eq!(result[1].runs.len(), 1);
        assert_eq!(result[1].runs[0].text, "inside");

        // Third span: " after"
        assert_eq!(result[2].runs.len(), 1);
        assert_eq!(result[2].runs[0].text, " after");
    }

    #[test]
    fn test_empty_tag_structure() {
        let input = "[span]Text with []empty tag[/] content[/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        // Empty tag is treated as a span, creating separate TextSpans
        assert_eq!(result.len(), 3);

        assert_eq!(result[0].runs[0].text, "Text with ");
        assert_eq!(result[1].runs[0].text, "empty tag");
        assert_eq!(result[2].runs[0].text, " content");
    }

    #[test]
    fn test_nonexistent_tag_fallback_to_span() {
        let input = "[span]Before [unknownTag]content[/unknownTag] after[/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        // Unknown tags without style_prefabs are treated as spans
        assert_eq!(result.len(), 3);

        assert_eq!(result[0].runs[0].text, "Before ");
        assert_eq!(result[1].runs[0].text, "content");
        assert_eq!(result[2].runs[0].text, " after");
    }

    #[test]
    fn test_nested_spans() {
        let input = "[span]Outer [span]Middle [span]Inner[/span] middle[/span] outer[/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        // Multiple nested spans create multiple TextSpans
        assert_eq!(result.len(), 5);

        assert_eq!(result[0].runs[0].text, "Outer ");
        assert_eq!(result[1].runs[0].text, "Middle ");
        assert_eq!(result[2].runs[0].text, "Inner");
        assert_eq!(result[3].runs[0].text, " middle");
        assert_eq!(result[4].runs[0].text, " outer");
    }

    #[test]
    fn test_complex_scenario() {
        // Complex scenario with nested style tags, spans, colors, and stroke
        let input = concat!(
            "[span]",
            "Normal text ",
            "[size=48]large [color=#ff0000]red and large[/color] just large[/size]",
            " and ",
            "[span]nested [color=#00ff00]green[/color] span[/span]",
            " with [strokeColor=#0000ff]blue stroke[/strokeColor] end.",
            "[/span]"
        );
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        // This complex scenario has spans within style tags
        assert!(
            result.len() >= 3,
            "Expected multiple spans, got {}",
            result.len()
        );

        // Check first run
        assert_eq!(result[0].runs[0].text, "Normal text ");
        assert_eq!(result[0].runs[0].style.font_size, 32.0);

        // Find the "large " text with font_size = 48
        let large_run = result
            .iter()
            .flat_map(|span| &span.runs)
            .find(|run| run.text == "large ")
            .expect("Should find 'large ' text");
        assert_eq!(large_run.style.font_size, 48.0);

        // Find the "red and large" text with both size and color
        let red_run = result
            .iter()
            .flat_map(|span| &span.runs)
            .find(|run| run.text == "red and large")
            .expect("Should find 'red and large' text");
        assert_eq!(red_run.style.font_size, 48.0);
        // Color should be red (#ff0000)
        let red_color = red_run.style.fill_color.to_css_hex();
        assert!(red_color.contains("ff0000") || red_color.contains("FF0000"));

        // Find the "green" text
        let green_run = result
            .iter()
            .flat_map(|span| &span.runs)
            .find(|run| run.text == "green")
            .expect("Should find 'green' text");
        let green_color = green_run.style.fill_color.to_css_hex();
        assert!(green_color.contains("00ff00") || green_color.contains("00FF00"));

        // Find the "blue stroke" text
        let stroke_run = result
            .iter()
            .flat_map(|span| &span.runs)
            .find(|run| run.text == "blue stroke")
            .expect("Should find 'blue stroke' text");
        assert!(stroke_run.style.stroke.is_some());
        let stroke_style = stroke_run.style.stroke.as_ref().unwrap();
        let stroke_color = stroke_style.stroke_color.to_css_hex();
        assert!(stroke_color.contains("0000ff") || stroke_color.contains("0000FF"));
    }

    #[test]
    fn test_multiple_style_attributes() {
        let input =
        "[span][color=#ff0000][size=48][lineHeight=2.0]Styled[/lineHeight][/size][/color][/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs.len(), 1);
        assert_eq!(result[0].runs[0].text, "Styled");

        // Check that all styles are applied
        assert_eq!(result[0].runs[0].style.font_size, 48.0);
        assert_eq!(result[0].runs[0].style.line_height, 2.0);
        let color = result[0].runs[0].style.fill_color.to_css_hex();
        assert!(color.contains("ff0000") || color.contains("FF0000"));
    }

    #[test]
    fn test_shadow_style_attributes() {
        // Test individual shadow attributes
        let input = concat!(
            "[span]",
            "[shadowOffsetX=2][shadowOffsetY=3][shadowBlur=5]",
            "Shadow text",
            "[/shadowBlur][/shadowOffsetY][/shadowOffsetX]",
            "[/span]"
        );
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs[0].text, "Shadow text");

        let shadow = result[0].runs[0]
            .style
            .shadow
            .as_ref()
            .expect("Should have shadow");
        assert_eq!(shadow.shadow_offset_x, 2.0);
        assert_eq!(shadow.shadow_offset_y, 3.0);
        assert_eq!(shadow.shadow_blur, 5.0);
    }

    #[test]
    fn test_stroke_attributes() {
        let input =
        "[span][strokeColor=#0000ff][strokeWidth=2.5]Stroked[/strokeWidth][/strokeColor][/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs[0].text, "Stroked");

        let stroke = result[0].runs[0]
            .style
            .stroke
            .as_ref()
            .expect("Should have stroke");
        assert_eq!(stroke.stroke_width, 2.5);
        let color = stroke.stroke_color.to_css_hex();
        assert!(color.contains("0000ff") || color.contains("0000FF"));
    }

    #[test]
    fn test_byte_positions() {
        // Test with multi-byte characters
        let input = "[span]你好[size=48]世界[/size]！[/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        println!("{:#?}", result);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs.len(), 3);

        // "你好" - 2 Chinese characters, each 3 bytes in UTF-8
        assert_eq!(result[0].runs[0].text, "你好");
        let sr = &result[0].runs[0].source_range;
        assert_eq!(sr.start, 6);
        assert_eq!(sr.end, 12);

        // "世界" with size=48
        assert_eq!(result[0].runs[1].text, "世界");
        assert_eq!(result[0].runs[1].style.font_size, 48.0);
        let sr = &result[0].runs[1].source_range;
        assert_eq!(sr.start, 21);
        assert_eq!(sr.end, 27);

        // "！"
        assert_eq!(result[0].runs[2].text, "！");
        let sr = &result[0].runs[2].source_range;
        assert_eq!(sr.start, 34);
        assert_eq!(sr.end, 37);
    }

    #[test]
    fn test_indent_attribute() {
        let input = "[span][indent=2.5]Indented text[/indent][/span]";
        let elements = parse(&Segment::dummy(input)).expect("Failed to parse");

        let result =
            to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs[0].text, "Indented text");
        assert_eq!(result[0].runs[0].style.indent, 2.5);
    }
}
