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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_adjustable_cjk_punctuation() {
        for ch in ['（', '」', '、', '，', '。', '：', '；', '？', '！'] {
            assert!(is_adjustable(ch), "{ch} should be adjustable");
        }

        for ch in ['中', 'A', ',', '.', '—', '…'] {
            assert!(!is_adjustable(ch), "{ch} should not be adjustable");
        }
    }

    #[test]
    fn classifies_hangable_cjk_punctuation() {
        for ch in ['、', '，', '。', '：', '；', '？', '！'] {
            assert!(is_hangable(ch), "{ch} should be hangable");
        }

        for ch in ['（', '」', '中', ',', '.'] {
            assert!(!is_hangable(ch), "{ch} should not be hangable");
        }
    }

    #[test]
    fn compresses_only_adjacent_adjustable_punctuation() {
        assert_eq!(compression_between(Some('，'), '。'), 0.5);
        assert_eq!(compression_between(Some('」'), '「'), 0.5);
        assert_eq!(compression_between(Some('中'), '。'), 0.0);
        assert_eq!(compression_between(None, '。'), 0.0);
    }
}
