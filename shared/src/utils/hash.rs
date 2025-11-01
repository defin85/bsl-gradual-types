//! Утилиты для хеширования содержимого файлов
//!
//! Использует xxHash64 для быстрого, детерминированного хеширования.

/// Быстрое хеширование содержимого для кеш-ключей
///
/// # Характеристики
/// - xxHash64: 2-3x быстрее DefaultHasher
/// - Детерминированный: seed = 0
/// - Не криптографический (подходит для кеша)
///
/// # Примеры
/// ```
/// use bsl_shared::utils::hash::hash_content;
///
/// let content = "Функция Тест() КонецФункции";
/// let hash = hash_content(content);
/// println!("Hash: {}", hash);
/// ```
pub fn hash_content(content: &str) -> u64 {
    use xxhash_rust::xxh64::xxh64;
    xxh64(content.as_bytes(), 0) // seed = 0 для детерминированности
}

#[cfg(test)]
mod tests {
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
}
