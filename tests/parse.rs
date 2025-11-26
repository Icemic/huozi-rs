use huozi::parser::*;

/// Helper function to create a default TextStyle
fn default_style() -> TextStyle {
    TextStyle::default()
}

#[test]
fn test_plain_text() {
    let input = "<span>Hello, World!</span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

    let result =
        to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].runs.len(), 1);
    assert_eq!(result[0].runs[0].text, "Hello, World!");

    // Check source range byte positions
    let source_range = result[0].runs[0].source_range.as_ref().unwrap();
    assert_eq!(source_range.start, 6); // After "<span>"
    assert_eq!(source_range.end, 19); // Before "</span>"
}

#[test]
fn test_single_style_tag() {
    let input = "<span>Text with <size=48>large</size> size</span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

    let result =
        to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].runs.len(), 3);

    // First run: "Text with "
    assert_eq!(result[0].runs[0].text, "Text with ");
    assert_eq!(result[0].runs[0].style.font_size, 32.0);
    let sr = result[0].runs[0].source_range.as_ref().unwrap();
    assert_eq!(sr.start, 6);
    assert_eq!(sr.end, 16);

    // Second run: "large" with size=48
    assert_eq!(result[0].runs[1].text, "large");
    assert_eq!(result[0].runs[1].style.font_size, 48.0);
    let sr = result[0].runs[1].source_range.as_ref().unwrap();
    assert_eq!(sr.start, 25);
    assert_eq!(sr.end, 30);

    // Third run: " size"
    assert_eq!(result[0].runs[2].text, " size");
    assert_eq!(result[0].runs[2].style.font_size, 32.0);
    let sr = result[0].runs[2].source_range.as_ref().unwrap();
    assert_eq!(sr.start, 37);
    assert_eq!(sr.end, 42);
}

#[test]
fn test_single_span_tag() {
    let input = "<span>Before <span>inside</span> after</span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

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
    let input = "<span>Text with <>empty tag</> content</span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

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
    let input = "<span>Before <unknownTag>content</unknownTag> after</span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

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
    let input = "<span>Outer <span>Middle <span>Inner</span> middle</span> outer</span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

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
        "<span>",
        "Normal text ",
        "<size=48>large <color=#ff0000>red and large</color> just large</size>",
        " and ",
        "<span>nested <color=#00ff00>green</color> span</span>",
        " with <strokeColor=#0000ff>blue stroke</strokeColor> end.",
        "</span>"
    );
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

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
        "<span><color=#ff0000><size=48><lineHeight=2.0>Styled</lineHeight></size></color></span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

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
        "<span>",
        "<shadowOffsetX=2><shadowOffsetY=3><shadowBlur=5>",
        "Shadow text",
        "</shadowBlur></shadowOffsetY></shadowOffsetX>",
        "</span>"
    );
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

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
        "<span><strokeColor=#0000ff><strokeWidth=2.5>Stroked</strokeWidth></strokeColor></span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

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
    let input = "<span>你好<size=48>世界</size>！</span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

    let result =
        to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

    println!("{:#?}", result);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].runs.len(), 3);

    // "你好" - 2 Chinese characters, each 3 bytes in UTF-8
    assert_eq!(result[0].runs[0].text, "你好");
    let sr = result[0].runs[0].source_range.as_ref().unwrap();
    assert_eq!(sr.start, 6);
    assert_eq!(sr.end, 12);

    // "世界" with size=48
    assert_eq!(result[0].runs[1].text, "世界");
    assert_eq!(result[0].runs[1].style.font_size, 48.0);
    let sr = result[0].runs[1].source_range.as_ref().unwrap();
    assert_eq!(sr.start, 21);
    assert_eq!(sr.end, 27);

    // "！"
    assert_eq!(result[0].runs[2].text, "！");
    let sr = result[0].runs[2].source_range.as_ref().unwrap();
    assert_eq!(sr.start, 34);
    assert_eq!(sr.end, 37);
}

#[test]
fn test_indent_attribute() {
    let input = "<span><indent=2.5>Indented text</indent></span>";
    let elements = parse_with::<'<', '>'>(input).expect("Failed to parse");

    let result =
        to_spans(elements, &default_style(), None).expect("Failed to parse text recursive");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].runs[0].text, "Indented text");
    assert_eq!(result[0].runs[0].style.indent, 2.5);
}
