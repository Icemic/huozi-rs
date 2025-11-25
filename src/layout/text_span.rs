use super::TextRun;

/// A sequence of text runs which form a complete paragraph or a block of text.
///
/// For example, a text span can be created as follows:
///
/// ```rust
/// use huozi::layout::TextRun;
/// let span = TextSpan {
///   runs: vec![
///     TextRun {
///       text: "Hello, ".to_string(),
///      ..Default::default()
///     },
///     TextRun {
///       text: "world!".to_string(),
///      ..Default::default()
///     },
///   ],
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct TextSpan {
    pub runs: Vec<TextRun>,
    pub span_id: Option<SpanId>,
}

/// An identifier for a text span, which can be either a String or u32.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanId {
    Tag(String),
    Lite(u32),
}
