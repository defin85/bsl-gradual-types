//! String utilities for fuzzy matching
//!
//! This module provides string comparison utilities including
//! Levenshtein distance calculation for suggesting similar names.

/// Computes the Levenshtein distance (edit distance) between two strings.
///
/// The comparison is case-insensitive to handle both Russian and English
/// identifiers uniformly.
///
/// # Algorithm
///
/// Uses the Wagner-Fischer algorithm with O(min(m, n)) space optimization.
///
/// # Parameters
///
/// * `a` - First string to compare
/// * `b` - Second string to compare
///
/// # Returns
///
/// The minimum number of single-character edits (insertions, deletions,
/// substitutions) needed to transform `a` into `b`.
///
/// # Examples
///
/// ```
/// use bsl_shared::utils::string_utils::levenshtein_distance;
///
/// // Identical strings
/// assert_eq!(levenshtein_distance("Контрагенты", "Контрагенты"), 0);
///
/// // One character difference
/// assert_eq!(levenshtein_distance("abc", "abd"), 1);
///
/// // Case insensitive
/// assert_eq!(levenshtein_distance("Test", "test"), 0);
/// ```
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    // Case-insensitive comparison (works for both Cyrillic and Latin)
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let a_chars: Vec<char> = a_lower.chars().collect();
    let b_chars: Vec<char> = b_lower.chars().collect();

    let m = a_chars.len();
    let n = b_chars.len();

    // Optimize for empty strings
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Optimize: use shorter string as column to reduce memory
    let (a_chars, b_chars, m, n) = if m <= n {
        (a_chars, b_chars, m, n)
    } else {
        (b_chars, a_chars, n, m)
    };

    // Wagner-Fischer with O(min(m,n)) space
    // We only need two rows: previous and current
    let mut prev_row: Vec<usize> = (0..=m).collect();
    let mut curr_row: Vec<usize> = vec![0; m + 1];

    for j in 1..=n {
        curr_row[0] = j;

        for i in 1..=m {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };

            curr_row[i] = (prev_row[i] + 1) // deletion
                .min(curr_row[i - 1] + 1) // insertion
                .min(prev_row[i - 1] + cost); // substitution
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[m]
}

/// Computes normalized similarity between two strings.
///
/// Returns a value between 0.0 and 1.0 where:
/// - 1.0 means the strings are identical (case-insensitive)
/// - 0.0 means the strings are completely different
///
/// # Formula
///
/// `similarity = 1.0 - (distance / max(len(a), len(b)))`
///
/// # Examples
///
/// ```
/// use bsl_shared::utils::string_utils::similarity;
///
/// // Identical strings
/// assert!((similarity("test", "test") - 1.0).abs() < 0.001);
///
/// // Completely different
/// assert!((similarity("abc", "xyz") - 0.0).abs() < 0.001);
///
/// // One character difference in 4-char string = 0.75 similarity
/// assert!((similarity("test", "text") - 0.75).abs() < 0.001);
/// ```
pub fn similarity(a: &str, b: &str) -> f64 {
    let a_len = a.chars().count();
    let b_len = b.chars().count();

    if a_len == 0 && b_len == 0 {
        return 1.0;
    }

    let max_len = a_len.max(b_len);
    let distance = levenshtein_distance(a, b);

    1.0 - (distance as f64 / max_len as f64)
}

#[cfg(test)]
#[path = "string_utils/tests.rs"]
mod tests;
