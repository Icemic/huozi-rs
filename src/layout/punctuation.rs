/// Returns whether a character participates in full-width punctuation
/// adjustment.
pub(super) fn is_adjustable(ch: char) -> bool {
    matches!(
        ch,
        // Opening brackets and quotation marks.
        '（' | '〔'
            | '［'
            | '｛'
            | '〈'
            | '《'
            | '「'
            | '『'
            | '【'
            | '〖'
            | '〘'
            | '〚'
            | '‘'
            | '“'
            // Closing brackets and quotation marks.
            | '）'
            | '〕'
            | '］'
            | '｝'
            | '〉'
            | '》'
            | '」'
            | '』'
            | '】'
            | '〗'
            | '〙'
            | '〛'
            | '’'
            | '”'
            // Full-width pause and stop punctuation.
            | '、'
            | '，'
            | '。'
            | '：'
            | '；'
            | '？'
            | '！'
    )
}

/// Returns the amount, in em, removed between two adjacent punctuation marks.
pub(super) fn compression_between(previous: Option<char>, current: char) -> f64 {
    if previous.is_some_and(is_adjustable) && is_adjustable(current) {
        0.5
    } else {
        0.0
    }
}

/// Returns whether a punctuation mark may hang at the end of a horizontal line.
pub(super) fn is_hangable(ch: char) -> bool {
    matches!(ch, '、' | '，' | '。' | '：' | '；' | '？' | '！')
}
