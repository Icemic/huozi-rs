use crate::Glyph;

use super::TextStyle;

/// An item in a sequence of text which is divided to groups by its' same style.
/// For example, a plain text shall be presented as
///
/// ```rust
///     use huozi::layout::TextSection;
///     let section = TextSection {
///         text: "This is plain text.".to_string(),
///         ..Default::default()
///     };
/// ```
///
/// or a rich text can be presented as a vector of TextSection, such as
///
/// ```rust
///     use huozi::layout::TextSection;
///     // for text "This is <color=#f00>rich</color> text."
///     let sections = vec![
///         TextSection {
///             text: "This is ".to_string(),
///             ..Default::default()
///         },
///         TextSection {
///             text: "rich".to_string(),
///             // the style should be set to red color
///             ..Default::default()
///         },
///         TextSection {
///             text: " text.".to_string(),
///             ..Default::default()
///         },
///     ];
/// ```
///
#[derive(Debug, Clone, Default)]
pub struct TextSection {
    pub text: String,
    pub style: TextStyle,
}
