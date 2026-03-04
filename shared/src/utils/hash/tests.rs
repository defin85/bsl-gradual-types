use super::*;

#[test]
fn test_hash_content_deterministic() {
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

#[test]
fn test_hash_content_empty() {
    let hash = hash_content("");
    assert_ne!(hash, 0, "Пустая строка должна иметь ненулевой хеш");
}
