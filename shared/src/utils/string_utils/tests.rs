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
