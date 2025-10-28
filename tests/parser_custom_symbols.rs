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

use huozi::parser::{parse_with, Block, Element};

#[test]
fn unicode_brackets_basic() {
    // Test basic tag parsing with 【】
    assert_eq!(
        parse_with::<'【', '】'>("文本 【粗体】内容【/粗体】").unwrap(),
        vec![
            Element::Text("文本 ".to_string()),
            Element::Block(Block {
                inner: vec![Element::Text("内容".to_string())],
                tag: "粗体".to_string(),
                value: None
            })
        ]
    );
}

#[test]
fn unicode_brackets_with_escape() {
    // Test 【【 and 】】 escape sequences
    assert_eq!(
        parse_with::<'【', '】'>("显示 【【字面】】 和 【标签】内容【/标签】").unwrap(),
        vec![
            Element::Text("显示 【字面】 和 ".to_string()),
            Element::Block(Block {
                inner: vec![Element::Text("内容".to_string())],
                tag: "标签".to_string(),
                value: None
            })
        ]
    );
}

#[test]
fn unicode_brackets_nested() {
    // Test nested tags
    assert_eq!(
        parse_with::<'【', '】'>("文本 【外层】a【内层】b【/内层】c【/外层】").unwrap(),
        vec![
            Element::Text("文本 ".to_string()),
            Element::Block(Block {
                inner: vec![
                    Element::Text("a".to_string()),
                    Element::Block(Block {
                        inner: vec![Element::Text("b".to_string())],
                        tag: "内层".to_string(),
                        value: None
                    }),
                    Element::Text("c".to_string())
                ],
                tag: "外层".to_string(),
                value: None
            })
        ]
    );
}

#[test]
fn unicode_brackets_with_value() {
    // Test tags with values
    assert_eq!(
        parse_with::<'【', '】'>("文本 【颜色=红色】内容【/颜色】").unwrap(),
        vec![
            Element::Text("文本 ".to_string()),
            Element::Block(Block {
                inner: vec![Element::Text("内容".to_string())],
                tag: "颜色".to_string(),
                value: Some("红色".to_string())
            })
        ]
    );
}

#[test]
fn unicode_brackets_with_quoted_value() {
    // Test tags with quoted values (double quotes)
    assert_eq!(
        parse_with::<'【', '】'>(r#"文本 【颜色="红 色"】内容【/颜色】"#).unwrap(),
        vec![
            Element::Text("文本 ".to_string()),
            Element::Block(Block {
                inner: vec![Element::Text("内容".to_string())],
                tag: "颜色".to_string(),
                value: Some("红 色".to_string())
            })
        ]
    );
}

#[test]
fn unicode_brackets_with_single_quoted_value() {
    // Test tags with quoted values (single quotes)
    assert_eq!(
        parse_with::<'【', '】'>("文本 【颜色='红 色'】内容【/颜色】").unwrap(),
        vec![
            Element::Text("文本 ".to_string()),
            Element::Block(Block {
                inner: vec![Element::Text("内容".to_string())],
                tag: "颜色".to_string(),
                value: Some("红 色".to_string())
            })
        ]
    );
}

#[test]
fn unicode_brackets_mixed_content() {
    // Test mixed ASCII and Unicode content
    assert_eq!(
        parse_with::<'【', '】'>("Hello 【bold】世界【/bold】 World").unwrap(),
        vec![
            Element::Text("Hello ".to_string()),
            Element::Block(Block {
                inner: vec![Element::Text("世界".to_string())],
                tag: "bold".to_string(),
                value: None
            }),
            Element::Text(" World".to_string())
        ]
    );
}

#[test]
fn unicode_brackets_empty_tag() {
    // Test empty tag
    assert_eq!(
        parse_with::<'【', '】'>("文本 【标签】【/标签】").unwrap(),
        vec![
            Element::Text("文本 ".to_string()),
            Element::Block(Block {
                inner: vec![],
                tag: "标签".to_string(),
                value: None
            })
        ]
    );
}

#[test]
fn unicode_brackets_multiline() {
    // Test multiline content
    assert_eq!(
        parse_with::<'【', '】'>("【标签】第一行\n第二行\n【/标签】").unwrap(),
        vec![Element::Block(Block {
            inner: vec![Element::Text("第一行\n第二行\n".to_string())],
            tag: "标签".to_string(),
            value: None
        })]
    );
}

#[test]
fn unicode_brackets_quad_escape() {
    // Test quadruple escaping: 【【【【 -> 【【
    assert_eq!(
        parse_with::<'【', '】'>("显示 【【【【双层】】】】").unwrap(),
        vec![Element::Text("显示 【【双层】】".to_string())]
    );
}
