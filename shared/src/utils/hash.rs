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
#[path = "hash/tests.rs"]
mod tests;
