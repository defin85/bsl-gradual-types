use super::*;

#[test]
fn test_tree_cache_basic() {
    let cache = TreeCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_hash_content() {
    let content1 = "Функция Тест() КонецФункции";
    let content2 = "Функция Тест() КонецФункции";
    let content3 = "Функция Другой() КонецФункции";

    let hash1 = hash_content(content1);
    let hash2 = hash_content(content2);
    let hash3 = hash_content(content3);

    assert_eq!(
        hash1, hash2,
        "Одинаковый контент должен иметь одинаковый хеш"
    );
    assert_ne!(hash1, hash3, "Разный контент должен иметь разный хеш");
}
