mod elements_to_spans;
mod parse_elements;
mod segment;
mod source_range;
mod text_run;
mod text_span;
mod text_style;

pub(crate) use elements_to_spans::*;
pub use parse_elements::*;
pub use segment::*;
pub use source_range::*;
pub use text_run::*;
pub use text_span::*;
pub use text_style::*;

#[cfg(test)]
mod tests {
    use crate::parser::{parse_elements::*, Segment};

    #[test]
    fn plain_text() {
        assert_eq!(
            parse(&Segment::dummy(" some text ")).unwrap(),
            vec![Element::Text {
                start: 0,
                end: 11,
                content: " some text ".to_string()
            }]
        );
    }

    #[test]
    fn plain_text_multiple_line() {
        assert_eq!(
            parse(&Segment::dummy(" some \n  text ")).unwrap(),
            vec![Element::Text {
                start: 0,
                end: 14,
                content: " some \n  text ".to_string()
            }]
        );
    }

    #[test]
    fn plain_text_with_backslash() {
        assert_eq!(
            parse(&Segment::dummy(r" some \n [[text")).unwrap(),
            vec![Element::Text {
                start: 0,
                end: 15,
                content: r" some \n [text".to_string()
            }]
        );
    }

    #[test]
    fn double_bracket_escape() {
        assert_eq!(
            parse(&Segment::dummy("[[bracket]]")).unwrap(),
            vec![Element::Text {
                start: 0,
                end: 11,
                content: "[bracket]".to_string()
            }]
        );
    }

    #[test]
    fn double_bracket_in_text() {
        assert_eq!(
            parse(&Segment::dummy("text [[left]] more [[right]]")).unwrap(),
            vec![Element::Text {
                start: 0,
                end: 28,
                content: "text [left] more [right]".to_string()
            }]
        );
    }

    #[test]
    fn quad_bracket_escape() {
        assert_eq!(
            parse(&Segment::dummy("[[[[double]]]]")).unwrap(),
            vec![Element::Text {
                start: 0,
                end: 14,
                content: "[[double]]".to_string()
            }]
        );
    }

    #[test]
    fn mixed_escape_and_tag() {
        assert_eq!(
            parse(&Segment::dummy("[[tag]] [real]content[/real]")).unwrap(),
            vec![
                Element::Text {
                    start: 0,
                    end: 8,
                    content: "[tag] ".to_string()
                },
                Element::Block {
                    start: 8,
                    end: 28,
                    inner: vec![Element::Text {
                        start: 14,
                        end: 21,
                        content: "content".to_string()
                    }],
                    tag: "real".to_string(),
                    value: None
                }
            ]
        );
    }

    #[test]
    fn mixed_escape_and_tag2() {
        assert_eq!(
            parse(&Segment::dummy(" [real]content[/real]")).unwrap(),
            vec![
                Element::Text {
                    start: 0,
                    end: 1,
                    content: " ".to_string()
                },
                Element::Block {
                    start: 1,
                    end: 21,
                    inner: vec![Element::Text {
                        start: 7,
                        end: 14,
                        content: "content".to_string()
                    }],
                    tag: "real".to_string(),
                    value: None
                }
            ]
        );
    }

    #[test]
    fn complex_escape_tags() {
        assert_eq!(
            parse(&Segment::dummy(
                "Show [[bold]]text[[/bold]] as literal, but [bold]this[/bold] is real"
            ))
            .unwrap(),
            vec![
                Element::Text {
                    start: 0,
                    end: 43,
                    content: "Show [bold]text[/bold] as literal, but ".to_string()
                },
                Element::Block {
                    start: 43,
                    end: 60,
                    inner: vec![Element::Text {
                        start: 49,
                        end: 53,
                        content: "this".to_string()
                    }],
                    tag: "bold".to_string(),
                    value: None
                },
                Element::Text {
                    start: 60,
                    end: 68,
                    content: " is real".to_string()
                }
            ]
        );
    }

    #[test]
    fn single_block_without_value() {
        assert_eq!(
            parse(&Segment::dummy(r"[foo]text[/foo]")).unwrap(),
            vec![Element::Block {
                start: 0,
                end: 15,
                inner: vec![Element::Text {
                    start: 5,
                    end: 9,
                    content: "text".to_string()
                }],
                tag: "foo".to_string(),
                value: None
            }]
        );
    }

    #[test]
    fn no_tag_block() {
        assert_eq!(
            parse(&Segment::dummy(r"[]text[/]")).unwrap(),
            vec![Element::Block {
                start: 0,
                end: 9,
                inner: vec![Element::Text {
                    start: 2,
                    end: 6,
                    content: "text".to_string()
                }],
                tag: "".to_string(),
                value: None
            }]
        );
    }

    #[test]
    fn single_block_with_value() {
        assert_eq!(
            parse(&Segment::dummy(r"[foo=bar]text[/foo]")).unwrap(),
            vec![Element::Block {
                start: 0,
                end: 19,
                inner: vec![Element::Text {
                    start: 9,
                    end: 13,
                    content: "text".to_string()
                }],
                tag: "foo".to_string(),
                value: Some("bar".to_string())
            }]
        );
    }

    #[test]
    fn single_block_with_value_quoted_double() {
        assert_eq!(
            parse(&Segment::dummy(r#"[foo="bar "]text[/foo]"#)).unwrap(),
            vec![Element::Block {
                start: 0,
                end: 22,
                inner: vec![Element::Text {
                    start: 12,
                    end: 16,
                    content: "text".to_string()
                }],
                tag: "foo".to_string(),
                value: Some("bar ".to_string())
            }]
        );
    }

    #[test]
    fn single_block_with_value_quoted_single() {
        assert_eq!(
            parse(&Segment::dummy(r#"[foo='bar ']text[/foo]"#)).unwrap(),
            vec![Element::Block {
                start: 0,
                end: 22,
                inner: vec![Element::Text {
                    start: 12,
                    end: 16,
                    content: "text".to_string()
                }],
                tag: "foo".to_string(),
                value: Some("bar ".to_string())
            }]
        );
    }

    #[test]
    fn single_block_multiline() {
        assert_eq!(
            parse(&Segment::dummy("[foo=bar]\ntext\n  \n[/foo]")).unwrap(),
            vec![Element::Block {
                start: 0,
                end: 24,
                inner: vec![Element::Text {
                    start: 9,
                    end: 18,
                    content: "\ntext\n  \n".to_string()
                }],
                tag: "foo".to_string(),
                value: Some("bar".to_string())
            }]
        );
    }

    #[test]
    fn mixed_text_and_block() {
        assert_eq!(
            parse(&Segment::dummy(r" some text [foo=bar]text[/foo]")).unwrap(),
            vec![
                Element::Text {
                    start: 0,
                    end: 11,
                    content: " some text ".to_string()
                },
                Element::Block {
                    start: 11,
                    end: 30,
                    inner: vec![Element::Text {
                        start: 20,
                        end: 24,
                        content: "text".to_string()
                    }],
                    tag: "foo".to_string(),
                    value: Some("bar".to_string())
                }
            ]
        );
    }

    #[test]
    fn nested_blocks() {
        assert_eq!(
            parse(&Segment::dummy(r"[foo=bar][xx=123][/xx][/foo]")).unwrap(),
            vec![Element::Block {
                start: 0,
                end: 28,
                inner: vec![Element::Block {
                    start: 9,
                    end: 22,
                    inner: vec![],
                    tag: "xx".to_string(),
                    value: Some("123".to_string())
                }],
                tag: "foo".to_string(),
                value: Some("bar".to_string())
            }]
        );
    }

    #[test]
    fn complex_elements() {
        // Backslash and 'n' are now treated as literal characters, not escape sequence
        assert_eq!(
            parse(&Segment::dummy(
                r"a\n[foo=bar]q[xx=123][/xx]x[/foo][yy][/yy]"
            ))
            .unwrap(),
            vec![
                Element::Text {
                    start: 0,
                    end: 3,
                    content: r"a\n".to_string()
                },
                Element::Block {
                    start: 3,
                    end: 33,
                    inner: vec![
                        Element::Text {
                            start: 12,
                            end: 13,
                            content: "q".to_string()
                        },
                        Element::Block {
                            start: 13,
                            end: 26,
                            inner: vec![],
                            tag: "xx".to_string(),
                            value: Some("123".to_string())
                        },
                        Element::Text {
                            start: 26,
                            end: 27,
                            content: "x".to_string()
                        }
                    ],
                    tag: "foo".to_string(),
                    value: Some("bar".to_string())
                },
                Element::Block {
                    start: 33,
                    end: 42,
                    inner: vec![],
                    tag: "yy".to_string(),
                    value: None
                }
            ]
        );
    }

    #[test]
    fn tagpair_with_spaces() {
        assert_eq!(
            parse(&Segment::dummy(r#"[ foo = "bar " ]text[/ foo  ]"#)).unwrap(),
            vec![Element::Block {
                start: 0,
                end: 29,
                inner: vec![Element::Text {
                    start: 16,
                    end: 20,
                    content: "text".to_string()
                }],
                tag: "foo".to_string(),
                value: Some("bar ".to_string())
            }]
        );
    }

    #[test]
    fn empty() {
        assert_eq!(parse(&Segment::dummy("")).unwrap(), vec![]);
    }
}
