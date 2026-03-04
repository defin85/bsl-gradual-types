use super::*;

#[test]
fn default_keywords_include_basics() {
    let items = default_keyword_items();
    let names: std::collections::HashSet<String> =
        items.into_iter().map(|item| item.name).collect();

    assert!(names.contains("Если"));
    assert!(names.contains("КонецЕсли"));
    assert!(names.contains("Процедура"));
    assert!(names.contains("Функция"));
    assert!(names.contains("Перем"));
}

#[test]
fn fallback_to_default_when_empty() {
    let items = keyword_items_from_syntax_or_default(&[]);
    let names: std::collections::HashSet<String> =
        items.into_iter().map(|item| item.name).collect();

    assert!(names.contains("Если"));
    assert!(names.contains("Процедура"));
}
