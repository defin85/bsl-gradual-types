//! Кеш tree-sitter деревьев для инкрементального парсинга
//!
//! Хранит деревья в памяти для быстрого обновления при изменениях файлов

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tree_sitter::Tree;

/// Кеш tree-sitter деревьев для инкрементального парсинга
pub struct TreeCache {
    /// Кеш деревьев: file_path → (content_hash, Tree)
    cache: Arc<RwLock<HashMap<PathBuf, CachedTree>>>,
}

/// Закешированное дерево с метаданными
#[derive(Clone)]
struct CachedTree {
    /// Хеш содержимого файла (для проверки актуальности)
    content_hash: u64,
    /// Tree-sitter дерево
    tree: Arc<Tree>,
    /// Исходный код (нужен для tree.edit)
    source: String,
}

impl TreeCache {
    /// Создать новый кеш
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Получить закешированное дерево
    pub fn get(&self, file_path: &PathBuf) -> Option<(Arc<Tree>, String, u64)> {
        let cache = self.cache.read().ok()?;
        let cached = cache.get(file_path)?;
        Some((cached.tree.clone(), cached.source.clone(), cached.content_hash))
    }

    /// Сохранить дерево в кеш
    pub fn set(&self, file_path: PathBuf, tree: Tree, source: String, content_hash: u64) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(
                file_path,
                CachedTree {
                    content_hash,
                    tree: Arc::new(tree),
                    source,
                },
            );
        }
    }

    /// Обновить закешированное дерево после редактирования
    pub fn update(
        &self,
        file_path: &PathBuf,
        new_tree: Tree,
        new_source: String,
        new_hash: u64,
    ) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(
                file_path.clone(),
                CachedTree {
                    content_hash: new_hash,
                    tree: Arc::new(new_tree),
                    source: new_source,
                },
            );
        }
    }

    /// Удалить дерево из кеша (при закрытии файла)
    pub fn remove(&self, file_path: &PathBuf) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(file_path);
        }
    }

    /// Очистить весь кеш
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Получить количество закешированных деревьев
    pub fn len(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    /// Проверить, пуст ли кеш
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for TreeCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Быстрый хеш для содержимого файла
pub fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
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

        assert_eq!(hash1, hash2, "Одинаковый контент должен иметь одинаковый хеш");
        assert_ne!(hash1, hash3, "Разный контент должен иметь разный хеш");
    }
}
