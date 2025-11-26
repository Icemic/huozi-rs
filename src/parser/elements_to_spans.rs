use std::str::FromStr;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::Result;

use crate::parser::*;

pub fn to_spans(
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
                    source_range: Some(SourceRange {
                        segment_id: SegmentId::Lite(0),
                        start,
                        end,
                    }),
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
