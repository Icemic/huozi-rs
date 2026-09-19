mod elements_to_document;
mod parse_elements;
mod parsed_text;
mod segment;
mod source_range;
mod text_run;
mod text_span;
mod text_style;

pub(crate) use elements_to_document::*;
pub use parse_elements::*;
pub use parsed_text::*;
pub use segment::*;
pub use source_range::*;
pub use text_run::*;
pub use text_span::*;
pub use text_style::*;

#[cfg(test)]
mod tests {
    use crate::parser::{Element, ScalarOffset, Segment, parse};

    #[test]
    fn parses_attributes_escapes_and_self_closing_tags() {
        let elements = parse(&Segment::dummy(
            r#"[span size=24 color="a\"b" locale='zh-Hans'][br indent=2/]文[/span]"#,
        ))
        .unwrap();
        let Element::Block {
            attributes, inner, ..
        } = &elements[0]
        else {
            panic!("expected span block");
        };
        assert_eq!(
            attributes
                .iter()
                .map(|attribute| attribute.name.as_str())
                .collect::<Vec<_>>(),
            ["size", "color", "locale"]
        );
        assert_eq!(attributes[1].value, "a\"b");
        assert_eq!(attributes[1].value_start, ScalarOffset(21));
        let Element::SelfClosing {
            tag, attributes, ..
        } = &inner[0]
        else {
            panic!("expected self-closing br");
        };
        assert_eq!(tag, "br");
        assert_eq!(attributes[0].name, "indent");
        assert_eq!(attributes[0].value, "2");
    }

    #[test]
    fn preserves_valid_inner_block_after_unclosed_outer_block() {
        let elements = parse(&Segment::dummy("[a]前[good]后[/good]")).unwrap();
        assert!(matches!(&elements[0], Element::Text { content, .. } if content == "[a]前"));
        assert!(matches!(&elements[1], Element::Block { tag, .. } if tag == "good"));
    }

    #[test]
    fn preserves_unclosed_quote_as_text_without_reparsing_its_content() {
        let elements = parse(&Segment::dummy(
            r#"[font family="unterminated [good]后[/good]"#,
        ))
        .unwrap();
        assert_eq!(elements.len(), 1);
        assert!(
            matches!(&elements[0], Element::Text { content, .. } if content == r#"[font family="unterminated [good]后[/good]"#)
        );
    }

    #[test]
    fn resumes_after_bad_tag_head_before_a_later_complete_tag() {
        let elements = parse(&Segment::dummy("[bad attr][good]后[/good]")).unwrap();
        assert!(matches!(&elements[0], Element::Text { content, .. } if content == "[bad attr]"));
        assert!(matches!(&elements[1], Element::Block { tag, .. } if tag == "good"));
    }

    #[test]
    fn preserves_escaped_brackets_as_text() {
        let elements = parse(&Segment::dummy("[[/]]")).unwrap();
        assert!(
            matches!(&elements[0], Element::Text { content, start, end, .. } if content == "[/]" && *start == ScalarOffset(0) && *end == ScalarOffset(5))
        );
    }
}
