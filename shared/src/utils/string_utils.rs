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
mod tests {
    use super::*;

    // === Levenshtein distance tests ===

    #[test]
    fn test_identical_strings() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("Контрагенты", "Контрагенты"), 0);
    }

    #[test]
    fn test_empty_strings() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "xyz"), 3);
    }

    #[test]
    fn test_single_operation() {
        // One substitution
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        // One insertion
        assert_eq!(levenshtein_distance("abc", "abcd"), 1);
        // One deletion
        assert_eq!(levenshtein_distance("abcd", "abc"), 1);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(levenshtein_distance("Test", "test"), 0);
        assert_eq!(levenshtein_distance("HELLO", "hello"), 0);
        assert_eq!(levenshtein_distance("ABC", "abc"), 0);
    }

    #[test]
    fn test_cyrillic_strings() {
        // Identical Cyrillic
        assert_eq!(levenshtein_distance("Справочник", "Справочник"), 0);

        // One character difference (Контрагенты vs Контрогенты - 'а' -> 'о')
        assert_eq!(levenshtein_distance("Контрагенты", "Контрогенты"), 1);

        // Case insensitive Cyrillic
        assert_eq!(levenshtein_distance("ДОКУМЕНТ", "документ"), 0);
    }

    #[test]
    fn test_cyrillic_typos() {
        // Common typos in 1C identifiers
        // "Номенклатура" vs "Номенклотура" (а -> о)
        assert_eq!(levenshtein_distance("Номенклатура", "Номенклотура"), 1);

        // "Контрагенты" vs "Контрагены" (missing 'т')
        assert_eq!(levenshtein_distance("Контрагенты", "Контрагены"), 1);
    }

    #[test]
    fn test_multiple_operations() {
        // "kitten" -> "sitting": k->s, e->i, +g = 3 operations
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);

        // "saturday" -> "sunday": a->u, t->, u->n, r->d, d->a, a->y (complex)
        // Actually: sat->sun=2, urday->day=2 = around 3-4
        let dist = levenshtein_distance("saturday", "sunday");
        assert!((3..=4).contains(&dist));
    }

    // === Similarity tests ===

    #[test]
    fn test_similarity_identical() {
        assert!((similarity("test", "test") - 1.0).abs() < 0.001);
        assert!((similarity("Контрагенты", "Контрагенты") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_similarity_empty() {
        assert!((similarity("", "") - 1.0).abs() < 0.001);
        assert!((similarity("abc", "") - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_similarity_one_difference() {
        // "test" vs "text" - 1 difference in 4 chars = 0.75 similarity
        let sim = similarity("test", "text");
        assert!((sim - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_similarity_case_insensitive() {
        assert!((similarity("Test", "TEST") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_similarity_range() {
        // Ensure similarity is always in [0.0, 1.0]
        let sim1 = similarity("abc", "xyz");
        assert!((0.0..=1.0).contains(&sim1));

        let sim2 = similarity("hello", "hallo");
        assert!((0.0..=1.0).contains(&sim2));
    }
}
