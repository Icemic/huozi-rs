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

use huozi::parser::{parse_with, Element, Segment};

#[test]
fn unicode_brackets_basic() {
    // Test basic tag parsing with 【】
    let input = "文本 【粗体】内容【/粗体】";
    assert_eq!(
        parse_with::<'【', '】'>(&Segment::dummy(input)).unwrap(),
        vec![
            Element::Text {
                start: 0,
                end: 7,
                content: "文本 ".to_string()
            },
            Element::Block {
                start: 7,
                end: 38,
                inner: vec![Element::Text {
                    start: 19,
                    end: 25,
                    content: "内容".to_string()
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
                start: 0,
                end: 30,
                content: "显示 【字面】 和 ".to_string()
            },
            Element::Block {
                start: 30,
                end: 61,
                inner: vec![Element::Text {
                    start: 42,
                    end: 48,
                    content: "内容".to_string()
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
                start: 0,
                end: 7,
                content: "文本 ".to_string()
            },
            Element::Block {
                start: 7,
                end: 60,
                inner: vec![
                    Element::Text {
                        start: 19,
                        end: 20,
                        content: "a".to_string()
                    },
                    Element::Block {
                        start: 20,
                        end: 46,
                        inner: vec![Element::Text {
                            start: 32,
                            end: 33,
                            content: "b".to_string()
                        }],
                        tag: "内层".to_string(),
                        value: None
                    },
                    Element::Text {
                        start: 46,
                        end: 47,
                        content: "c".to_string()
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
                start: 0,
                end: 7,
                content: "文本 ".to_string()
            },
            Element::Block {
                start: 7,
                end: 45,
                inner: vec![Element::Text {
                    start: 26,
                    end: 32,
                    content: "内容".to_string()
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
                start: 0,
                end: 7,
                content: "文本 ".to_string()
            },
            Element::Block {
                start: 7,
                end: 48,
                inner: vec![Element::Text {
                    start: 29,
                    end: 35,
                    content: "内容".to_string()
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
                start: 0,
                end: 7,
                content: "文本 ".to_string()
            },
            Element::Block {
                start: 7,
                end: 48,
                inner: vec![Element::Text {
                    start: 29,
                    end: 35,
                    content: "内容".to_string()
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
                start: 0,
                end: 6,
                content: "Hello ".to_string()
            },
            Element::Block {
                start: 6,
                end: 33,
                inner: vec![Element::Text {
                    start: 16,
                    end: 22,
                    content: "世界".to_string()
                }],
                tag: "bold".to_string(),
                value: None
            },
            Element::Text {
                start: 33,
                end: 39,
                content: " World".to_string()
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
                start: 0,
                end: 7,
                content: "文本 ".to_string()
            },
            Element::Block {
                start: 7,
                end: 32,
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
            start: 0,
            end: 45,
            inner: vec![Element::Text {
                start: 12,
                end: 32,
                content: "第一行\n第二行\n".to_string()
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
            start: 0,
            end: 37,
            content: "显示 【【双层】】".to_string()
        }]
    );
}
