use huozi::parser::{Element, Segment, parse_with};

#[test]
fn custom_symbols_preserve_attributes_and_nesting() {
    let elements = parse_with::<'【', '】'>(&Segment::dummy(
        "文本 【span size=24】【ruby text='nèi'】内【/ruby】【/span】",
    ))
    .unwrap();
    let Element::Block {
        tag,
        attributes,
        inner,
        ..
    } = &elements[1]
    else {
        panic!("expected span block");
    };
    assert_eq!(tag, "span");
    assert_eq!(attributes[0].name, "size");
    let Element::Block {
        tag, attributes, ..
    } = &inner[0]
    else {
        panic!("expected ruby block");
    };
    assert_eq!(tag, "ruby");
    assert_eq!(attributes[0].value, "nèi");
}

#[test]
fn custom_symbols_keep_escaped_text() {
    let elements = parse_with::<'【', '】'>(&Segment::dummy("【【标签】】")).unwrap();
    assert!(matches!(&elements[0], Element::Text { content, .. } if content == "【标签】"));
}
