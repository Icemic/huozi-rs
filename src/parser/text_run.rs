use crate::parser::source_range::SourceRange;
use crate::parser::text_style::TextStyle;

/// An item in a sequence of text which is divided to groups by its' same style.
/// For example, a plain text shall be presented as
///
/// ```rust
///     use huozi::parser::TextRun;
///     let run = TextRun {
///         text: "This is plain text.".to_string(),
///         ..Default::default()
///     };
/// ```
///
/// or a rich text can be presented as a vector of TextRun, such as
///
/// ```rust
///     use huozi::parser::TextRun;
///     // for text "This is <color=#f00>rich</color> text."
///     let runs = vec![
///         TextRun {
///             text: "This is ".to_string(),
///             ..Default::default()
///         },
///         TextRun {
///             text: "rich".to_string(),
///             // the style should be set to red color
///             ..Default::default()
///         },
///         TextRun {
///             text: " text.".to_string(),
///             ..Default::default()
///         },
///     ];
/// ```
///
#[derive(Debug, Clone, Default)]
pub struct TextRun {
    pub text: String,
    pub style: TextStyle,
    pub source_range: SourceRange,
}
