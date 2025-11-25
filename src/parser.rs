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
use nom_locate::LocatedSpan;
use std::sync::OnceLock;

// Type alias for input with location tracking
pub type Span<'a> = LocatedSpan<&'a str>;

// Global caches for tag symbols
// Note: These are shared across all generic parameter combinations.
// Convention: Use only ONE symbol combination throughout the program's lifetime.
static DOUBLE_OPEN: OnceLock<String> = OnceLock::new();
static DOUBLE_CLOSE: OnceLock<String> = OnceLock::new();
static END_PREFIX: OnceLock<String> = OnceLock::new();
static EXCLUDED_CHARS: OnceLock<String> = OnceLock::new();

/// Get the double open tag string (e.g., "[[" for '[')
/// Initialized on first call and cached for subsequent calls.
fn get_double_open<const OPEN: char>() -> &'static str {
    DOUBLE_OPEN.get_or_init(|| format!("{}{}", OPEN, OPEN))
}

/// Get the double close tag string (e.g., "]]" for ']')
/// Initialized on first call and cached for subsequent calls.
fn get_double_close<const CLOSE: char>() -> &'static str {
    DOUBLE_CLOSE.get_or_init(|| format!("{}{}", CLOSE, CLOSE))
}

/// Get the end tag prefix string (e.g., "[/" for '[')
/// Initialized on first call and cached for subsequent calls.
fn get_end_prefix<const OPEN: char>() -> &'static str {
    END_PREFIX.get_or_init(|| format!("{}/", OPEN))
}

/// Get the excluded characters for string parsing
/// Initialized on first call and cached for subsequent calls.
fn get_excluded_chars<const OPEN: char, const CLOSE: char>() -> &'static str {
    EXCLUDED_CHARS.get_or_init(|| format!("\"\'{}{}{}= \t\n\r", OPEN, CLOSE, '/'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Element {
    Text {
        start: usize,
        end: usize,
        content: String,
    },
    Block {
        start: usize,
        end: usize,
        inner: Vec<Element>,
        tag: String,
        value: Option<String>,
    },
}

pub type ParseResult<'a, T, E = VerboseError<Span<'a>>> = IResult<Span<'a>, T, E>;

/// Parse plain text with support for [[ and ]] escape sequences
/// [[ -> [
/// ]] -> ]
/// Single [ or ] will stop the parser (not consume them)
fn plain_text_content<const OPEN: char, const CLOSE: char>(
    input: Span<'_>,
) -> ParseResult<'_, String> {
    use nom::bytes::complete::take_while1;

    let double_open = get_double_open::<OPEN>();
    let double_close = get_double_close::<CLOSE>();

    context(
        "PlainTextContent",
        fold_many0(
            alt((
                // [[ -> [
                value(OPEN.to_string(), tag(double_open)),
                // ]] -> ]
                value(CLOSE.to_string(), tag(double_close)),
                // Regular text (not starting with [ or ])
                map(take_while1(|c| c != OPEN && c != CLOSE), |s: Span| {
                    s.fragment().to_string()
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

fn string_quoted_single(input: Span<'_>) -> ParseResult<'_, String> {
    context(
        "String Quoted Single",
        preceded(
            char('\''),
            cut(terminated(
                map(is_not("\'"), |s: Span| s.fragment().to_string()),
                char('\''),
            )),
        ),
    )
    .parse(input)
}

fn string_quoted_double(input: Span<'_>) -> ParseResult<'_, String> {
    context(
        "String Quoted",
        preceded(
            char('\"'),
            cut(terminated(
                map(is_not("\""), |s: Span| s.fragment().to_string()),
                char('\"'),
            )),
        ),
    )
    .parse(input)
}

fn string_quoted(input: Span<'_>) -> ParseResult<'_, String> {
    context(
        "String Quoted",
        alt((string_quoted_double, string_quoted_single)),
    )
    .parse(input)
}

fn string_without_space<const OPEN: char, const CLOSE: char>(
    input: Span<'_>,
) -> ParseResult<'_, String> {
    let chars = get_excluded_chars::<OPEN, CLOSE>();
    context(
        "String without Space",
        map(preceded(multispace0, is_not(chars)), |s: Span| {
            s.fragment().to_string()
        }),
    )
    .parse(input)
}

fn plain_text<const OPEN: char, const CLOSE: char>(input: Span<'_>) -> ParseResult<'_, Element> {
    let start_offset = input.location_offset();

    let (remaining, content) = context(
        "PlainText",
        verify(plain_text_content::<OPEN, CLOSE>, |s: &String| {
            !s.is_empty()
        }),
    )
    .parse(input)?;

    let end_offset = remaining.location_offset();

    Ok((
        remaining,
        Element::Text {
            start: start_offset,
            end: end_offset,
            content,
        },
    ))
}

fn tag_head_keypair<const OPEN: char, const CLOSE: char>(
    input: Span<'_>,
) -> ParseResult<'_, (String, Option<String>)> {
    context(
        "TagHeadKeyPair",
        alt((
            map(
                separated_pair(
                    preceded(multispace0, tag_key::<OPEN, CLOSE>),
                    preceded(multispace0, char('=')),
                    preceded(multispace0, tag_value::<OPEN, CLOSE>),
                ),
                |(k, v)| (k, Some(v)),
            ),
            map(preceded(multispace0, tag_key::<OPEN, CLOSE>), |s| (s, None)),
        )),
    )
    .parse(input)
}

fn tag_key<const OPEN: char, const CLOSE: char>(input: Span<'_>) -> ParseResult<'_, String> {
    context("TagKey", string_without_space::<OPEN, CLOSE>).parse(input)
}

fn tag_value<const OPEN: char, const CLOSE: char>(input: Span<'_>) -> ParseResult<'_, String> {
    context(
        "TagValue",
        alt((string_without_space::<OPEN, CLOSE>, string_quoted)),
    )
    .parse(input)
}

fn tag_head<const OPEN: char, const CLOSE: char>(
    input: Span<'_>,
) -> ParseResult<'_, (String, Option<String>)> {
    context(
        "TagHead",
        preceded(
            (char(OPEN), not(char('/'))),
            cut(terminated(
                tag_head_keypair::<OPEN, CLOSE>,
                preceded(multispace0, char(CLOSE)),
            )),
        ),
    )
    .parse(input)
}

fn tag_end<const OPEN: char, const CLOSE: char>(input: Span<'_>) -> ParseResult<'_, String> {
    let end_prefix = get_end_prefix::<OPEN>();
    context(
        "TagEnd",
        preceded(
            tag(end_prefix),
            cut(terminated(
                tag_value::<OPEN, CLOSE>,
                preceded(multispace0, char(CLOSE)),
            )),
        ),
    )
    .parse(input)
}

fn closed_tag<const OPEN: char, const CLOSE: char>(input: Span<'_>) -> ParseResult<'_, Element> {
    let start_offset = input.location_offset();

    let (remaining, ((key, value), inner, _)) = context(
        "Tag",
        verify(
            (
                tag_head::<OPEN, CLOSE>,
                elements::<OPEN, CLOSE>,
                tag_end::<OPEN, CLOSE>,
            ),
            |&((ref head_key, _), _, ref end_key)| head_key == end_key,
        ),
    )
    .parse(input)?;

    let end_offset = remaining.location_offset();

    Ok((
        remaining,
        Element::Block {
            start: start_offset,
            end: end_offset,
            inner,
            tag: key.to_string(),
            value: value.map(|s| s.to_string()),
        },
    ))
}

fn element<const OPEN: char, const CLOSE: char>(input: Span<'_>) -> ParseResult<'_, Element> {
    context(
        "Element",
        alt((plain_text::<OPEN, CLOSE>, closed_tag::<OPEN, CLOSE>)),
    )
    .parse(input)
}

fn elements<const OPEN: char, const CLOSE: char>(input: Span<'_>) -> ParseResult<'_, Vec<Element>> {
    context("Element[]", many0(element::<OPEN, CLOSE>)).parse(input)
}

/// Parse input with custom tag symbols.
///
/// # Type Parameters
/// * `OPEN` - The opening tag character (e.g., '[', '<', '{')
/// * `CLOSE` - The closing tag character (e.g., ']', '>', '}')
///
/// # Convention
/// Only one symbol combination should be used throughout the program's lifetime.
/// Mixing different symbol combinations in the same program run may produce incorrect results.
///
/// # Examples
/// ```ignore
/// // Use square brackets
/// let result = parse_with::<'[', ']'>("text [bold]content[/bold]");
///
/// // Use angle brackets
/// let result = parse_with::<'<', '>'>("text <bold>content</bold>");
///
/// // Use curly braces
/// let result = parse_with::<'{', '}'>("text {bold}content{/bold}");
/// ```
pub fn parse_with<const OPEN: char, const CLOSE: char>(
    input: &str,
) -> Result<Vec<Element>, String> {
    let span = Span::new(input);
    match context("Root", many_till(element::<OPEN, CLOSE>, eof)).parse(span) {
        Ok((_, (r, _))) => Ok(r),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            // Convert Span-based error to str-based error for convert_error
            let converted_error: VerboseError<&str> = VerboseError {
                errors: e
                    .errors
                    .into_iter()
                    .map(|(span, kind)| (*span.fragment(), kind))
                    .collect(),
            };
            Err(convert_error(input, converted_error))
        }
        Err(nom::Err::Incomplete(_)) => {
            unreachable!("it should not reach this branch, may be a bug.");
        }
    }
}

/// Parse input with default square bracket tags `[]`.
///
/// This is equivalent to calling `parse_with::<'[', ']'>(input)`.
pub fn parse(input: &str) -> Result<Vec<Element>, String> {
    parse_with::<'[', ']'>(input)
}

#[cfg(test)]
mod tests {
    use crate::parser::*;

    #[test]
    fn plain_text() {
        assert_eq!(
            parse(" some text ").unwrap(),
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
            parse(" some \n  text ").unwrap(),
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
            parse(r" some \n [[text").unwrap(),
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
            parse("[[bracket]]").unwrap(),
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
            parse("text [[left]] more [[right]]").unwrap(),
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
            parse("[[[[double]]]]").unwrap(),
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
            parse("[[tag]] [real]content[/real]").unwrap(),
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
            parse(" [real]content[/real]").unwrap(),
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
            parse("Show [[bold]]text[[/bold]] as literal, but [bold]this[/bold] is real").unwrap(),
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
            parse(r"[foo]text[/foo]").unwrap(),
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
    fn single_block_with_value() {
        assert_eq!(
            parse(r"[foo=bar]text[/foo]").unwrap(),
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
            parse(r#"[foo="bar "]text[/foo]"#).unwrap(),
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
            parse(r#"[foo='bar ']text[/foo]"#).unwrap(),
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
            parse("[foo=bar]\ntext\n  \n[/foo]").unwrap(),
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
            parse(r" some text [foo=bar]text[/foo]").unwrap(),
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
            parse(r"[foo=bar][xx=123][/xx][/foo]").unwrap(),
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
            parse(r"a\n[foo=bar]q[xx=123][/xx]x[/foo][yy][/yy]").unwrap(),
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
            parse(r#"[ foo = "bar " ]text[/ foo  ]"#).unwrap(),
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
        assert_eq!(parse("").unwrap(), vec![]);
    }
}
