// Integration tests for custom tag symbols in parser
//
// IMPORTANT: These tests are in a separate file because they use different
// symbol combinations. Due to OnceLock caching optimization in the parser,
// only ONE symbol combination should be used throughout a program's lifetime.
//
// This file uses Chinese corner brackets 【】 to demonstrate:
// 1. Support for Unicode characters as tag symbols
// 2. Support for non-ASCII symbols
// 3. The custom symbol feature works correctly

use huozi::parser::{Element, ScalarOffset, Segment, parse_with};

#[test]
fn unicode_brackets_basic() {
    // Test basic tag parsing with 【】
    let input = "文本 【粗体】内容【/粗体】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: ScalarOffset(0),
                end: ScalarOffset(3),
                content: "文本 ".to_string(),
                segment_id: None,
            },
            Element::Block {
                start: ScalarOffset(3),
                end: ScalarOffset(14),
                inner: vec![Element::Text {
                    start: ScalarOffset(7),
                    end: ScalarOffset(9),
                    content: "内容".to_string(),
                    segment_id: None,
                }],
                tag: "粗体".to_string(),
                value: None
            }
        ]
    );
}

#[test]
fn unicode_brackets_with_escape() {
    // Test 【【 and 】】 escape sequences
    let input = "显示 【【字面】】 和 【标签】内容【/标签】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: ScalarOffset(0),
                end: ScalarOffset(12),
                content: "显示 【字面】 和 ".to_string(),
                segment_id: None,
            },
            Element::Block {
                start: ScalarOffset(12),
                end: ScalarOffset(23),
                inner: vec![Element::Text {
                    start: ScalarOffset(16),
                    end: ScalarOffset(18),
                    content: "内容".to_string(),
                    segment_id: None,
                }],
                tag: "标签".to_string(),
                value: None
            }
        ]
    );
}

#[test]
fn unicode_brackets_nested() {
    // Test nested tags
    let input = "文本 【外层】a【内层】b【/内层】c【/外层】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: ScalarOffset(0),
                end: ScalarOffset(3),
                content: "文本 ".to_string(),
                segment_id: None,
            },
            Element::Block {
                start: ScalarOffset(3),
                end: ScalarOffset(24),
                inner: vec![
                    Element::Text {
                        start: ScalarOffset(7),
                        end: ScalarOffset(8),
                        content: "a".to_string(),
                        segment_id: None,
                    },
                    Element::Block {
                        start: ScalarOffset(8),
                        end: ScalarOffset(18),
                        inner: vec![Element::Text {
                            start: ScalarOffset(12),
                            end: ScalarOffset(13),
                            content: "b".to_string(),
                            segment_id: None,
                        }],
                        tag: "内层".to_string(),
                        value: None
                    },
                    Element::Text {
                        start: ScalarOffset(18),
                        end: ScalarOffset(19),
                        content: "c".to_string(),
                        segment_id: None,
                    }
                ],
                tag: "外层".to_string(),
                value: None
            }
        ]
    );
}

#[test]
fn unicode_brackets_with_value() {
    // Test tags with values
    let input = "文本 【颜色=红色】内容【/颜色】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: ScalarOffset(0),
                end: ScalarOffset(3),
                content: "文本 ".to_string(),
                segment_id: None,
            },
            Element::Block {
                start: ScalarOffset(3),
                end: ScalarOffset(17),
                inner: vec![Element::Text {
                    start: ScalarOffset(10),
                    end: ScalarOffset(12),
                    content: "内容".to_string(),
                    segment_id: None,
                }],
                tag: "颜色".to_string(),
                value: Some("红色".to_string())
            }
        ]
    );
}

#[test]
fn unicode_brackets_with_quoted_value() {
    // Test tags with quoted values (double quotes)
    let input = r#"文本 【颜色="红 色"】内容【/颜色】"#;
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: ScalarOffset(0),
                end: ScalarOffset(3),
                content: "文本 ".to_string(),
                segment_id: None,
            },
            Element::Block {
                start: ScalarOffset(3),
                end: ScalarOffset(20),
                inner: vec![Element::Text {
                    start: ScalarOffset(13),
                    end: ScalarOffset(15),
                    content: "内容".to_string(),
                    segment_id: None,
                }],
                tag: "颜色".to_string(),
                value: Some("红 色".to_string())
            }
        ]
    );
}

#[test]
fn unicode_brackets_with_single_quoted_value() {
    // Test tags with quoted values (single quotes)
    let input = "文本 【颜色='红 色'】内容【/颜色】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: ScalarOffset(0),
                end: ScalarOffset(3),
                content: "文本 ".to_string(),
                segment_id: None,
            },
            Element::Block {
                start: ScalarOffset(3),
                end: ScalarOffset(20),
                inner: vec![Element::Text {
                    start: ScalarOffset(13),
                    end: ScalarOffset(15),
                    content: "内容".to_string(),
                    segment_id: None,
                }],
                tag: "颜色".to_string(),
                value: Some("红 色".to_string())
            }
        ]
    );
}

#[test]
fn unicode_brackets_mixed_content() {
    // Test mixed ASCII and Unicode content
    let input = "Hello 【bold】世界【/bold】 World";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: ScalarOffset(0),
                end: ScalarOffset(6),
                content: "Hello ".to_string(),
                segment_id: None,
            },
            Element::Block {
                start: ScalarOffset(6),
                end: ScalarOffset(21),
                inner: vec![Element::Text {
                    start: ScalarOffset(12),
                    end: ScalarOffset(14),
                    content: "世界".to_string(),
                    segment_id: None,
                }],
                tag: "bold".to_string(),
                value: None
            },
            Element::Text {
                start: ScalarOffset(21),
                end: ScalarOffset(27),
                content: " World".to_string(),
                segment_id: None,
            }
        ]
    );
}

#[test]
fn unicode_brackets_empty_tag() {
    // Test empty tag
    let input = "文本 【标签】【/标签】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: ScalarOffset(0),
                end: ScalarOffset(3),
                content: "文本 ".to_string(),
                segment_id: None,
            },
            Element::Block {
                start: ScalarOffset(3),
                end: ScalarOffset(12),
                inner: vec![],
                tag: "标签".to_string(),
                value: None
            }
        ]
    );
}

#[test]
fn unicode_brackets_multiline() {
    // Test multiline content
    let input = "【标签】第一行\n第二行\n【/标签】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![Element::Block {
            start: ScalarOffset(0),
            end: ScalarOffset(17),
            inner: vec![Element::Text {
                start: ScalarOffset(4),
                end: ScalarOffset(12),
                content: "第一行\n第二行\n".to_string(),
                segment_id: None,
            }],
            tag: "标签".to_string(),
            value: None
        }]
    );
}

#[test]
fn unicode_brackets_quad_escape() {
    // Test quadruple escaping: 【【【【 -> 【【
    let input = "显示 【【【【双层】】】】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![Element::Text {
            start: ScalarOffset(0),
            end: ScalarOffset(13),
            content: "显示 【【双层】】".to_string(),
            segment_id: None,
        }]
    );
}
