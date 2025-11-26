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

use crate::parser::{Segment, SegmentId};

// Type alias for input with location tracking
pub type Span<'a> = LocatedSpan<&'a str, Option<SegmentId>>;

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
        segment_id: Option<SegmentId>,
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
    let segment_id = input.extra.clone();

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
            segment_id,
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
            value(("".to_string(), None), multispace0),
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
                alt((tag_key::<OPEN, CLOSE>, value("".to_string(), multispace0))),
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
    input: &Segment<'_>,
) -> Result<Vec<Element>, String> {
    let span = Span::new_extra(input.content, input.id.clone());
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
            Err(convert_error(input.content, converted_error))
        }
        Err(nom::Err::Incomplete(_)) => {
            unreachable!("it should not reach this branch, may be a bug.");
        }
    }
}

/// Parse input with default square bracket tags `[]`.
///
/// This is equivalent to calling `parse_with::<'[', ']'>(input)`.
pub fn parse(input: &Segment<'_>) -> Result<Vec<Element>, String> {
    parse_with::<'[', ']'>(input)
}
