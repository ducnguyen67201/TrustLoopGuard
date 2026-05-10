//! Edit-distance bypass detection.
//!
//! Catches typo-style attempts to evade literal banned phrases:
//! `refund` → `refunddd`, `refun*d`, `r3fund`. Cheaper than embedding
//! similarity and complements it well — Levenshtein excels at character
//! perturbations, embeddings excel at paraphrase.

use strsim::{levenshtein, normalized_levenshtein};

/// True if any window of `text` of length `pattern.len() ± slack` has
/// normalised Levenshtein similarity >= `threshold` against `pattern`.
///
/// `pattern` is matched case-insensitively. `threshold` is `[0.0, 1.0]`
/// where 1.0 means identical and 0.0 means completely different. A
/// threshold of 0.85 is a reasonable starting point for catching common
/// typos and digit-substitutions while rejecting unrelated words.
pub fn fuzzy_contains(text: &str, pattern: &str, threshold: f32) -> bool {
    let p_lower = pattern.to_ascii_lowercase();
    let t_lower = text.to_ascii_lowercase();
    if p_lower.is_empty() {
        return false;
    }
    let p_len = p_lower.chars().count();
    if p_len == 0 {
        return false;
    }
    let slack = (p_len / 4).max(1);
    let min_w = p_len.saturating_sub(slack);
    let max_w = p_len + slack;

    let chars: Vec<char> = t_lower.chars().collect();
    if chars.len() < min_w {
        return false;
    }

    for w_len in min_w..=max_w.min(chars.len()) {
        for start in 0..=(chars.len() - w_len) {
            let window: String = chars[start..start + w_len].iter().collect();
            if normalized_levenshtein(&window, &p_lower) as f32 >= threshold {
                return true;
            }
        }
    }
    false
}

/// Raw distance helper. Useful when the caller wants the actual edit
/// count (e.g. logging "1 character off" vs "3 characters off").
pub fn distance(a: &str, b: &str) -> usize {
    levenshtein(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_passes() {
        assert!(fuzzy_contains("the refund is processing", "refund", 0.85));
    }

    #[test]
    fn case_insensitive() {
        assert!(fuzzy_contains("REFUND today", "refund", 0.85));
    }

    #[test]
    fn one_char_typo_passes() {
        // refun*d / refundd / refunds — all within edit-distance 1.
        assert!(fuzzy_contains("refundd today", "refund", 0.85));
        assert!(fuzzy_contains("refun*d today", "refund", 0.8));
    }

    #[test]
    fn digit_substitution_passes_with_lower_threshold() {
        // r3fund swaps 'e'->'3', distance 1. At 0.85 threshold for
        // length-6 pattern, 1 edit is allowed.
        assert!(fuzzy_contains("r3fund please", "refund", 0.8));
    }

    #[test]
    fn unrelated_word_does_not_pass() {
        assert!(!fuzzy_contains("the weather is sunny", "refund", 0.85));
    }

    #[test]
    fn empty_inputs_safe() {
        assert!(!fuzzy_contains("", "refund", 0.85));
        assert!(!fuzzy_contains("hello", "", 0.85));
    }

    #[test]
    fn distance_helper_basic() {
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", ""), 0);
    }
}
