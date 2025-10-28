use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{char, multispace0},
    combinator::{cut, eof, map, not, value, verify},
    error::context,
    multi::{fold_many0, many0, many_till},
    sequence::{preceded, separated_pair, terminated},
    IResult, Parser,
};
use nom_language::error::{convert_error, VerboseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Element {
    Text(String),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub inner: Vec<Element>,
    pub tag: String,
    pub value: Option<String>,
}

pub type ParseResult<'a, T, E = VerboseError<&'a str>> = IResult<&'a str, T, E>;

/// Parse plain text with support for [[ and ]] escape sequences
/// [[ -> [
/// ]] -> ]
/// Single [ or ] will stop the parser (not consume them)
fn plain_text_content(input: &str) -> ParseResult<'_, String> {
    use nom::bytes::complete::take_while1;

    context(
        "PlainTextContent",
        fold_many0(
            alt((
                // [[ -> [
                value("[".to_string(), tag("[[")),
                // ]] -> ]
                value("]".to_string(), tag("]]")),
                // Regular text (not starting with [ or ])
                map(take_while1(|c| c != '[' && c != ']'), |s: &str| {
                    s.to_string()
                }),
            )),
            String::new,
            |mut acc, item| {
                acc.push_str(&item);
                acc
            },
        ),
    )
    .parse(input)
}

fn string_quoted(input: &str) -> ParseResult<'_, String> {
    context(
        "String Quoted",
        preceded(
            char('\"'),
            cut(terminated(
                map(is_not("\""), |s: &str| s.to_string()),
                char('\"'),
            )),
        ),
    )
    .parse(input)
}

fn string_without_space(input: &str) -> ParseResult<'_, String> {
    let chars = "\"[]/= \t\n\r";
    context(
        "String without Space",
        map(preceded(multispace0, is_not(chars)), |s: &str| {
            s.to_string()
        }),
    )
    .parse(input)
}

fn plain_text(input: &str) -> ParseResult<'_, Element> {
    context(
        "PlainText",
        verify(
            map(plain_text_content, |s: String| Element::Text(s)),
            |elem| {
                // Only succeed if we actually parsed some text
                if let Element::Text(s) = elem {
                    !s.is_empty()
                } else {
                    false
                }
            },
        ),
    )
    .parse(input)
}

fn tag_head_keypair(input: &str) -> ParseResult<'_, (String, Option<String>)> {
    context(
        "TagHeadKeyPair",
        alt((
            map(
                separated_pair(
                    preceded(multispace0, tag_key),
                    preceded(multispace0, char('=')),
                    preceded(multispace0, tag_value),
                ),
                |(k, v)| (k, Some(v)),
            ),
            map(preceded(multispace0, tag_key), |s| (s, None)),
        )),
    )
    .parse(input)
}

fn tag_key(input: &str) -> ParseResult<'_, String> {
    context("TagKey", string_without_space).parse(input)
}

fn tag_value(input: &str) -> ParseResult<'_, String> {
    context("TagValue", alt((string_without_space, string_quoted))).parse(input)
}

fn tag_head(input: &str) -> ParseResult<'_, (String, Option<String>)> {
    context(
        "TagHead",
        preceded(
            (char('['), not(char('/'))),
            cut(terminated(
                tag_head_keypair,
                preceded(multispace0, char(']')),
            )),
        ),
    )
    .parse(input)
}

fn tag_end(input: &str) -> ParseResult<'_, String> {
    context(
        "TagEnd",
        preceded(
            tag("[/"),
            cut(terminated(tag_value, preceded(multispace0, char(']')))),
        ),
    )
    .parse(input)
}

fn closed_tag(input: &str) -> ParseResult<'_, Element> {
    context(
        "Tag",
        map(
            verify(
                (tag_head, elements, tag_end),
                |&((ref head_key, _), _, ref end_key)| head_key == end_key,
            ),
            |((key, value), inner, _)| {
                Element::Block(Block {
                    inner,
                    tag: key.to_string(),
                    value: value.and_then(|s| Some(s.to_string())),
                })
            },
        ),
    )
    .parse(input)
}

fn element(input: &str) -> ParseResult<'_, Element> {
    context("Element", alt((plain_text, closed_tag))).parse(input)
}

fn elements(input: &str) -> ParseResult<'_, Vec<Element>> {
    context("Element[]", many0(element)).parse(input)
}

pub fn parse(input: &str) -> Result<Vec<Element>, String> {
    match context("Root", many_till(element, eof)).parse(input) {
        Ok((_, (r, _))) => Ok(r),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(convert_error(input, e)),
        Err(nom::Err::Incomplete(_)) => {
            unreachable!("it should not reach this branch, may be a bug.");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::*;

    #[test]
    #[ignore]
    fn parse_text() {
        // let input = "泽材[fillColor=0xff6600]灭[bold]逐[/bold][/fillColor]莫笔[strokeEnable=true]亡[/strokeEnable]鲜，[strokeEnable=true][strokeColor=black][fillColor=red][fontSize=64]如何[/fontSize][fillColor=orange][italic]气[/italic][fillColor=yellow][bold]死[/bold][fillColor=green]你的[fillColor=0xff6600]设[fillColor=blue]计师[fillColor=magenta][fontSize=28]朋[/fontSize]友[/fillColor][/fillColor][/fillColor][/fillColor][/fillColor][/fillColor][/fillColor][/strokeColor][/strokeEnable]";

        let input = r#"ssf[xx="123"]aaa[/xx]"#;

        match parse(input) {
            Ok(r) => println!("{:#?}", r),
            Err(error) => {
                println!("{}", error);
            }
        }
    }

    #[test]
    fn plain_text() {
        assert_eq!(
            parse(" some text ").unwrap(),
            vec![Element::Text(" some text ".to_string())]
        );
    }

    #[test]
    fn plain_text_multiple_line() {
        assert_eq!(
            parse(" some \n  text ").unwrap(),
            vec![Element::Text(" some \n  text ".to_string())]
        );
    }

    #[test]
    fn plain_text_with_backslash() {
        assert_eq!(
            parse(r" some \n [[text").unwrap(),
            vec![Element::Text(r" some \n [text".to_string())]
        );
    }

    #[test]
    fn double_bracket_escape() {
        assert_eq!(
            parse("[[bracket]]").unwrap(),
            vec![Element::Text("[bracket]".to_string())]
        );
    }

    #[test]
    fn double_bracket_in_text() {
        assert_eq!(
            parse("text [[left]] more [[right]]").unwrap(),
            vec![Element::Text("text [left] more [right]".to_string())]
        );
    }

    #[test]
    fn quad_bracket_escape() {
        assert_eq!(
            parse("[[[[double]]]]").unwrap(),
            vec![Element::Text("[[double]]".to_string())]
        );
    }

    #[test]
    fn mixed_escape_and_tag() {
        assert_eq!(
            parse("[[tag]] [real]content[/real]").unwrap(),
            vec![
                Element::Text("[tag] ".to_string()),
                Element::Block(Block {
                    inner: vec![Element::Text("content".to_string())],
                    tag: "real".to_string(),
                    value: None
                })
            ]
        );
    }

    #[test]
    fn mixed_escape_and_tag2() {
        assert_eq!(
            parse(" [real]content[/real]").unwrap(),
            vec![
                Element::Text(" ".to_string()),
                Element::Block(Block {
                    inner: vec![Element::Text("content".to_string())],
                    tag: "real".to_string(),
                    value: None
                })
            ]
        );
    }

    #[test]
    fn complex_escape_tags() {
        assert_eq!(
            parse("Show [[bold]]text[[/bold]] as literal, but [bold]this[/bold] is real").unwrap(),
            vec![
                Element::Text("Show [bold]text[/bold] as literal, but ".to_string()),
                Element::Block(Block {
                    inner: vec![Element::Text("this".to_string())],
                    tag: "bold".to_string(),
                    value: None
                }),
                Element::Text(" is real".to_string())
            ]
        );
    }

    #[test]
    fn single_block_without_value() {
        assert_eq!(
            parse(r"[foo]text[/foo]").unwrap(),
            vec![Element::Block(Block {
                inner: vec![Element::Text("text".to_string())],
                tag: "foo".to_string(),
                value: None
            })]
        );
    }

    #[test]
    fn single_block_with_value() {
        assert_eq!(
            parse(r"[foo=bar]text[/foo]").unwrap(),
            vec![Element::Block(Block {
                inner: vec![Element::Text("text".to_string())],
                tag: "foo".to_string(),
                value: Some("bar".to_string())
            })]
        );
    }

    #[test]
    fn single_block_with_value_quoted() {
        assert_eq!(
            parse(r#"[foo="bar "]text[/foo]"#).unwrap(),
            vec![Element::Block(Block {
                inner: vec![Element::Text("text".to_string())],
                tag: "foo".to_string(),
                value: Some("bar ".to_string())
            })]
        );
    }

    #[test]
    fn single_block_multiline() {
        assert_eq!(
            parse("[foo=bar]\ntext\n  \n[/foo]").unwrap(),
            vec![Element::Block(Block {
                inner: vec![Element::Text("\ntext\n  \n".to_string())],
                tag: "foo".to_string(),
                value: Some("bar".to_string())
            })]
        );
    }

    #[test]
    fn mixed_text_and_block() {
        assert_eq!(
            parse(r" some text [foo=bar]text[/foo]").unwrap(),
            vec![
                Element::Text(" some text ".to_string()),
                Element::Block(Block {
                    inner: vec![Element::Text("text".to_string())],
                    tag: "foo".to_string(),
                    value: Some("bar".to_string())
                })
            ]
        );
    }

    #[test]
    fn nested_blocks() {
        assert_eq!(
            parse(r"[foo=bar][xx=123][/xx][/foo]").unwrap(),
            vec![Element::Block(Block {
                inner: vec![Element::Block(Block {
                    inner: vec![],
                    tag: "xx".to_string(),
                    value: Some("123".to_string())
                })],
                tag: "foo".to_string(),
                value: Some("bar".to_string())
            })]
        );
    }

    #[test]
    fn complex_elements() {
        // Backslash and 'n' are now treated as literal characters, not escape sequence
        assert_eq!(
            parse(r"a\n[foo=bar]q[xx=123][/xx]x[/foo][yy][/yy]").unwrap(),
            vec![
                Element::Text(r"a\n".to_string()),
                Element::Block(Block {
                    inner: vec![
                        Element::Text("q".to_string()),
                        Element::Block(Block {
                            inner: vec![],
                            tag: "xx".to_string(),
                            value: Some("123".to_string())
                        }),
                        Element::Text("x".to_string())
                    ],
                    tag: "foo".to_string(),
                    value: Some("bar".to_string())
                }),
                Element::Block(Block {
                    inner: vec![],
                    tag: "yy".to_string(),
                    value: None
                })
            ]
        );
    }

    #[test]
    fn tagpair_with_spaces() {
        assert_eq!(
            parse(r#"[ foo = "bar " ]text[/ foo  ]"#).unwrap(),
            vec![Element::Block(Block {
                inner: vec![Element::Text("text".to_string())],
                tag: "foo".to_string(),
                value: Some("bar ".to_string())
            })]
        );
    }

    #[test]
    fn empty() {
        assert_eq!(parse("").unwrap(), vec![]);
    }
}
