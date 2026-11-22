//! Word-level diff between the original and the rewrite, for the preview highlights.
//!
//! A longest common subsequence over words: the words outside it are removed (original) or
//! added (rewrite). Plain O(n·m) is enough, because the input is at most 2,000 characters.

#![forbid(unsafe_code)]

use std::ops::Range;

/// Above this many table cells (about 16 MB), the preview shows no highlights instead.
const MAX_CELLS: usize = 4_000_000;

/// Byte ranges of the removed words in `old` and the added words in `new`. Neighbor words
/// that both changed become one range, so the highlight also covers the space between them.
pub fn changed_words(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let a = words(old);
    let b = words(new);
    if a.len().saturating_mul(b.len()) > MAX_CELLS {
        return (Vec::new(), Vec::new());
    }
    let same = |i: usize, j: usize| old[a[i].clone()] == new[b[j].clone()];
    // lcs[i][j]: the length of the longest common subsequence of a[i..] and b[j..].
    let width = b.len() + 1;
    let mut lcs = vec![0u32; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i * width + j] = if same(i, j) {
                lcs[(i + 1) * width + j + 1] + 1
            } else {
                lcs[(i + 1) * width + j].max(lcs[i * width + j + 1])
            };
        }
    }
    let (mut kept_a, mut kept_b) = (vec![false; a.len()], vec![false; b.len()]);
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if same(i, j) {
            kept_a[i] = true;
            kept_b[j] = true;
            i += 1;
            j += 1;
        } else if lcs[(i + 1) * width + j] >= lcs[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    (merged(&a, &kept_a), merged(&b, &kept_b))
}

/// Byte ranges of the runs of non-white-space characters.
fn words(text: &str) -> Vec<Range<usize>> {
    let mut words = Vec::new();
    let mut start = None;
    for (at, c) in text.char_indices() {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(at),
            (true, Some(from)) => {
                words.push(from..at);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        words.push(from..text.len());
    }
    words
}

fn merged(words: &[Range<usize>], kept: &[bool]) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut previous_changed = false;
    for (word, kept) in words.iter().zip(kept) {
        if *kept {
            previous_changed = false;
            continue;
        }
        match ranges.last_mut() {
            Some(last) if previous_changed => last.end = word.end,
            _ => ranges.push(word.clone()),
        }
        previous_changed = true;
    }
    ranges
}

#[cfg(test)]
// The expected values are lists of ranges, not a range to expand.
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::changed_words;

    #[test]
    fn a_replaced_word_is_marked_on_both_sides() {
        assert_eq!(changed_words("the meeting are moved", "the meeting has moved"), (vec![12..15], vec![12..15]));
    }

    #[test]
    fn neighbor_changes_become_one_range_with_the_space_between() {
        // "x y" is one range (2..5); unchanged "c" splits it from nothing else.
        assert_eq!(changed_words("a b c", "a x y c"), (vec![2..3], vec![2..5]));
    }

    #[test]
    fn ranges_are_byte_offsets_with_multibyte_text() {
        // "café is " is 9 bytes (é is 2), so "très" (5 bytes) is 9..14.
        assert_eq!(changed_words("café is très bon", "café is bon"), (vec![9..14], vec![]));
    }

    #[test]
    fn a_moved_word_keeps_the_longer_common_run() {
        // Matching the first "x" greedily would mark "a b" instead.
        assert_eq!(changed_words("x a b", "a b x"), (vec![0..1], vec![4..5]));
    }

    #[test]
    fn identical_text_and_white_space_changes_mark_nothing() {
        assert_eq!(changed_words("same text", "same   text\n"), (vec![], vec![]));
    }
}
